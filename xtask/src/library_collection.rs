use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use untwist_rename::library::collection::{
    bloom_file, sha256_hex, FileRef, Index, Package, Shard, INDEX, SCHEMA,
};
use untwist_rename::library::table::{Role, RowData as Row, Table};
use untwist_rename::library::SHAPE_EPOCH;

use crate::library_shards::Blooms;
use crate::library_table::{aggregate, process_build, read_manifest, Observed};

const TOOLS: &str = "tools/library-collection";
const RUNTIMES: &str = "runtimes";
const RUNTIME_KEYS: &str = "runtimes.keys";
const MIN_UNIT_TOKENS: u32 = 12;
const LANES: [&[&str]; 2] = [&[], &["--unsafe"]];
const BABEL_6_HELPERS: [&str; 3] = [
    "sliceIterator",
    "possibleConstructorReturn",
    "interopRequireDefault",
];
const SHARDS: &str = "shards";
const SHARED_LABELS: usize = 3;
const KEY_LIMIT: usize = 300;
const CURATED_DIRS: [&str; 2] = [
    "crates/untwist-libraries-core/data",
    "crates/untwist-libraries-corejs/data",
];

#[derive(clap::Args, Debug)]
pub struct Request {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(clap::Subcommand, Debug)]
pub enum Action {
    Select {
        #[arg(long)]
        work: PathBuf,
        #[arg(long, default_value_t = 200)]
        top: usize,
        #[arg(long, default_value_t = 4)]
        max_versions: usize,
        #[arg(long)]
        wanted: Option<PathBuf>,
        #[arg(long)]
        wrap: Option<String>,
    },
    Build {
        #[arg(long)]
        work: PathBuf,
        #[arg(long)]
        untwist: Option<PathBuf>,
        #[arg(long, default_value_t = 4)]
        jobs: usize,
        #[arg(long)]
        only: Option<String>,
        #[arg(long)]
        wrap: Option<String>,
    },
    Pack {
        #[arg(long)]
        work: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        only: Option<String>,
        #[arg(long)]
        no_curated: bool,
    },
    Hints {
        files: Vec<PathBuf>,
        #[arg(long)]
        collection: Option<PathBuf>,
        #[arg(long)]
        log: Option<PathBuf>,
    },
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    match &request.action {
        Action::Select {
            work,
            top,
            max_versions,
            wanted,
            wrap,
        } => select(
            root,
            work,
            (*top, *max_versions),
            wanted.as_deref(),
            wrap.as_deref(),
        ),
        Action::Build {
            work,
            untwist,
            jobs,
            only,
            wrap,
        } => build(
            root,
            work,
            untwist.as_deref(),
            *jobs,
            only.as_deref(),
            wrap.as_deref(),
        ),
        Action::Pack {
            work,
            out,
            version,
            only,
            no_curated,
        } => {
            let curated = match no_curated {
                true => Vec::new(),
                false => curated_tables(root, only.as_deref())?,
            };
            pack(work, out, version, only.as_deref(), &curated)
        }
        Action::Hints {
            files,
            collection,
            log,
        } => hints(files, collection.as_deref(), log.as_deref()),
    }
}

fn node(root: &Path, wrap: Option<&str>, script: &str) -> Command {
    let tools = root.join(TOOLS);
    let mut command = match wrap {
        Some(wrapper) => {
            let mut parts = wrapper.split_whitespace();
            let mut command = Command::new(parts.next().unwrap_or("node"));
            command.args(parts).arg("node");
            command
        }
        None => Command::new("node"),
    };
    command.current_dir(&tools).arg(tools.join(script));
    command
}

fn checked(mut command: Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("running {what}"))?;
    if !status.success() {
        bail!("{what} failed");
    }
    Ok(())
}

fn select(
    root: &Path,
    work: &Path,
    (top, max_versions): (usize, usize),
    wanted: Option<&Path>,
    wrap: Option<&str>,
) -> Result<()> {
    let mut command = node(root, wrap, "select.mjs");
    command
        .arg("--out")
        .arg(work)
        .arg("--top")
        .arg(top.to_string())
        .arg("--max-versions")
        .arg(max_versions.to_string());
    if let Some(wanted) = wanted {
        command.arg("--wanted").arg(wanted);
    }
    checked(command, "select.mjs")
}

struct Selected {
    name: String,
    versions: Vec<String>,
}

