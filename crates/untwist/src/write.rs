use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::pipeline::Output;
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
    text.push_str(&format!("// ==== {banner} ====\n"));
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

#[derive(Debug)]
pub struct PlannedFile {
    pub path: PathBuf,
    pub relative: String,
    pub contents: String,
}

pub fn is_safe_relative(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        return false;
    }
    if path.split('/').any(|segment| segment == "..") {
        return false;
    }
    let first = path.split('/').next().unwrap_or_default();
    !(first.len() >= 2 && first.as_bytes().get(1) == Some(&b':'))
}

fn deobfuscated_name(index: usize, total: usize, input: Option<&Path>) -> String {
    if total <= 1 {
        return DEOBFUSCATED.to_string();
    }
    let stem = input
        .and_then(Path::file_stem)
        .and_then(|stem| stem.to_str())
        .filter(|stem| is_safe_relative(stem))
        .map(str::to_string)
        .unwrap_or_else(|| format!("input{index}"));
    format!("deobfuscated.{stem}.js")
}

const OWNER_OUTPUT: &str = "the deobfuscated output is already planned at that path";
const OWNER_MODULE: &str = "an earlier module is already planned at that path";

#[derive(Debug, PartialEq, Eq)]
pub enum PlanRefusal {
    Unsafe(String),
    Collision { path: String, owner: &'static str },
    NotRemoved(String),
    SuffixesExhausted(String),
}

const MAX_DEOBFUSCATED_SUFFIX: usize = 1000;

fn with_stem_suffix(name: &str, suffix: &str) -> String {
    match name.rfind('.') {
        Some(dot) => format!("{}{suffix}{}", &name[..dot], &name[dot..]),
        None => format!("{name}{suffix}"),
    }
}

fn uniquify_deobfuscated_name(base: &str, taken: &mut HashSet<String>) -> Option<String> {
    let mut candidate = base.to_string();
    let mut suffix = 2usize;
    while taken.contains(&candidate) && suffix <= MAX_DEOBFUSCATED_SUFFIX {
        candidate = with_stem_suffix(base, &format!("-{suffix}"));
        suffix += 1;
    }
    taken.insert(candidate.clone()).then_some(candidate)
}

fn plan(dir: &Path, result: &Output, inputs: &[PathBuf]) -> (Vec<PlannedFile>, Vec<PlanRefusal>) {
    if result.single_file {
        return (single_file_plan(dir, result), Vec::new());
    }
    plan_per_module(dir, result, inputs)
}

fn single_file_plan(dir: &Path, result: &Output) -> Vec<PlannedFile> {
    let mut planned = vec![PlannedFile {
        path: dir.join(DEOBFUSCATED),
        relative: DEOBFUSCATED.to_string(),
        contents: result.concatenated().into_owned(),
    }];
    let Some(bundle) = result.bundle.as_ref() else {
        return planned;
    };
    planned.push(PlannedFile {
        path: dir.join(BUNDLE_JSON),
        relative: BUNDLE_JSON.to_string(),
        contents: bundle.bundle_json(),
    });
    planned
}

#[derive(Default)]
struct ModulePlan {
    planned: Vec<PlannedFile>,
    refused: Vec<PlanRefusal>,
    taken: HashSet<String>,
    outputs: HashSet<String>,
}

impl ModulePlan {
    fn plan_output(&mut self, dir: &Path, base: String, code: &str) {
        let Some(relative) = uniquify_deobfuscated_name(&base, &mut self.taken) else {
            self.refused.push(PlanRefusal::SuffixesExhausted(base));
            return;
        };
        self.outputs.insert(relative.clone());
        self.planned.push(PlannedFile {
            path: dir.join(&relative),
            relative,
            contents: code.to_string(),
        });
    }

    fn plan_module(&mut self, dir: &Path, file: crate::bundle::FileOutput) {
        if !is_safe_relative(&file.path) {
            self.refused.push(PlanRefusal::Unsafe(file.path));
            return;
        }
        if !self.taken.insert(file.path.clone()) {
            let owner = match self.outputs.contains(&file.path) {
                true => OWNER_OUTPUT,
                false => OWNER_MODULE,
            };
            self.refused.push(PlanRefusal::Collision {
                path: file.path,
                owner,
            });
            return;
        }
        self.planned.push(PlannedFile {
            path: dir.join(&file.path),
            relative: file.path.clone(),
            contents: file.code,
        });
    }
}

fn plan_per_module(
    dir: &Path,
    result: &Output,
    inputs: &[PathBuf],
) -> (Vec<PlannedFile>, Vec<PlanRefusal>) {
    let mut plan = ModulePlan::default();
    let total = result.codes.len();
    for (index, code) in result.codes.iter().enumerate() {
        let base = deobfuscated_name(index, total, inputs.get(index).map(PathBuf::as_path));
        plan.plan_output(dir, base, code);
    }
    if let Some(bundle) = result.bundle.as_ref() {
        for file in bundle.files() {
            plan.plan_module(dir, file);
        }
    }
    (plan.planned, plan.refused)
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
        .filter(|path| is_safe_relative(path))
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
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err).with_context(|| format!("check {}", current.display()));
            }
        }
    }
    Ok(true)
}

