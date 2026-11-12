use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::bundle::BundleOutput;
use crate::pipeline::Output;
use crate::plan::{plan, Plan, PlanRefusal, PlannedFile, Role, ACTION_REMOVE, ACTION_WRITE};
use anyhow::{Context, Result};

pub const DEOBFUSCATED: &str = "deobfuscated.js";
pub const BUNDLE_JSON: &str = "bundle.json";

const SINGLE_FILE_HEADER: &str =
    "// untwist: every module of this bundle, concatenated for reading. require() calls still \
     name the paths in bundle.json; this file is not runnable.";

fn push_section(text: &mut String, banner: &str, code: &str) {
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(&format!("// ==== {} ====\n", crate::printable(banner)));
    text.push_str(code);
    if !code.ends_with('\n') {
        text.push('\n');
    }
}

impl Output {
    pub fn concatenated(&self) -> Cow<'_, str> {
        let Some(bundle) = self.bundle.as_ref() else {
            return self.concatenated_inputs();
        };
        let mut text = String::from(SINGLE_FILE_HEADER);
        text.push('\n');
        let mut modules: Vec<&crate::bundle::ModuleOutput> = bundle.modules.iter().collect();
        modules.sort_by_key(|module| (!module.is_entry, module.path.clone()));
        for module in modules {
            push_section(&mut text, &module.path, &module.code);
        }
        Cow::Owned(text)
    }

    fn concatenated_inputs(&self) -> Cow<'_, str> {
        match self.codes.as_slice() {
            [] => Cow::Borrowed(""),
            [only] => Cow::Borrowed(only.as_str()),
            codes => {
                let mut text = String::new();
                for (index, code) in codes.iter().enumerate() {
                    push_section(&mut text, &format!("input {}", index + 1), code);
                }
                Cow::Owned(text)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct OutputConflict {
    pub paths: Vec<String>,
}

impl fmt::Display for OutputConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.paths.as_slice() {
            [] => write!(f, "the output directory already holds this run's files"),
            [only] => write!(f, "{only:?} already exists in the output directory"),
            [first, _] => write!(
                f,
                "{first:?} and 1 more file already exist in the output directory"
            ),
            [first, rest @ ..] => write!(
                f,
                "{first:?} and {} more files already exist in the output directory",
                rest.len()
            ),
        }
    }
}

impl std::error::Error for OutputConflict {}

fn is_occupied(dir: &Path, relative: &str) -> bool {
    let mut current = dir.to_path_buf();
    let mut segments = relative.split('/').peekable();
    while let Some(segment) = segments.next() {
        current.push(segment);
        let Ok(metadata) = std::fs::metadata(&current) else {
            return false;
        };
        if segments.peek().is_none() || !metadata.is_dir() {
            return true;
        }
    }
    false
}

fn occupied(dir: &Path, planned: &[PlannedFile]) -> Vec<String> {
    planned
        .iter()
        .filter(|file| is_occupied(dir, &file.relative))
        .map(|file| file.relative.clone())
        .collect()
}

fn canonical_inputs(inputs: &[PathBuf]) -> Vec<PathBuf> {
    inputs
        .iter()
        .filter_map(|input| input.canonicalize().ok())
        .collect()
}

fn is_input(path: &Path, inputs: &[PathBuf]) -> bool {
    !inputs.is_empty()
        && path
            .canonicalize()
            .is_ok_and(|canonical| inputs.contains(&canonical))
}

fn without_inputs(plan: Plan, inputs: &[PathBuf]) -> Plan {
    let (files, dropped): (Vec<PlannedFile>, Vec<PlannedFile>) = plan
        .files
        .into_iter()
        .partition(|file| !is_input(&file.path, inputs));
    let dropped_modules = dropped.iter().filter_map(|file| match file.role {
        Role::Module(index) => Some(index),
        _ => None,
    });
    let unwritten_modules = plan
        .unwritten_modules
        .into_iter()
        .chain(dropped_modules)
        .collect();
    let refused = plan
        .refused
        .into_iter()
        .chain(
            dropped
                .iter()
                .map(|file| PlanRefusal::InputFile(file.relative.clone())),
        )
        .collect();
    Plan {
        files,
        refused,
        unwritten_modules,
    }
}