fn selection(work: &Path) -> Result<Vec<Selected>> {
    let path = work.join("selection.json");
    let value: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
    )?;
    Ok(value["packages"]
        .as_array()
        .context("selection.json has no packages")?
        .iter()
        .filter_map(|package| {
            Some(Selected {
                name: package["name"].as_str()?.to_string(),
                versions: package["versions"]
                    .as_array()?
                    .iter()
                    .filter_map(|version| version.as_str().map(str::to_string))
                    .collect(),
            })
        })
        .collect())
}

fn safe_name(name: &str) -> String {
    name.replace('/', "__")
}

fn untwist_binary(root: &Path, given: Option<&Path>) -> Result<PathBuf> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let binary = given.map_or_else(|| target.join("release/untwist"), Path::to_path_buf);
    if !binary.exists() {
        bail!("build first: cargo build --release -p untwist-cli");
    }
    Ok(binary)
}

fn build_package(
    root: &Path,
    work: &Path,
    package: &Selected,
    wrap: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let builds = work.join("builds").join(safe_name(&package.name));
    let mut manifests = Vec::new();
    for version in &package.versions {
        let pinned = format!("{}@{version}", safe_name(&package.name));
        let manifest = builds.join(format!("{pinned}.manifest.json"));
        if manifest.exists() {
            manifests.push(manifest);
            continue;
        }
        let install = work.join("pkgs").join(&pinned);
        let mut command = node(root, wrap, "install.mjs");
        command.arg(&install).arg(&package.name).arg(version);
        if let Err(error) = checked(command, "install.mjs") {
            eprintln!("{pinned}: {error:#}");
            continue;
        }
        let mut command = node(root, wrap, "build.mjs");
        command
            .arg(&install)
            .arg(&package.name)
            .arg(version)
            .arg(&builds);
        if let Err(error) = checked(command, "build.mjs") {
            eprintln!("{pinned}: {error:#}");
            continue;
        }
        manifests.push(manifest);
    }
    Ok(manifests)
}

fn table_package(
    untwist: &Path,
    work: &Path,
    package: &Selected,
    manifests: &[PathBuf],
    jobs: usize,
) -> Result<()> {
    let out = work
        .join("tables")
        .join(format!("{}.table.gz", safe_name(&package.name)));
    if out.exists() {
        return Ok(());
    }
    let builds_dir = work.join("builds").join(safe_name(&package.name));
    let manifests: Vec<_> = manifests
        .iter()
        .map(|path| read_manifest(path))
        .collect::<Result<_>>()?;
    let jobs_list: Vec<(usize, usize)> = manifests
        .iter()
        .enumerate()
        .flat_map(|(index, manifest)| (0..manifest.builds_len()).map(move |build| (index, build)))
        .collect();
    let next = AtomicUsize::new(0);
    let observed: Mutex<Vec<Observed>> = Mutex::new(Vec::new());
    let keys: Mutex<BTreeMap<String, BTreeSet<String>>> = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(manifest, build)) = jobs_list.get(index) else {
                    break;
                };
                let manifest = &manifests[manifest];
                match process_build(untwist, &builds_dir, manifest.build(build), manifest) {
                    Ok((rows, found)) => {
                        let mut known = keys.lock().expect("keys lock");
                        for (key, module) in found {
                            known.entry(key).or_default().insert(module);
                        }
                        drop(known);
                        observed.lock().expect("observed lock").extend(rows);
                    }
                    Err(error) => eprintln!("{}: {error:#}", package.name),
                }
            });
        }
    });
    let Some(first) = manifests.first() else {
        bail!("{}: nothing was built", package.name);
    };
    let rows = aggregate(observed.into_inner().expect("observed lock"), first);
    std::fs::create_dir_all(out.parent().context("a tables folder")?)?;
    crate::library_gz::write(&out, &Table::from_rows(&rows).write(&package.name))?;
    let mut by_label: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for (key, modules) in keys.into_inner().expect("keys lock") {
        for module in &modules {
            if let Some(owner) = crate::library_table::parse_npm_key(module) {
                *by_label
                    .entry(owner.label)
                    .or_default()
                    .entry(key.clone())
                    .or_default() += 1;
            }
        }
    }
    let labels: BTreeMap<String, Vec<String>> = by_label
        .into_iter()
        .map(|(label, counts)| {
            let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
            ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            ranked.truncate(KEY_LIMIT);
            let mut keys: Vec<String> = ranked.into_iter().map(|(key, _)| key).collect();
            keys.sort();
            (label, keys)
        })
        .collect();
    std::fs::write(
        out.with_extension("").with_extension("keys.json"),
        serde_json::to_string_pretty(&json!({ "labels": labels }))? + "\n",
    )?;
    eprintln!("{}: {} rows", package.name, rows.len());
    Ok(())
}

