use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use serde_json::Value;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::shape::statement_shapes;
use untwist_rename::library::shape::UnitKind;
use untwist_rename::library::table::{
    exact_key, loose_key, order_key, Role, RowData as Row, Table,
};

use crate::library_order::{first_lane_per_file, order_family, order_rows};
use untwist_rename::library::{unit_records, version_key, UnitRecord};

#[derive(clap::Args, Debug)]
pub struct Request {
    pub id: String,
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    pub jobs: usize,
    #[arg(long)]
    pub versions: Option<String>,
    #[arg(long)]
    pub bundlers: Option<String>,
    #[arg(long)]
    pub minifiers: Option<String>,
    #[arg(long)]
    pub obfuscate: Option<String>,
    #[arg(long)]
    pub work: Option<PathBuf>,
    #[arg(long)]
    pub reuse_builds: bool,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub only: Option<String>,
}

const MIN_UNIT_TOKENS: u32 = 12;
const LANES: [(&str, &[&str]); 2] = [("default", &[]), ("unsafe", &["--unsafe"])];

pub(crate) struct Build {
    file: String,
    version: String,
    dir: String,
    bundler: String,
    minifier: String,
    obfuscated: bool,
    minified: bool,
    names: BTreeMap<String, String>,
}

pub(crate) struct Manifest {
    label: String,
    group: String,
    versions: Vec<String>,
    packages: BTreeMap<String, BTreeMap<String, String>>,
    builds: Vec<Build>,
    npm: bool,
    owners: BTreeMap<String, String>,
}

const NPM_KEY: &str = "npm:";
const NODE_MODULES: &str = "node_modules/";
const SCRIPT_EXTENSIONS: [&str; 3] = [".js", ".mjs", ".cjs"];

pub(crate) struct Owner {
    pub(crate) label: String,
    pub(crate) version: String,
    pub(crate) module: String,
}

pub(crate) fn parse_npm_key(key: &str) -> Option<Owner> {
    let rest = key.strip_prefix(NPM_KEY)?;
    let (pinned, file) = rest.split_once(':')?;
    let (label, version) = pinned.rsplit_once('@')?;
    let module = SCRIPT_EXTENSIONS
        .iter()
        .find_map(|extension| file.strip_suffix(extension))
        .unwrap_or(file);
    (!label.is_empty() && !version.is_empty() && !module.is_empty()).then(|| Owner {
        label: label.to_string(),
        version: version.to_string(),
        module: module.to_string(),
    })
}

fn npm_key(path: &str, owners: &BTreeMap<String, String>) -> Option<String> {
    let path = path.replace('\\', "/");
    let start = path.find(NODE_MODULES)?;
    let path = &path[start..];
    let last = path.rfind(NODE_MODULES)?;
    let after = &path[last + NODE_MODULES.len()..];
    let mut parts = after.splitn(3, '/');
    let first = parts.next()?;
    let (name_len, file) = match first.starts_with('@') {
        true => {
            let second = parts.next()?;
            (first.len() + 1 + second.len(), parts.next()?)
        }
        false => {
            let rest = after.get(first.len() + 1..)?;
            (first.len(), rest)
        }
    };
    let chain = &path[..last + NODE_MODULES.len() + name_len];
    let pinned = owners.get(chain)?;
    let scripted = SCRIPT_EXTENSIONS
        .iter()
        .any(|extension| file.ends_with(extension));
    (scripted && !file.is_empty()).then(|| format!("{NPM_KEY}{pinned}:{file}"))
}