fn recorded_module_paths(dir: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(dir.join(BUNDLE_JSON)) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(modules) = json.get("modules").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    modules
        .iter()
        .filter_map(|module| module.get("path").and_then(serde_json::Value::as_str))
        .filter(|path| crate::plan::is_safe_relative(path))
        .map(|path| dir.join(path))
        .collect()
}

pub fn stale_paths(dir: &Path, planned: &[PlannedFile]) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = planned.iter().map(|file| file.path.clone()).collect();
    for name in [DEOBFUSCATED, BUNDLE_JSON] {
        paths.push(dir.join(name));
    }
    paths.extend(recorded_module_paths(dir));
    paths.sort();
    paths.dedup();
    paths
}

fn is_absent(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

fn symlink_free(dir: &Path, relative: &str) -> Result<bool> {
    let mut current = dir.to_path_buf();
    for segment in relative.split('/') {
        current.push(segment);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Ok(false);
                }
            }
            Err(err) if is_absent(&err) => {}
            Err(err) => {
                return Err(err).with_context(|| format!("check {}", current.display()));
            }
        }
    }
    Ok(true)
}

fn failed(path: &str, action: &'static str, error: &anyhow::Error) -> PlanRefusal {
    PlanRefusal::Failed {
        path: path.to_string(),
        action,
        reason: format!("{error:#}"),
    }
}

fn removed(dir: &Path, path: &Path, relative: &str) -> Result<bool> {
    if !symlink_free(dir, relative)? {
        return Ok(false);
    }
    if path.is_file() {
        std::fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(true)
}

fn removal_refusal(dir: &Path, path: &Path) -> Option<PlanRefusal> {
    let Some(relative) = path.strip_prefix(dir).ok().and_then(Path::to_str) else {
        return Some(PlanRefusal::NotRemoved(path.display().to_string()));
    };
    match removed(dir, path, relative) {
        Ok(true) => None,
        Ok(false) => Some(PlanRefusal::NotRemoved(relative.to_string())),
        Err(error) => Some(failed(relative, ACTION_REMOVE, &error)),
    }
}

pub fn clean(dir: &Path, paths: &[PathBuf]) -> Vec<PlanRefusal> {
    paths
        .iter()
        .filter_map(|path| removal_refusal(dir, path))
        .collect()
}

fn deepest_existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path;
    while !current.exists() {
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    current.to_path_buf()
}

fn is_contained(dir_canonical: &Path, target: &Path) -> io::Result<bool> {
    let ancestor = deepest_existing_ancestor(target);
    let ancestor_canonical = ancestor.canonicalize()?;
    Ok(ancestor_canonical.starts_with(dir_canonical))
}

#[cfg(unix)]
fn create_temp_file(parent: &Path) -> io::Result<tempfile::NamedTempFile> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o644))
        .tempfile_in(parent)
}

#[cfg(not(unix))]
fn create_temp_file(parent: &Path) -> io::Result<tempfile::NamedTempFile> {
    tempfile::Builder::new().tempfile_in(parent)
}

const SAME_FILE_ON_DISK: &str =
    "a file already has that name on this file system, written earlier in this run or made during it";

enum Outcome {
    Written,
    Unsafe,
    Taken,
}

fn write_via_temp_file(path: &Path, contents: &[u8]) -> Result<Outcome> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = create_temp_file(parent)
        .with_context(|| format!("create temp file in {}", parent.display()))?;
    temp.write_all(contents)
        .with_context(|| format!("write {}", path.display()))?;
    match temp.persist_noclobber(path) {
        Ok(_) => Ok(Outcome::Written),
        Err(err) if err.error.kind() == io::ErrorKind::AlreadyExists => Ok(Outcome::Taken),
        Err(err) => Err(err.error).with_context(|| format!("persist {}", path.display())),
    }
}