pub fn clean(dir: &Path, paths: &[PathBuf]) -> Result<Vec<String>> {
    let mut refused = Vec::new();
    for path in paths {
        let Ok(relative) = path.strip_prefix(dir) else {
            refused.push(path.display().to_string());
            continue;
        };
        let Some(relative) = relative.to_str() else {
            refused.push(path.display().to_string());
            continue;
        };
        if !symlink_free(dir, relative)? {
            refused.push(relative.to_string());
            continue;
        }
        if path.is_file() {
            std::fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
        }
    }
    Ok(refused)
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

fn write_via_temp_file(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = create_temp_file(parent)
        .with_context(|| format!("create temp file in {}", parent.display()))?;
    temp.write_all(contents.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|err| err.error)
        .with_context(|| format!("persist {}", path.display()))
}

fn write_all(dir: &Path, planned: &[PlannedFile]) -> Result<(Vec<String>, Vec<String>)> {
    let dir_canonical = dir
        .canonicalize()
        .with_context(|| format!("canonicalize {}", dir.display()))?;
    let mut written = Vec::with_capacity(planned.len());
    let mut refused = Vec::new();
    for file in planned {
        if !symlink_free(dir, &file.relative)? {
            refused.push(file.relative.clone());
            continue;
        }
        let contained = is_contained(&dir_canonical, &file.path)
            .with_context(|| format!("check {}", file.path.display()))?;
        if !contained {
            refused.push(file.relative.clone());
            continue;
        }
        if let Some(parent) = file.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        write_via_temp_file(&file.path, &file.contents)?;
        written.push(file.relative.clone());
    }
    Ok((written, refused))
}

#[derive(Debug)]
pub struct WriteReport {
    pub files: Vec<String>,
    pub refused: Vec<PlanRefusal>,
    pub stale: Vec<String>,
    pub first: Option<PathBuf>,
}

fn left_behind(dir: &Path, planned: &[PlannedFile]) -> (Vec<String>, Vec<String>) {
    let planned_paths: HashSet<&Path> = planned.iter().map(|file| file.path.as_path()).collect();
    let mut stale = Vec::new();
    let mut refused = Vec::new();
    for path in stale_paths(dir, planned) {
        if planned_paths.contains(path.as_path()) || !path.is_file() {
            continue;
        }
        match path.strip_prefix(dir).ok().and_then(Path::to_str) {
            Some(relative) => stale.push(relative.to_string()),
            None => refused.push(path.display().to_string()),
        }
    }
    (stale, refused)
}

impl Output {
    pub fn plan_write(
        &self,
        dir: &Path,
        inputs: &[PathBuf],
    ) -> (Vec<PlannedFile>, Vec<PlanRefusal>) {
        plan(dir, self, inputs)
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
        let (planned, mut refused) = self.plan_write(dir, inputs);
        let (stale, not_removed) = match force && dir.is_dir() {
            true => (Vec::new(), clean(dir, &stale_paths(dir, &planned))?),
            false => left_behind(dir, &planned),
        };
        refused.extend(not_removed.into_iter().map(PlanRefusal::NotRemoved));
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        let (files, write_refused) = write_all(dir, &planned)?;
        refused.extend(write_refused.into_iter().map(PlanRefusal::Unsafe));
        Ok(WriteReport {
            files,
            refused,
            stale,
            first: planned.first().map(|file| file.path.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{BundleOutput, ModuleOutput};

    fn output_with(codes: Vec<String>, bundle: Option<BundleOutput>) -> Output {
        Output {
            codes,
            stages: Vec::new(),
            parse_diagnostics: Vec::new(),
            parse_errors: 0,
            notes: Vec::new(),
            bundle,
            panicked: Vec::new(),
            depth_refused: Vec::new(),
            unpack_panicked: false,
            single_file: false,
            heap_peak: 0,
            stats: crate::RunStats::default(),
            #[cfg(feature = "test-hooks")]
            refusal_spans: Vec::new(),
        }
    }

    fn bundle_with(paths: &[&str]) -> BundleOutput {
        BundleOutput {
            kind: "webpack".to_string(),
            entry_id: None,
            chunk_ids: Vec::new(),
            modules: paths
                .iter()
                .enumerate()
                .map(|(index, path)| ModuleOutput {
                    id: index.to_string(),
                    path: (*path).to_string(),
                    is_entry: false,
                    code: format!("module {index}"),
                    #[cfg(feature = "module-spans")]
                    span: None,
                })
                .collect(),
        }
    }

    #[test]
    fn a_collision_names_which_file_already_owns_the_path() {
        let dir = Path::new("/out");
        let against_output = output_with(
            vec!["deobfuscated code".to_string()],
            Some(bundle_with(&[DEOBFUSCATED])),
        );
        let (_, refused) = plan(dir, &against_output, &[]);
        assert_eq!(
            refused,
            vec![PlanRefusal::Collision {
                path: DEOBFUSCATED.to_string(),
                owner: OWNER_OUTPUT,
            }]
        );
        let against_module = output_with(
            vec!["deobfuscated code".to_string()],
            Some(bundle_with(&["shared.js", "shared.js"])),
        );
        let (_, refused) = plan(dir, &against_module, &[]);
        assert_eq!(
            refused,
            vec![PlanRefusal::Collision {
                path: "shared.js".to_string(),
                owner: OWNER_MODULE,
            }]
        );
    }

    #[test]
    fn a_module_colliding_with_the_deobfuscated_output_is_refused() {
        let dir = Path::new("/out");
        let bundle = BundleOutput {
            kind: "webpack".to_string(),
            entry_id: None,
            chunk_ids: Vec::new(),
            modules: vec![ModuleOutput {
                id: "./deobfuscated".to_string(),
                path: "deobfuscated.js".to_string(),
                is_entry: false,
                code: "module code".to_string(),
                #[cfg(feature = "module-spans")]
                span: None,
            }],
        };
        let result = output_with(vec!["deobfuscated code".to_string()], Some(bundle));
        let (planned, refused) = plan(dir, &result, &[]);
        let deobfuscated = planned
            .iter()
            .find(|file| file.relative == DEOBFUSCATED)
            .expect("deobfuscated file planned");
        assert_eq!(deobfuscated.contents, "deobfuscated code");
        assert!(planned.iter().all(|file| file.contents != "module code"));
        assert!(refused.iter().any(
            |refusal| matches!(refusal, PlanRefusal::Collision { path, .. } if path == DEOBFUSCATED)
        ));
    }

    #[test]
    fn a_deobfuscated_name_with_every_suffix_taken_is_refused_not_planned_twice() {
        let mut taken: HashSet<String> = HashSet::new();
        taken.insert("deobfuscated.a.js".to_string());
        for suffix in 2..=MAX_DEOBFUSCATED_SUFFIX {
            taken.insert(format!("deobfuscated.a-{suffix}.js"));
        }
        let before = taken.len();
        assert_eq!(
            uniquify_deobfuscated_name("deobfuscated.a.js", &mut taken),
            None
        );
        assert_eq!(taken.len(), before);
    }

    #[test]
    fn deobfuscated_names_are_uniquified_on_collision() {
        let dir = Path::new("/out");
        let result = output_with(vec!["first".to_string(), "second".to_string()], None);
        let inputs = vec![PathBuf::from("a/bundle.js"), PathBuf::from("b/bundle.js")];
        let (planned, refused) = plan(dir, &result, &inputs);
        assert!(refused.is_empty());
        let names: Vec<&str> = planned.iter().map(|file| file.relative.as_str()).collect();
        assert_eq!(
            names,
            vec!["deobfuscated.bundle.js", "deobfuscated.bundle-2.js"]
        );
    }

    #[test]
    fn empty_path_is_unsafe() {
        assert!(!is_safe_relative(""));
    }

    #[test]
    fn absolute_path_is_unsafe() {
        assert!(!is_safe_relative("/etc/passwd"));
    }

    #[test]
    fn backslash_path_is_unsafe() {
        assert!(!is_safe_relative("sub\\file.js"));
    }

    #[test]
    fn parent_segment_is_unsafe() {
        assert!(!is_safe_relative("../escape.js"));
    }

    #[test]
    fn nested_parent_segment_is_unsafe() {
        assert!(!is_safe_relative("sub/../../escape.js"));
    }

    #[test]
    fn drive_letter_path_is_unsafe() {
        assert!(!is_safe_relative("C:/escape.js"));
    }

    #[test]
    fn safe_relative_path_is_accepted() {
        assert!(is_safe_relative("sub/index.js"));
    }
}