fn build(
    root: &Path,
    work: &Path,
    untwist: Option<&Path>,
    jobs: usize,
    only: Option<&str>,
    wrap: Option<&str>,
) -> Result<()> {
    let untwist = untwist_binary(root, untwist)?;
    runtime_keys(root, work, &untwist, wrap)?;
    for package in selection(work)? {
        if only.is_some_and(|only| !only.split(',').any(|name| name == package.name)) {
            continue;
        }
        let manifests = build_package(root, work, &package, wrap)?;
        if manifests.is_empty() {
            eprintln!("{}: no build", package.name);
            continue;
        }
        if let Err(error) = table_package(&untwist, work, &package, &manifests, jobs) {
            eprintln!("{}: {error:#}", package.name);
        }
    }
    Ok(())
}

fn unit_keys(text: &str, into: &mut BTreeSet<u64>) {
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, text);
    if untwist_core::parse::has_error(&parsed.diagnostics) {
        return;
    }
    let sem = untwist_core::semantic::build_scoping(&parsed.program);
    for record in untwist_rename::library::unit_records(&parsed.program, &sem.scoping) {
        if record.loose.0 >= MIN_UNIT_TOKENS {
            into.insert(untwist_rename::library::table::exact_key(record.exact));
        }
    }
}

fn runtime_keys(root: &Path, work: &Path, untwist: &Path, wrap: Option<&str>) -> Result<()> {
    let keys_file = work.join(RUNTIME_KEYS);
    if keys_file.exists() {
        return Ok(());
    }
    let dir = work.join(RUNTIMES);
    let mut command = node(root, wrap, "runtimes.mjs");
    command.arg(&dir);
    checked(command, "runtimes.mjs")?;
    let mut builds: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    builds.sort();
    let mut keys = BTreeSet::new();
    for build in &builds {
        for flags in LANES {
            let out = dir.join("out");
            let status = Command::new(untwist)
                .args(flags)
                .arg("-f")
                .arg("-o")
                .arg(&out)
                .arg(build)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()?;
            if !status.success() {
                eprintln!("untwist failed on {}", build.display());
            }
            for entry in std::fs::read_dir(&out)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
            {
                if entry.path().extension().is_some_and(|ext| ext == "js") {
                    unit_keys(&std::fs::read_to_string(entry.path())?, &mut keys);
                }
            }
            let _ = std::fs::remove_dir_all(&out);
        }
    }
    let text: String = keys.iter().map(|key| format!("{key:016x}\n")).collect();
    std::fs::write(&keys_file, text)?;
    eprintln!(
        "{} bundler runtime shapes from {} builds",
        keys.len(),
        builds.len()
    );
    Ok(())
}

fn read_runtime_keys(work: &Path) -> Result<BTreeSet<u64>> {
    let Ok(text) = std::fs::read_to_string(work.join(RUNTIME_KEYS)) else {
        return Ok(BTreeSet::new());
    };
    text.lines()
        .map(|line| u64::from_str_radix(line.trim(), 16).context("a runtime key"))
        .collect()
}

fn helper_key(name: &str) -> String {
    name.trim_start_matches('_')
        .trim_end_matches(|ch: char| ch.is_ascii_digit() || ch == '$')
        .to_ascii_lowercase()
}

fn builtin_helper_names() -> BTreeSet<String> {
    let table = untwist_rename::library::table::current();
    table
        .rows
        .iter()
        .filter_map(|row| table.own(row))
        .map(helper_key)
        .filter(|name| name.len() >= 4)
        .chain(BABEL_6_HELPERS.iter().map(|name| helper_key(name)))
        .collect()
}

fn helper_like(row: &Row, helpers: &BTreeSet<String>) -> bool {
    row.own.as_deref().is_some_and(|own| {
        let explicit = BABEL_6_HELPERS.contains(&own);
        (own.starts_with('_') || explicit) && helpers.contains(&helper_key(own))
    })
}

fn table_rows(path: &Path) -> Result<Vec<Row>> {
    let table = Table::parse(&crate::library_gz::read(path)?)
        .with_context(|| format!("{} does not parse", path.display()))?;
    Ok(table
        .rows
        .iter()
        .chain(&table.order)
        .map(|row| table.data(row))
        .collect())
}