struct Target<'a> {
    dir: &'a Path,
    dir_canonical: PathBuf,
    written: RefCell<HashSet<PathBuf>>,
}

impl Target<'_> {
    fn written(&self, file: &PlannedFile, contents: &[u8]) -> Result<Outcome> {
        if self.written.borrow().contains(&file.path) {
            return Ok(Outcome::Taken);
        }
        if !symlink_free(self.dir, &file.relative)? {
            return Ok(Outcome::Unsafe);
        }
        let contained = is_contained(&self.dir_canonical, &file.path)
            .with_context(|| format!("check {}", file.path.display()))?;
        if !contained {
            return Ok(Outcome::Unsafe);
        }
        if let Some(parent) = file.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let outcome = write_via_temp_file(&file.path, contents)?;
        if matches!(outcome, Outcome::Written) {
            self.written.borrow_mut().insert(file.path.clone());
        }
        Ok(outcome)
    }

    fn refusal(&self, file: &PlannedFile, contents: &[u8]) -> Option<PlanRefusal> {
        match self.written(file, contents) {
            Ok(Outcome::Written) => None,
            Ok(Outcome::Unsafe) => Some(PlanRefusal::Unsafe(file.relative.clone())),
            Ok(Outcome::Taken) => Some(PlanRefusal::Collision {
                path: file.relative.clone(),
                owner: SAME_FILE_ON_DISK,
            }),
            Err(error) => Some(failed(&file.relative, ACTION_WRITE, &error)),
        }
    }
}

fn listing_contents<'a>(
    file: &'a PlannedFile,
    bundle: Option<&BundleOutput>,
    unwritten_modules: &[usize],
) -> Cow<'a, [u8]> {
    match bundle {
        Some(bundle) => Cow::Owned(
            bundle
                .listing_without(unwritten_modules)
                .json()
                .into_bytes(),
        ),
        None => Cow::Borrowed(&file.contents),
    }
}

struct Written {
    files: Vec<String>,
    refused: Vec<PlanRefusal>,
    unwritten_modules: Vec<usize>,
}

fn write_all(dir: &Path, plan: &Plan, bundle: Option<&BundleOutput>) -> Result<Written> {
    let target = Target {
        dir,
        dir_canonical: dir
            .canonicalize()
            .with_context(|| format!("canonicalize {}", dir.display()))?,
        written: RefCell::new(HashSet::new()),
    };
    let mut unwritten_modules = plan.unwritten_modules.clone();
    let mut refusals: Vec<Option<PlanRefusal>> = Vec::with_capacity(plan.files.len());
    for file in &plan.files {
        let refusal = match file.role {
            Role::Listing => None,
            _ => target.refusal(file, &file.contents),
        };
        if let (Some(_), Role::Module(index)) = (&refusal, file.role) {
            unwritten_modules.push(index);
        }
        refusals.push(refusal);
    }
    for (file, refusal) in plan.files.iter().zip(&mut refusals) {
        if file.role == Role::Listing {
            let contents = listing_contents(file, bundle, &unwritten_modules);
            *refusal = target.refusal(file, &contents);
        }
    }
    unwritten_modules.sort_unstable();
    Ok(Written {
        files: plan
            .files
            .iter()
            .zip(&refusals)
            .filter(|(_, refusal)| refusal.is_none())
            .map(|(file, _)| file.relative.clone())
            .collect(),
        refused: refusals.into_iter().flatten().collect(),
        unwritten_modules,
    })
}

#[derive(Debug)]
pub struct WriteReport {
    pub files: Vec<String>,
    pub refused: Vec<PlanRefusal>,
    pub stale: Vec<String>,
    pub first: Option<PathBuf>,
    pub unwritten_modules: Vec<usize>,
}

fn left_behind(dir: &Path, planned: &[PlannedFile]) -> (Vec<String>, Vec<PlanRefusal>) {
    let planned_paths: HashSet<&Path> = planned.iter().map(|file| file.path.as_path()).collect();
    let mut stale = Vec::new();
    let mut refused = Vec::new();
    for path in stale_paths(dir, planned) {
        if planned_paths.contains(path.as_path()) || !path.is_file() {
            continue;
        }
        match path.strip_prefix(dir).ok().and_then(Path::to_str) {
            Some(relative) => stale.push(relative.to_string()),
            None => refused.push(PlanRefusal::NotRemoved(path.display().to_string())),
        }
    }
    (stale, refused)
}

