use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const NPM_PACKAGE: &str = "crates/untwist-wasm/npm/package.json";
const WEB_PACKAGE: &str = "web/package.json";
const WEB_LOCK: &str = "web/package-lock.json";
const CHANGELOG: &str = "CHANGELOG.md";
const INTERNAL_PATH: &str = "path = \"../untwist";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub fn parse(text: &str) -> Result<Version> {
        let parts: Vec<&str> = text.split('.').collect();
        let [major, minor, patch] = parts.as_slice() else {
            bail!("{text:?} is not a MAJOR.MINOR.PATCH version");
        };
        Ok(Version {
            major: number(text, major)?,
            minor: number(text, minor)?,
            patch: number(text, patch)?,
        })
    }
}

fn number(text: &str, part: &str) -> Result<u64> {
    let digits = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits || (part.len() > 1 && part.starts_with('0')) {
        bail!("{text:?} is not a MAJOR.MINOR.PATCH version");
    }
    part.parse()
        .with_context(|| format!("{text:?} has a number that is too large"))
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn quoted_value(line: &str) -> Option<(usize, usize)> {
    let start = line.find('"')? + 1;
    let len = line[start..].find('"')?;
    Some((start, start + len))
}

fn replace_quoted(line: &str, value: &str) -> String {
    match quoted_value(line) {
        Some((start, end)) => format!("{}{value}{}", &line[..start], &line[end..]),
        None => line.to_string(),
    }
}

fn workspace_version_line(cargo_toml: &str) -> Option<usize> {
    let mut in_package = false;
    for (index, line) in cargo_toml.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[workspace.package]";
        } else if in_package && trimmed.starts_with("version") && trimmed.contains('=') {
            return Some(index);
        }
    }
    None
}