fn builtin_exact() -> BTreeSet<u64> {
    let table = untwist_rename::library::table::current();
    table.rows.iter().map(|row| row.exact).collect()
}

pub(crate) fn shared_exact(rows: &[Row], limit: usize) -> BTreeSet<u64> {
    let mut labels: BTreeMap<u64, BTreeSet<&str>> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.role != Role::Order) {
        labels.entry(row.exact).or_default().insert(&row.label);
    }
    labels
        .into_iter()
        .filter(|(_, labels)| labels.len() >= limit)
        .map(|(exact, _)| exact)
        .collect()
}

type MergeKey = (u64, String, String, String, Option<(u32, Option<String>)>);

fn merged_rows(rows: Vec<Row>) -> Vec<Row> {
    let mut merged: BTreeMap<MergeKey, Row> = BTreeMap::new();
    for row in rows {
        let key = (
            row.exact,
            row.label.clone(),
            row.module.clone(),
            row.role.as_str().to_string(),
            (row.role == Role::Order).then(|| (row.specific, row.own.clone())),
        );
        match merged.get_mut(&key) {
            Some(known) => {
                let ranges = untwist_rename::library::union_ranges(
                    &untwist_rename::library::parse_ranges(&known.versions),
                    &untwist_rename::library::parse_ranges(&row.versions),
                );
                known.versions = ranges
                    .iter()
                    .map(|(low, high)| match low == high {
                        true => low.clone(),
                        false => format!("{low}~{high}"),
                    })
                    .collect::<Vec<_>>()
                    .join(",");
            }
            None => {
                merged.insert(key, row);
            }
        }
    }
    merged.into_values().collect()
}

fn read_keys(work: &Path) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(work.join("tables")) else {
        return Ok(keys);
    };
    for entry in entries {
        let path = entry?.path();
        if !path.to_string_lossy().ends_with(".keys.json") {
            continue;
        }
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        for (label, list) in value["labels"].as_object().into_iter().flatten() {
            let set = keys.entry(label.clone()).or_default();
            set.extend(
                list.as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|key| key.as_str().map(str::to_string)),
            );
        }
    }
    Ok(keys)
}

pub(crate) fn write_content(dir: &Path, bytes: &[u8], extension: &str) -> Result<(String, String)> {
    let sha = sha256_hex(bytes);
    let file = format!("{SHARDS}/{sha}.{extension}");
    std::fs::write(dir.join(&file), bytes)?;
    Ok((file, sha))
}

pub(crate) fn versions_of(rows: &[Row]) -> Vec<String> {
    let mut versions: Vec<String> = rows
        .iter()
        .flat_map(|row| untwist_rename::library::parse_ranges(&row.versions))
        .flat_map(|(low, high)| [low, high])
        .collect();
    versions.sort_by(|a, b| {
        untwist_rename::library::version_key(a)
            .cmp(&untwist_rename::library::version_key(b))
            .then_with(|| a.cmp(b))
    });
    versions.dedup();
    versions
}

struct Curated {
    id: String,
    bytes: Vec<u8>,
    rows: Vec<Row>,
    keys: Vec<String>,
}

fn curated_tables(root: &Path, only: Option<&str>) -> Result<Vec<Curated>> {
    let wanted: Option<BTreeSet<&str>> = only.map(|only| only.split(',').collect());
    let mut paths: Vec<PathBuf> = CURATED_DIRS
        .iter()
        .flat_map(|dir| std::fs::read_dir(root.join(dir)).into_iter().flatten())
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".table.gz"))
        .collect();
    paths.sort();
    let mut curated = Vec::new();
    for path in paths {
        let text = crate::library_gz::read(&path)?;
        let id = text
            .lines()
            .next()
            .and_then(|line| line.split('\t').nth(2))
            .with_context(|| format!("{} has no table id", path.display()))?
            .to_string();
        let rows = table_rows(&path)?;
        let named = |wanted: &BTreeSet<&str>| {
            wanted.contains(id.as_str())
                || rows.iter().any(|row| wanted.contains(row.label.as_str()))
        };
        if wanted.as_ref().is_some_and(|wanted| !named(wanted)) {
            continue;
        }
        let keys_path = path.with_file_name(format!("{id}.keys.json"));
        let keys = match std::fs::read_to_string(&keys_path) {
            Ok(text) => serde_json::from_str::<Value>(&text)?["keys"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|key| key.as_str().map(str::to_string))
                .collect(),
            Err(_) => Vec::new(),
        };
        curated.push(Curated {
            id,
            bytes: std::fs::read(&path)?,
            rows,
            keys,
        });
    }
    Ok(curated)
}