fn removable(dir: &Path, planned: &[PlannedFile], inputs: &[PathBuf]) -> Vec<PathBuf> {
    stale_paths(dir, planned)
        .into_iter()
        .filter(|path| !is_input(path, inputs))
        .collect()
}

impl Output {
    pub fn plan_write(
        &self,
        dir: &Path,
        inputs: &[PathBuf],
    ) -> (Vec<PlannedFile>, Vec<PlanRefusal>) {
        let plan = plan(dir, self, inputs);
        (plan.files, plan.refused)
    }

    pub fn write_to(&self, dir: impl AsRef<Path>) -> Result<WriteReport> {
        self.write_to_with(dir.as_ref(), &[], false)
    }

    pub fn write_to_with(
        &self,
        dir: &Path,
        inputs: &[PathBuf],
        force: bool,
    ) -> Result<WriteReport> {
        let plan = plan(dir, self, inputs);
        if !force {
            let paths = occupied(dir, &plan.files);
            if !paths.is_empty() {
                return Err(OutputConflict { paths }.into());
            }
        }
        let inputs = canonical_inputs(inputs);
        let mut plan = without_inputs(plan, &inputs);
        let (stale, not_removed) = match force && dir.is_dir() {
            true => (
                Vec::new(),
                clean(dir, &removable(dir, &plan.files, &inputs)),
            ),
            false => left_behind(dir, &plan.files),
        };
        plan.refused.extend(not_removed);
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        let written = write_all(dir, &plan, self.bundle.as_ref())?;
        plan.refused.extend(written.refused);
        Ok(WriteReport {
            files: written.files,
            refused: plan.refused,
            stale,
            first: plan.files.first().map(|file| file.path.clone()),
            unwritten_modules: written.unwritten_modules,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{write_all, Plan, PlanRefusal, PlannedFile, Role};
    use std::path::Path;

    fn planned(dir: &Path, relative: &str, contents: &str, role: Role) -> PlannedFile {
        PlannedFile {
            path: dir.join(relative),
            relative: relative.to_string(),
            contents: contents.as_bytes().to_vec(),
            role,
        }
    }

    fn plan_of(files: Vec<PlannedFile>) -> Plan {
        Plan {
            files,
            refused: Vec::new(),
            unwritten_modules: Vec::new(),
        }
    }

    #[test]
    fn a_second_write_to_a_path_of_this_run_is_refused_and_keeps_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan_of(vec![
            planned(dir.path(), "deobfuscated.js", "first", Role::Output),
            planned(dir.path(), "deobfuscated.js", "second", Role::Module(4)),
        ]);
        let written = write_all(dir.path(), &plan, None).unwrap();
        assert_eq!(written.files, ["deobfuscated.js"]);
        assert!(matches!(
            written.refused.as_slice(),
            [PlanRefusal::Collision { path, .. }] if path == "deobfuscated.js"
        ));
        assert_eq!(written.unwritten_modules, [4]);
        let kept = std::fs::read_to_string(dir.path().join("deobfuscated.js")).unwrap();
        assert_eq!(kept, "first");
    }

    #[test]
    fn a_file_that_appears_under_a_planned_name_is_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.js"), "already there").unwrap();
        let plan = plan_of(vec![planned(dir.path(), "a.js", "module", Role::Module(1))]);
        let written = write_all(dir.path(), &plan, None).unwrap();
        assert!(written.files.is_empty());
        assert!(matches!(
            written.refused.as_slice(),
            [PlanRefusal::Collision { path, .. }] if path == "a.js"
        ));
        assert_eq!(written.unwritten_modules, [1]);
        let kept = std::fs::read_to_string(dir.path().join("a.js")).unwrap();
        assert_eq!(kept, "already there");
    }
}