fn map_line(text: &str, index: usize, f: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if i == index {
            let body = line.trim_end_matches('\n');
            out.push_str(&f(body));
            out.push_str(&line[body.len()..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

pub fn workspace_version(cargo_toml: &str) -> Result<Version> {
    let index = workspace_version_line(cargo_toml)
        .context("Cargo.toml has no version under [workspace.package]")?;
    let line = cargo_toml.lines().nth(index).unwrap_or_default();
    let (start, end) = quoted_value(line).context("the workspace version is not a string")?;
    Version::parse(&line[start..end])
}

pub fn set_workspace_version(cargo_toml: &str, version: &Version) -> Result<String> {
    let index = workspace_version_line(cargo_toml)
        .context("Cargo.toml has no version under [workspace.package]")?;
    Ok(map_line(cargo_toml, index, |line| {
        replace_quoted(line, &version.to_string())
    }))
}

fn pin_range(line: &str) -> Option<(usize, usize)> {
    if !line.contains(INTERNAL_PATH) {
        return None;
    }
    let key = line.find("version = \"")? + "version = ".len();
    quoted_value(&line[key..]).map(|(start, end)| (key + start, key + end))
}

pub fn internal_pins(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .filter_map(|line| pin_range(line).map(|(start, end)| line[start..end].to_string()))
        .collect()
}

pub fn set_internal_pins(manifest: &str, version: &Version) -> String {
    let mut out = String::with_capacity(manifest.len());
    for line in manifest.split_inclusive('\n') {
        match pin_range(line) {
            Some((start, end)) => {
                out.push_str(&line[..start]);
                out.push_str(&version.to_string());
                out.push_str(&line[end..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

pub fn package_versions(json: &str) -> Result<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let mut versions = Vec::new();
    for found in [&value["version"], &value["packages"][""]["version"]] {
        if let Some(text) = found.as_str() {
            versions.push(text.to_string());
        }
    }
    Ok(versions)
}

fn is_version_key(line: &str, indent: usize) -> bool {
    line.len() > indent
        && line[..indent].bytes().all(|b| b == b' ')
        && line[indent..].starts_with("\"version\":")
}

fn set_json_line(json: &str, index: usize, version: &Version) -> String {
    map_line(json, index, |line| {
        let colon = line.find(':').unwrap_or(0);
        format!(
            "{}{}",
            &line[..=colon],
            replace_quoted(&line[colon + 1..], &version.to_string())
        )
    })
}

fn verified(json: String, version: &Version, expected: usize) -> Result<String> {
    let versions = package_versions(&json)?;
    let want = version.to_string();
    if versions.len() != expected || versions.iter().any(|found| *found != want) {
        bail!("could not set the version to {want}; found {versions:?}");
    }
    Ok(json)
}

pub fn set_package_version(json: &str, version: &Version) -> Result<String> {
    let index = json
        .lines()
        .position(|line| is_version_key(line, 2))
        .context("no top-level \"version\"")?;
    verified(set_json_line(json, index, version), version, 1)
}

pub fn set_lock_version(json: &str, version: &Version) -> Result<String> {
    let lines: Vec<&str> = json.lines().collect();
    let top = lines
        .iter()
        .position(|line| is_version_key(line, 2))
        .context("no top-level \"version\"")?;
    let root = lines
        .iter()
        .position(|line| *line == "    \"\": {")
        .context("no root package under \"packages\"")?;
    let nested = lines[root..]
        .iter()
        .take_while(|line| **line != "    }," && **line != "    }")
        .position(|line| is_version_key(line, 6))
        .context("the root package has no \"version\"")?;
    let json = set_json_line(json, top, version);
    verified(set_json_line(&json, root + nested, version), version, 2)
}

fn is_unreleased_heading(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("## ") && (lower == "## [unreleased]" || lower.ends_with("- unreleased"))
}

pub fn release_changelog(text: &str, version: &Version, date: &str) -> Result<String> {
    let index = text
        .lines()
        .position(is_unreleased_heading)
        .context("CHANGELOG.md has no Unreleased section")?;
    Ok(map_line(text, index, |_| {
        format!("## [Unreleased]\n\n## [{version}] - {date}")
    }))
}

fn has_section(text: &str, version: &Version) -> bool {
    let heading = format!("## [{version}]");
    text.lines()
        .any(|line| line == heading || line.starts_with(&format!("{heading} ")))
}

pub fn civil_date(days_since_epoch: i64) -> String {
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn read(root: &Path, path: &str) -> Result<String> {
    fs::read_to_string(root.join(path)).with_context(|| format!("read {path}"))
}

fn crate_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifests = Vec::new();
    for entry in fs::read_dir(root.join("crates")).context("read crates/")? {
        let manifest = entry?.path().join("Cargo.toml");
        if manifest.is_file() {
            manifests.push(manifest);
        }
    }
    manifests.sort();
    Ok(manifests)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn workspace_members(root: &Path) -> Result<Vec<String>> {
    let mut manifests = crate_manifests(root)?;
    manifests.push(root.join("xtask/Cargo.toml"));
    let mut names = Vec::new();
    for manifest in manifests.iter().filter(|path| path.is_file()) {
        let text = fs::read_to_string(manifest)?;
        if !text
            .lines()
            .any(|line| line.trim() == "version.workspace = true")
        {
            continue;
        }
        let name = text
            .lines()
            .find(|line| line.trim_start().starts_with("name ="))
            .and_then(|line| quoted_value(line).map(|(start, end)| line[start..end].to_string()))
            .with_context(|| format!("{} has no name", relative(root, manifest)))?;
        names.push(name);
    }
    Ok(names)
}

fn cargo_lock_versions(lock: &str, names: &[String]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            block
                .lines()
                .find(|line| line.starts_with(&format!("{key} = ")))
                .and_then(|line| {
                    quoted_value(line).map(|(start, end)| line[start..end].to_string())
                })
        };
        if block.lines().any(|line| line.starts_with("source = ")) {
            continue;
        }
        if let (Some(name), Some(version)) = (field("name"), field("version")) {
            if names.contains(&name) {
                found.push((name, version));
            }
        }
    }
    found
}

fn mismatches(root: &Path, version: &Version) -> Result<Vec<String>> {
    let want = version.to_string();
    let mut wrong = Vec::new();
    let mut expect = |place: String, found: &str| {
        if found != want {
            wrong.push(format!("{place} has {found}"));
        }
    };
    expect(
        "Cargo.toml".into(),
        &workspace_version(&read(root, "Cargo.toml")?)?.to_string(),
    );
    for manifest in crate_manifests(root)? {
        for pin in internal_pins(&fs::read_to_string(&manifest)?) {
            expect(relative(root, &manifest), &pin);
        }
    }
    for path in [NPM_PACKAGE, WEB_PACKAGE, WEB_LOCK] {
        let versions = package_versions(&read(root, path)?)?;
        if versions.is_empty() {
            expect(path.into(), "no version");
        }
        for found in versions {
            expect(path.into(), &found);
        }
    }
    let members = workspace_members(root)?;
    let locked = cargo_lock_versions(&read(root, "Cargo.lock")?, &members);
    for name in &members {
        match locked.iter().find(|(locked_name, _)| locked_name == name) {
            Some((_, found)) => expect(format!("Cargo.lock ({name})"), found),
            None => expect(format!("Cargo.lock ({name})"), "no entry"),
        }
    }
    if !has_section(&read(root, CHANGELOG)?, version) {
        wrong.push(format!("{CHANGELOG} has no ## [{want}] section"));
    }
    Ok(wrong)
}

pub fn check_version(root: &Path, tag: &str) -> Result<()> {
    let Some(text) = tag.strip_prefix('v') else {
        bail!("tag {tag:?} does not start with v");
    };
    let version = Version::parse(text)?;
    let wrong = mismatches(root, &version)?;
    if !wrong.is_empty() {
        bail!("tag {tag} does not match:\n  {}", wrong.join("\n  "));
    }
    Ok(())
}

fn staged(
    root: &Path,
    path: PathBuf,
    f: impl FnOnce(&str) -> Result<String>,
) -> Result<(PathBuf, String)> {
    let name = relative(root, &path);
    let text = fs::read_to_string(&path).with_context(|| format!("read {name}"))?;
    let updated = f(&text).with_context(|| name.clone())?;
    Ok((path, updated))
}

pub fn bump(root: &Path, new: &str, date: &str) -> Result<Version> {
    let version = Version::parse(new)?;
    let current = workspace_version(&read(root, "Cargo.toml")?)?;
    if version <= current {
        bail!("{version} is not newer than the current version {current}");
    }
    let mut writes = vec![staged(root, root.join("Cargo.toml"), |text| {
        set_workspace_version(text, &version)
    })?];
    for manifest in crate_manifests(root)? {
        writes.push(staged(root, manifest, |text| {
            Ok(set_internal_pins(text, &version))
        })?);
    }
    for path in [NPM_PACKAGE, WEB_PACKAGE] {
        writes.push(staged(root, root.join(path), |text| {
            set_package_version(text, &version)
        })?);
    }
    writes.push(staged(root, root.join(WEB_LOCK), |text| {
        set_lock_version(text, &version)
    })?);
    writes.push(staged(root, root.join(CHANGELOG), |text| {
        release_changelog(text, &version, date)
    })?);
    for (path, text) in writes {
        fs::write(&path, text).with_context(|| format!("write {}", relative(root, &path)))?;
    }
    Ok(version)
}