fn string_map(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

impl Manifest {
    pub(crate) fn builds_len(&self) -> usize {
        self.builds.len()
    }

    pub(crate) fn build(&self, index: usize) -> &Build {
        &self.builds[index]
    }
}

pub(crate) fn read_manifest(path: &Path) -> Result<Manifest> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    let builds = value["builds"]
        .as_array()
        .context("manifest has no builds")?
        .iter()
        .map(|build| Build {
            file: text(&build["file"]),
            version: text(&build["version"]),
            dir: text(&build["dir"]),
            bundler: text(&build["bundler"]),
            minifier: text(&build["minifier"]),
            obfuscated: text(&build["obfuscator"]) != "none",
            minified: text(&build["minifier"]) != "none",
            names: string_map(&build["names"]),
        })
        .collect();
    let packages = value["packages"]
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(dir, files)| (dir.clone(), string_map(files)))
                .collect()
        })
        .unwrap_or_default();
    Ok(Manifest {
        npm: value["layout"].as_str() == Some("node_modules"),
        owners: string_map(&value["owners"]),
        label: text(&value["label"]),
        group: value["group"].as_str().unwrap_or("core").to_string(),
        versions: value["versions"]
            .as_array()
            .map(|list| list.iter().map(text).collect())
            .unwrap_or_default(),
        packages,
        builds,
    })
}

fn untwist_binary(root: &Path, request: &Request) -> Result<PathBuf> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let binary = request
        .untwist
        .clone()
        .unwrap_or_else(|| target.join("release/untwist"));
    if !binary.exists() {
        bail!("build first: cargo build --release -p untwist-cli");
    }
    Ok(binary)
}

fn run_builds(root: &Path, request: &Request, work: &Path) -> Result<()> {
    let tools = root.join("tools/library-fingerprints");
    let mut command = Command::new("node");
    command
        .current_dir(&tools)
        .arg("build.mjs")
        .arg(&request.id)
        .arg(work);
    let flags = [
        ("--versions", &request.versions),
        ("--bundlers", &request.bundlers),
        ("--minifiers", &request.minifiers),
        ("--obfuscate", &request.obfuscate),
    ];
    for (flag, value) in flags {
        if let Some(value) = value {
            command.arg(flag).arg(value);
        }
    }
    let status = command.status().context("running node build.mjs")?;
    if !status.success() {
        bail!("build.mjs failed for {}", request.id);
    }
    Ok(())
}

fn source_words(text: &str) -> std::collections::HashSet<String> {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn unsuffixed(name: &str, words: &std::collections::HashSet<String>) -> String {
    let stem = name.trim_end_matches(|ch: char| ch.is_ascii_digit());
    let introduced = stem.len() < name.len() && stem.len() >= 2 && words.contains(stem);
    if introduced {
        return stem.to_string();
    }
    name.to_string()
}

fn clean_name(
    name: &str,
    rollup: bool,
    words: &std::collections::HashSet<String>,
) -> Option<String> {
    let mut name = unsuffixed(name, words);
    if let Some((stem, suffix)) = name.rsplit_once('$') {
        if !stem.is_empty()
            && !suffix.is_empty()
            && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            name = stem.to_string();
        }
    }
    if rollup {
        if let Some((stem, digits)) = name.rsplit_once('_') {
            if !stem.is_empty() && !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            {
                name = stem.to_string();
            }
        }
    }
    let valid = name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_' || ch == '$')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
        && !name.starts_with("__unused");
    valid.then_some(name)
}

#[derive(Clone)]
struct Origin {
    span: Span,
    module: String,
    root: Option<Span>,
    nested: bool,
}

struct Attributor<'m> {
    dir: &'m str,
    files: &'m BTreeMap<String, String>,
    names: &'m BTreeMap<String, String>,
    owners: Option<&'m BTreeMap<String, String>>,
    contexts: Vec<Origin>,
}

fn function_root(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

impl Attributor<'_> {
    fn module_of_path(&self, path: &str) -> Option<String> {
        if let Some(owners) = self.owners {
            return npm_key(path, owners);
        }
        let marker = format!("{}/", self.dir);
        let at = path.find(&marker)?;
        self.files.get(&path[at + marker.len()..]).cloned()
    }

    fn top_level(&mut self, statement: &Statement<'_>) {
        let mut declared: Vec<(String, Option<Span>)> = Vec::new();
        match statement {
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    declared.push((id.name.to_string(), Some(function.span)));
                }
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                        let root = declarator.init.as_ref().and_then(function_root);
                        declared.push((id.name.to_string(), root));
                    }
                }
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    declared.push((id.name.to_string(), None));
                }
            }
            _ => {}
        }
        for (name, root) in declared {
            if let Some(module) = self.names.get(&name) {
                self.contexts.push(Origin {
                    span: statement.span(),
                    module: module.clone(),
                    root,
                    nested: false,
                });
                return;
            }
        }
    }
}