pub(crate) fn range_of(versions: &[String]) -> String {
    match (versions.first(), versions.last()) {
        (Some(low), Some(high)) if low != high => format!("{low}~{high}"),
        (Some(low), _) => low.clone(),
        _ => String::new(),
    }
}

fn curated_package(out: &Path, curated: &Curated) -> Result<(Package, Blooms)> {
    let (file, sha256) = write_content(out, &curated.bytes, "utl")?;
    let versions = versions_of(&curated.rows);
    let label = curated
        .rows
        .first()
        .map_or_else(|| curated.id.clone(), |row| row.label.clone());
    let package = Package {
        id: curated.id.clone(),
        label,
        shards: vec![Shard {
            file,
            bytes: curated.bytes.len() as u64,
            sha256,
            versions: range_of(&versions),
            rows: curated.rows.len() as u64,
        }],
        versions,
        keys: curated.keys.clone(),
        curated: true,
    };
    let blooms = crate::library_shards::shard_blooms(&curated.id, &curated.rows);
    Ok((package, blooms))
}

fn pack(
    work: &Path,
    out: &Path,
    version: &str,
    only: Option<&str>,
    curated: &[Curated],
) -> Result<()> {
    let mut rows = Vec::new();
    let mut tables: Vec<PathBuf> = std::fs::read_dir(work.join("tables"))
        .context("no tables: run build first")?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".table.gz"))
        .collect();
    tables.sort();
    for path in &tables {
        rows.extend(table_rows(path)?);
    }
    if let Some(only) = only {
        let wanted: BTreeSet<&str> = only.split(',').collect();
        rows.retain(|row| wanted.contains(row.label.as_str()));
    }
    let curated_labels: BTreeSet<&str> = curated
        .iter()
        .flat_map(|table| &table.rows)
        .map(|row| row.label.as_str())
        .collect();
    let rows = merged_rows(rows);
    let builtin = builtin_exact();
    let runtimes = read_runtime_keys(work)?;
    let helpers = builtin_helper_names();
    let copies = crate::library_owners::vendored_copies(&rows);
    let total = rows.len();
    let rows: Vec<Row> = rows
        .into_iter()
        .filter(|row| !copies.contains(&(row.label.clone(), row.exact)))
        .collect();
    let vendored = total - rows.len();
    let shared = shared_exact(&rows, SHARED_LABELS);
    let mut rows = rows;
    rows.retain(|row| !curated_labels.contains(row.label.as_str()));
    let known = |row: &Row| {
        row.role != Role::Order
            && (builtin.contains(&row.exact)
                || runtimes.contains(&row.exact)
                || helper_like(row, &helpers)
                || crate::library_owners::runtime_named(row))
    };
    let (kept, dropped): (Vec<Row>, Vec<Row>) = rows
        .into_iter()
        .partition(|row| !known(row) && !shared.contains(&row.exact));
    let dropped_builtin = dropped.iter().filter(|row| known(row)).count();
    let kept_rows = kept.len();
    let mut by_label: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for row in kept {
        by_label.entry(row.label.clone()).or_default().push(row);
    }
    std::fs::create_dir_all(out.join(SHARDS))?;
    let keys = read_keys(work)?;
    let mut packages = Vec::new();
    let mut blooms = Vec::new();
    for (label, rows) in &by_label {
        let (shards, package_blooms) = crate::library_shards::package_shards(out, label, rows)?;
        blooms.extend(package_blooms);
        packages.push(Package {
            id: label.clone(),
            label: label.clone(),
            versions: versions_of(rows),
            shards,
            keys: keys
                .get(label)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default(),
            curated: false,
        });
    }
    for table in curated {
        let (package, package_blooms) = curated_package(out, table)?;
        blooms.extend(package_blooms);
        packages.push(package);
    }
    let bloom = bloom_file(&blooms);
    let (file, sha256) = write_content(out, &bloom, "bloom")?;
    let index = Index {
        schema: SCHEMA.to_string(),
        version: version.to_string(),
        shape_epoch: SHAPE_EPOCH,
        untwist: format!(">={}", env!("CARGO_PKG_VERSION")),
        packages,
        filters: Some(FileRef {
            file,
            sha256,
            bytes: bloom.len() as u64,
        }),
    };
    std::fs::write(
        out.join(INDEX),
        serde_json::to_string_pretty(&index)? + "\n",
    )?;
    let generated = index.packages.iter().filter(|package| !package.curated);
    if generated.clone().count() > 0 && generated.clone().all(|package| package.keys.is_empty()) {
        bail!("no package has string keys: the tables were built without literal keys");
    }
    eprintln!(
        "{} packages ({} curated, unfiltered), {} of {total} generated rows kept ({vendored} vendored copies, {dropped_builtin} helper or bundler runtime shapes, {} in {SHARED_LABELS}+ packages)",
        index.packages.len(),
        curated.len(),
        kept_rows,
        dropped.len() - dropped_builtin
    );
    Ok(())
}