impl Attributor<'_> {
    fn nested(&mut self, name: &str, span: Span) {
        if self.owners.is_none() {
            return;
        }
        let Some(module) = self.names.get(name) else {
            return;
        };
        let enclosing = self
            .contexts
            .iter()
            .filter(|context| context.span.start <= span.start && span.end <= context.span.end)
            .min_by_key(|context| context.span.size());
        let package = |key: &str| parse_npm_key(key).map(|owner| owner.label);
        if enclosing.is_some_and(|context| package(&context.module) != package(module)) {
            return;
        }
        self.contexts.push(Origin {
            span,
            module: module.clone(),
            root: None,
            nested: true,
        });
    }
}

impl<'a> Visit<'a> for Attributor<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if let Some(id) = &it.id {
            self.nested(&id.name, it.span);
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if function_root(init).is_some() {
                self.nested(&id.name, it.span);
            }
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_program(&mut self, it: &Program<'a>) {
        for statement in &it.body {
            self.top_level(statement);
        }
        walk::walk_program(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let key = match &it.key {
            PropertyKey::StringLiteral(literal) => {
                Some(untwist_core::ast_util::js_text::js_text(literal.value).into_owned())
            }
            _ => None,
        };
        if let Some(module) = key.as_deref().and_then(|key| self.module_of_path(key)) {
            self.contexts.push(Origin {
                span: it.span,
                module,
                root: function_root(&it.value),
                nested: false,
            });
        }
        walk::walk_object_property(self, it);
    }
}

#[derive(Default)]
struct Literals {
    found: Vec<(Span, String)>,
}

impl<'a> Visit<'a> for Literals {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.found.push((
            it.span,
            untwist_core::ast_util::js_text::js_text(it.value).into_owned(),
        ));
    }
}

const KEY_MIN_LENGTH: usize = 6;
const KEY_LIMIT: usize = 300;

fn is_module_path(text: &str) -> bool {
    text.starts_with("./")
        || text.starts_with("../")
        || (text.contains('/') && text.ends_with(".js"))
}

fn literal_modules(
    program: &Program<'_>,
    (contexts, file_module): (&[Origin], Option<&str>),
    dir: &str,
) -> Vec<(String, String)> {
    let mut literals = Literals::default();
    literals.visit_program(program);
    literals
        .found
        .into_iter()
        .filter(|(_, text)| {
            text.chars().count() >= KEY_MIN_LENGTH && !text.chars().all(|ch| ch.is_ascii_digit())
        })
        .filter(|(_, text)| {
            (dir.is_empty() || !text.contains(dir))
                && !text.contains("node_modules")
                && !is_module_path(text)
        })
        .filter_map(|(span, text)| {
            let module = match file_module {
                Some(module) => module.to_string(),
                None => contexts
                    .iter()
                    .filter(|context| {
                        context.span.start <= span.start && span.end <= context.span.end
                    })
                    .min_by_key(|context| context.span.size())?
                    .module
                    .clone(),
            };
            Some((text, module))
        })
        .collect()
}

pub(crate) type BuildFindings = (Vec<Observed>, Vec<(String, String)>);

pub(crate) struct Observed {
    row: Row,
    known: bool,
}

fn attribute(
    records: Vec<UnitRecord>,
    contexts: &[Origin],
    file_module: Option<&str>,
) -> Vec<(UnitRecord, String, Role)> {
    records
        .into_iter()
        .filter_map(|record| {
            if let Some(module) = file_module {
                let role = match record.kind {
                    UnitKind::Program => Role::Module,
                    UnitKind::Function => Role::Function,
                };
                return Some((record, module.to_string(), role));
            }
            if record.kind == UnitKind::Program {
                return None;
            }
            let context = contexts
                .iter()
                .filter(|context| {
                    context.span.start <= record.span.start && record.span.end <= context.span.end
                })
                .min_by_key(|context| context.span.size())?;
            let role = match context.root {
                _ if context.nested => Role::Function,
                Some(root) if root == record.span => Role::Wrapper,
                None if context.span == record.span => Role::Wrapper,
                _ => Role::Function,
            };
            Some((record, context.module.clone(), role))
        })
        .collect()
}

const GENERATED_STEMS: [&str; 4] = ["fn", "module", "requireModule", "hasRequiredModule"];

fn generated(name: &str) -> bool {
    let stem = name.trim_end_matches(|ch: char| ch.is_ascii_digit());
    let numbered = stem.len() < name.len();
    name.len() <= 2 || GENERATED_STEMS.contains(&stem) || (numbered && stem.len() == 1)
}

fn is_strong(record: &UnitRecord) -> bool {
    record.loose.0 >= untwist_rename::library::STRONG_LENGTH
        && record.specific >= untwist_rename::library::STRONG_SPECIFIC
}

fn names(
    list: &[String],
    known: bool,
    rollup: bool,
    words: &std::collections::HashSet<String>,
) -> Vec<Option<String>> {
    list.iter()
        .map(|name| known.then(|| clean_name(name, rollup, words)).flatten())
        .collect()
}

fn owner_of(
    manifest: &Manifest,
    build: &Build,
    module: String,
) -> Option<(String, String, String)> {
    match manifest.npm {
        true => {
            let owner = parse_npm_key(&module)?;
            Some((owner.label, owner.version, owner.module))
        }
        false => Some((manifest.label.clone(), build.version.clone(), module)),
    }
}

fn observe_order(
    program: &Program<'_>,
    scoping: &oxc_semantic::Scoping,
    contexts: &[Origin],
    (build, manifest): (&Build, &Manifest),
) -> Vec<Observed> {
    let Some(family) = order_family(&build.bundler, &build.minifier, build.obfuscated) else {
        return Vec::new();
    };
    let shapes = statement_shapes(program, scoping);
    let contexts: Vec<(Span, String)> = contexts
        .iter()
        .map(|context| (context.span, context.module.clone()))
        .collect();
    order_rows(&shapes, &contexts)
        .into_iter()
        .filter_map(|row| {
            let (label, versions, module) = owner_of(manifest, build, row.module)?;
            Some(Observed {
                known: true,
                row: Row {
                    loose: loose_key(row.loose),
                    exact: order_key(row.loose),
                    label,
                    versions,
                    module,
                    role: Role::Order,
                    own: Some(family.to_string()),
                    inner: Vec::new(),
                    outer: Vec::new(),
                    specific: row.position,
                },
            })
        })
        .collect()
}

fn observe_file(
    text: &str,
    file_module: Option<&str>,
    (build, manifest): (&Build, &Manifest),
    words: &std::collections::HashSet<String>,
) -> (Vec<Observed>, Vec<(String, String)>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return (Vec::new(), Vec::new());
    }
    let sem = build_scoping(&parsed.program);
    let empty = BTreeMap::new();
    let mut attributor = Attributor {
        dir: &build.dir,
        files: manifest.packages.get(&build.dir).unwrap_or(&empty),
        names: &build.names,
        owners: manifest.npm.then_some(&manifest.owners),
        contexts: Vec::new(),
    };
    if file_module.is_none() {
        attributor.visit_program(&parsed.program);
    }
    let records = unit_records(&parsed.program, &sem.scoping);
    let rollup = build.bundler.starts_with("rollup");
    let keys = literal_modules(
        &parsed.program,
        (&attributor.contexts, file_module),
        &build.dir,
    );
    let mut observed: Vec<Observed> = attribute(records, &attributor.contexts, file_module)
        .into_iter()
        .filter(|(record, _, _)| record.loose.0 >= MIN_UNIT_TOKENS)
        .filter(|(record, _, _)| is_strong(record) || record.binding.is_some())
        .filter_map(|(record, module, role)| {
            let (label, versions, module) = owner_of(manifest, build, module)?;
            let known = !build.obfuscated;
            let own_known = known || role == Role::Wrapper;
            let real =
                |name: Option<String>| name.filter(|name| !build.minified || !generated(name));
            Some(Observed {
                known,
                row: Row {
                    loose: loose_key(record.loose),
                    exact: exact_key(record.exact),
                    label,
                    versions,
                    module,
                    role,
                    own: record
                        .binding
                        .as_deref()
                        .filter(|_| own_known)
                        .and_then(|name| real(clean_name(name, rollup, words))),
                    inner: names(&record.inner, known, rollup, words)
                        .into_iter()
                        .map(real)
                        .collect(),
                    outer: names(&record.outer, known, rollup, words)
                        .into_iter()
                        .map(real)
                        .collect(),
                    specific: record.specific,
                },
            })
        })
        .collect();
    if file_module.is_none() {
        observed.extend(observe_order(
            &parsed.program,
            &sem.scoping,
            &attributor.contexts,
            (build, manifest),
        ));
    }
    (observed, keys)
}