fn hint_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let valid = |name: &str| {
        let plain = |part: &str| {
            !part.is_empty()
                && part.len() <= 64
                && !part.starts_with('.')
                && part.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-._".contains(&byte)
                })
        };
        match name.split_once('/') {
            Some((scope, rest)) => scope.starts_with('@') && plain(&scope[1..]) && plain(rest),
            None => plain(name),
        }
    };
    for (at, _) in text.match_indices("node_modules/") {
        let rest = &text[at + "node_modules/".len()..];
        let end = rest
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || "@/-._".contains(ch)))
            .unwrap_or(rest.len());
        let mut parts = rest[..end].split('/');
        let first = parts.next().unwrap_or_default();
        let name = match first.starts_with('@') {
            true => format!("{first}/{}", parts.next().unwrap_or_default()),
            false => first.to_string(),
        };
        if valid(&name) {
            names.insert(name);
        }
    }
    for (at, _) in text.match_indices("/*!") {
        let rest = text[at + 3..].trim_start();
        let word: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || "@/-._".contains(*ch))
            .collect();
        let after = rest[word.len()..].trim_start();
        let versioned = after.starts_with('v') || after.starts_with(|ch: char| ch.is_ascii_digit());
        let name = word.to_ascii_lowercase();
        if versioned && valid(&name) {
            names.insert(name);
        }
    }
    names
}

fn hints(files: &[PathBuf], collection: Option<&Path>, log: Option<&Path>) -> Result<()> {
    let known: BTreeSet<String> = match collection {
        Some(dir) => {
            let index: Index = serde_json::from_str(&std::fs::read_to_string(dir.join(INDEX))?)?;
            index
                .packages
                .into_iter()
                .map(|package| package.id)
                .collect()
        }
        None => BTreeSet::new(),
    };
    let mut found: BTreeSet<String> = BTreeSet::new();
    for file in files {
        let text = std::fs::read(file).with_context(|| format!("read {}", file.display()))?;
        found.extend(hint_names(&String::from_utf8_lossy(&text)));
    }
    let mut logged: BTreeSet<String> = log
        .and_then(|log| std::fs::read_to_string(log).ok())
        .map(|text| {
            text.lines()
                .map(str::to_string)
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default();
    for name in found.difference(&known) {
        println!("{name}");
        logged.insert(name.clone());
    }
    if let Some(log) = log {
        let text: String = logged.iter().map(|name| format!("{name}\n")).collect();
        std::fs::write(log, text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use untwist_rename::library::table::Role;

    fn row(exact: u64, label: &str) -> Row {
        Row {
            loose: 1,
            exact,
            label: label.to_string(),
            versions: "1.0.0".to_string(),
            module: "index".to_string(),
            role: Role::Function,
            own: None,
            inner: Vec::new(),
            outer: Vec::new(),
            specific: 3,
        }
    }

    #[test]
    fn a_shape_in_three_packages_is_shared() {
        let rows = [
            row(1, "a"),
            row(1, "b"),
            row(1, "c"),
            row(2, "a"),
            row(2, "b"),
            row(3, "a"),
        ];
        assert_eq!(shared_exact(&rows, 3), BTreeSet::from([1]));
    }

    #[test]
    fn the_same_row_from_two_tables_merges_its_versions() {
        let mut later = row(1, "a");
        later.versions = "2.0.0".to_string();
        let merged = merged_rows(vec![row(1, "a"), later]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].versions, "1.0.0,2.0.0");
    }

    #[test]
    fn hints_come_from_node_modules_paths_and_license_banners() {
        let text = "/*! jQuery v3.7.1 | (c) */\n\"./node_modules/@vue/shared/dist/x.js\"\n// node_modules/lodash/lodash.js\n/*! not-a-version here */";
        let names: Vec<String> = hint_names(text).into_iter().collect();
        assert_eq!(names, ["@vue/shared", "jquery", "lodash"]);
    }
}