fn bundle_modules(
    out: &Path,
    build: &Build,
    attributor_files: &BTreeMap<String, String>,
    owners: Option<&BTreeMap<String, String>>,
) -> BTreeMap<PathBuf, String> {
    let Ok(text) = std::fs::read_to_string(out.join("bundle.json")) else {
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return BTreeMap::new();
    };
    let marker = format!("{}/", build.dir);
    value["modules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|module| {
            let id = module["id"].as_str()?;
            let path = module["path"].as_str()?;
            if let Some(owners) = owners {
                return Some((out.join(path), npm_key(id, owners)?));
            }
            let at = id.find(&marker)?;
            let library_module = attributor_files.get(&id[at + marker.len()..])?;
            Some((out.join(path), library_module.clone()))
        })
        .collect()
}

pub(crate) fn process_build(
    untwist: &Path,
    work: &Path,
    build: &Build,
    manifest: &Manifest,
) -> Result<BuildFindings> {
    let mut observed = Vec::new();
    let mut keys = Vec::new();
    let empty = BTreeMap::new();
    let files = manifest.packages.get(&build.dir).unwrap_or(&empty);
    let words = source_words(&std::fs::read_to_string(work.join(&build.file))?);
    let mut order_lanes = Vec::new();
    for (lane, flags) in LANES {
        let mut order = Vec::new();
        let out = work.join("out").join(format!("{}.{lane}", build.file));
        let status = Command::new(untwist)
            .args(flags)
            .arg("-o")
            .arg(&out)
            .arg("-f")
            .arg(work.join(&build.file))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| format!("running untwist on {}", build.file))?;
        if !status.success() {
            eprintln!("untwist failed on {} ({lane})", build.file);
            continue;
        }
        let modules = bundle_modules(&out, build, files, manifest.npm.then_some(&manifest.owners));
        for entry in std::fs::read_dir(&out)? {
            let path = entry?.path();
            if path.extension().is_none_or(|ext| ext != "js") {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            let module = modules.get(&path).map(String::as_str);
            let (rows, found) = observe_file(&text, module, (build, manifest), &words);
            let (in_order, rest): (Vec<Observed>, Vec<Observed>) = rows
                .into_iter()
                .partition(|item| item.row.role == Role::Order);
            observed.extend(rest);
            order.extend(in_order);
            keys.extend(found);
        }
        order_lanes.push(order);
        let _ = std::fs::remove_dir_all(&out);
    }
    observed.extend(first_lane_per_file(order_lanes, |item: &Observed| {
        (item.row.label.clone(), item.row.module.clone())
    }));
    Ok((observed, keys))
}

#[derive(Default)]
struct Aggregate {
    loose: u32,
    specific: u32,
    versions: BTreeSet<String>,
    own: Option<Option<String>>,
    inner: Vec<Option<Option<String>>>,
    outer: Vec<Option<Option<String>>>,
}

fn name_key(name: &str) -> String {
    name.chars()
        .filter(|ch| *ch != '_' && *ch != '$')
        .flat_map(char::to_lowercase)
        .collect()
}

fn merge_slot(slot: &mut Option<Option<String>>, name: Option<String>) {
    let Some(name) = name else {
        return;
    };
    match slot {
        None => *slot = Some(Some(name)),
        Some(Some(existing)) if name_key(existing) != name_key(&name) => *slot = Some(None),
        Some(Some(existing)) if name < *existing => *existing = name,
        _ => {}
    }
}

fn merge_list(into: &mut Vec<Option<Option<String>>>, from: Vec<Option<String>>) {
    if into.len() < from.len() {
        into.resize(from.len(), None);
    }
    for (slot, name) in into.iter_mut().zip(from) {
        merge_slot(slot, name);
    }
}

fn settle(list: &[Option<Option<String>>]) -> Vec<Option<String>> {
    list.iter().map(|slot| slot.clone().flatten()).collect()
}

fn ascending(versions: &[String]) -> Vec<String> {
    let mut sorted = versions.to_vec();
    sorted.sort_by(|a, b| version_key(a).cmp(&version_key(b)).then_with(|| a.cmp(b)));
    sorted.dedup();
    sorted
}

fn version_ranges(present: &BTreeSet<String>, order: &[String]) -> String {
    let mut ranges: Vec<(String, String)> = Vec::new();
    let mut open: Option<(String, String)> = None;
    for version in &ascending(order) {
        if present.contains(version) {
            open = Some(match open.take() {
                Some((low, _)) => (low, version.clone()),
                None => (version.clone(), version.clone()),
            });
        } else if let Some(range) = open.take() {
            ranges.push(range);
        }
    }
    ranges.extend(open);
    ranges
        .into_iter()
        .map(|(low, high)| match low == high {
            true => low,
            false => format!("{low}~{high}"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

type OrderedKey = (u64, String, String, Role, Option<(u32, Option<String>)>);

fn order_slot(row: &Row) -> Option<(u32, Option<String>)> {
    (row.role == Role::Order).then(|| (row.specific, row.own.clone()))
}

pub(crate) fn aggregate(observed: Vec<Observed>, manifest: &Manifest) -> Vec<Row> {
    let mut groups: BTreeMap<OrderedKey, Aggregate> = BTreeMap::new();
    let mut orders: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut sorted = observed;
    sorted.sort_by_key(|item| !item.known);
    for Observed { row, known } in sorted {
        orders
            .entry(row.label.clone())
            .or_default()
            .insert(row.versions.clone());
        let group = groups
            .entry((
                row.exact,
                row.label.clone(),
                row.module.clone(),
                row.role,
                order_slot(&row),
            ))
            .or_default();
        group.loose = row.loose;
        group.specific = row.specific;
        group.versions.insert(row.versions.clone());
        merge_slot(&mut group.own, row.own);
        if known {
            merge_list(&mut group.inner, row.inner);
            merge_list(&mut group.outer, row.outer);
        }
    }
    groups
        .into_iter()
        .map(|((exact, label, module, role, _), group)| Row {
            loose: group.loose,
            exact,
            versions: match manifest.npm {
                true => version_ranges(
                    &group.versions,
                    &orders[&label].iter().cloned().collect::<Vec<_>>(),
                ),
                false => version_ranges(&group.versions, &manifest.versions),
            },
            label,
            module,
            role,
            own: group.own.flatten().filter(|name| !name.is_empty()),
            inner: settle(&group.inner),
            outer: settle(&group.outer),
            specific: group.specific,
        })
        .collect()
}

fn write_keys(
    table: &Path,
    id: &str,
    label: &str,
    keys: BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    let mut ranked: Vec<(String, usize)> = keys
        .into_iter()
        .map(|(key, modules)| (key, modules.len()))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(KEY_LIMIT);
    let mut chosen: Vec<String> = ranked.into_iter().map(|(key, _)| key).collect();
    chosen.sort();
    let json = serde_json::json!({ "id": id, "label": label, "keys": chosen });
    let path = table.with_file_name(format!("{id}.keys.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&json)? + "\n")?;
    Ok(())
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let untwist = untwist_binary(root, request)?;
    let work = request.work.clone().unwrap_or_else(|| {
        root.join("tools/library-fingerprints/node_modules/.work/tables")
            .join(&request.id)
    });
    if !request.reuse_builds {
        run_builds(root, request, &work)?;
    }
    let mut manifest = read_manifest(&work.join("manifest.json"))?;
    if let Some(only) = &request.only {
        manifest
            .builds
            .retain(|build| build.file.contains(only.as_str()));
    }
    let next = AtomicUsize::new(0);
    let observed: Mutex<Vec<Observed>> = Mutex::new(Vec::new());
    let keys: Mutex<BTreeMap<String, BTreeSet<String>>> = Mutex::new(BTreeMap::new());
    let total = manifest.builds.len();
    std::thread::scope(|scope| {
        for _ in 0..request.jobs.max(1) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(build) = manifest.builds.get(index) else {
                    break;
                };
                match process_build(&untwist, &work, build, &manifest) {
                    Ok((rows, found)) => {
                        let mut known = keys.lock().expect("keys lock");
                        for (key, module) in found {
                            known.entry(key).or_default().insert(module);
                        }
                        drop(known);
                        let mut all = observed.lock().expect("observed lock");
                        all.extend(rows);
                        eprintln!(
                            "[{}/{total}] {} ({} units so far)",
                            index + 1,
                            build.file,
                            all.len()
                        );
                    }
                    Err(error) => eprintln!("{}: {error:#}", build.file),
                }
            });
        }
    });
    let observed = observed.into_inner().expect("observed lock");
    let rows = aggregate(observed, &manifest);
    let out = request.out.clone().unwrap_or_else(|| {
        root.join(format!("crates/untwist-libraries-{}/data", manifest.group))
            .join(format!("{}.table.gz", request.id))
    });
    let text = Table::from_rows(&rows).write(&request.id);
    crate::library_gz::write(&out, &text)?;
    let keys = keys.into_inner().expect("keys lock");
    write_keys(&out, &request.id, &manifest.label, keys)?;
    eprintln!("{} rows -> {}", rows.len(), out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use untwist_rename::library::parse_ranges;

    fn present(versions: &[&str]) -> BTreeSet<String> {
        versions.iter().map(|version| version.to_string()).collect()
    }

    fn order(versions: &[&str]) -> Vec<String> {
        versions.iter().map(|version| version.to_string()).collect()
    }

    #[test]
    fn a_spec_listed_newest_first_still_gives_ascending_ranges() {
        let listed = order(&["0.187.0", "0.170.0", "0.160.1", "0.150.1", "0.140.2"]);
        let ranges = version_ranges(&present(&["0.187.0", "0.170.0", "0.150.1"]), &listed);
        assert_eq!(ranges, "0.150.1,0.170.0~0.187.0");
        assert_eq!(parse_ranges(&ranges).len(), 2);
    }

    #[test]
    fn versions_sort_by_number_not_by_text() {
        let listed = order(&["1.10.0", "1.9.2", "1.2.0"]);
        assert_eq!(
            version_ranges(&present(&["1.9.2", "1.10.0"]), &listed),
            "1.9.2~1.10.0"
        );
    }

    #[test]
    fn a_bundled_path_is_credited_to_the_package_that_owns_it() {
        let owners: BTreeMap<String, String> = [
            ("node_modules/react-dom", "react-dom@19.3.1"),
            (
                "node_modules/react-dom/node_modules/scheduler",
                "scheduler@0.23.2",
            ),
            ("node_modules/@vue/shared", "@vue/shared@3.5.46"),
        ]
        .into_iter()
        .map(|(chain, pinned)| (chain.to_string(), pinned.to_string()))
        .collect();
        assert_eq!(
            npm_key(
                "./node_modules/react-dom/node_modules/scheduler/cjs/scheduler.production.js",
                &owners
            )
            .as_deref(),
            Some("npm:scheduler@0.23.2:cjs/scheduler.production.js")
        );
        assert_eq!(
            npm_key(
                "/abs/pkgs/x/node_modules/@vue/shared/dist/shared.esm-bundler.js",
                &owners
            )
            .as_deref(),
            Some("npm:@vue/shared@3.5.46:dist/shared.esm-bundler.js")
        );
        assert_eq!(
            npm_key("./node_modules/react-dom/package.json", &owners),
            None
        );
        assert_eq!(npm_key("./src/app.js", &owners), None);
        let owner =
            parse_npm_key("npm:@vue/shared@3.5.46:dist/shared.esm-bundler.js").expect("a key");
        assert_eq!(
            (
                owner.label.as_str(),
                owner.version.as_str(),
                owner.module.as_str()
            ),
            ("@vue/shared", "3.5.46", "dist/shared.esm-bundler")
        );
    }
}
