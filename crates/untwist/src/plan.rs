use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::pipeline::Output;
use crate::write::{BUNDLE_JSON, DEOBFUSCATED};
use untwist_unpack::path::{MAX_PATH_BYTES, MAX_SEGMENT_BYTES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Output,
    Listing,
    Module(usize),
    Wasm,
}

#[derive(Debug)]
pub struct PlannedFile {
    pub path: PathBuf,
    pub relative: String,
    pub contents: Vec<u8>,
    pub(crate) role: Role,
}

pub fn is_safe_relative(path: &str) -> bool {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.starts_with('/') {
        return false;
    }
    if path.contains(['\\', '\0']) {
        return false;
    }
    if path
        .split('/')
        .any(|segment| segment == ".." || segment.len() > MAX_SEGMENT_BYTES)
    {
        return false;
    }
    let first = path.split('/').next().unwrap_or_default();
    !(first.len() >= 2 && first.as_bytes().get(1) == Some(&b':'))
}

fn deobfuscated_name(index: usize, total: usize, input: Option<&Path>) -> String {
    if total <= 1 {
        return DEOBFUSCATED.to_string();
    }
    input
        .and_then(Path::file_stem)
        .and_then(|stem| stem.to_str())
        .filter(|stem| is_safe_relative(stem))
        .map(|stem| format!("deobfuscated.{stem}.js"))
        .filter(|name| is_safe_relative(name))
        .unwrap_or_else(|| format!("deobfuscated.input{index}.js"))
}

const OWNER_OUTPUT: &str = "the deobfuscated output is already planned at that path";
const OWNER_MODULE: &str = "an earlier module is already planned at that path";
const OWNER_LISTING: &str = "the bundle listing is written at that path";
const OWNER_WASM: &str = "a wasm module is already planned at that path";
const OWNER_INSIDE: &str = "other files are already planned inside that path";
const OWNER_ABOVE: &str = "a file is already planned at a parent of that path";

pub(crate) const ACTION_WRITE: &str = "write";
pub(crate) const ACTION_REMOVE: &str = "remove";

#[derive(Debug, PartialEq, Eq)]
pub enum PlanRefusal {
    Unsafe(String),
    Collision {
        path: String,
        owner: &'static str,
    },
    NotRemoved(String),
    SuffixesExhausted(String),
    InputFile(String),
    Failed {
        path: String,
        action: &'static str,
        reason: String,
    },
}

const FIRST_SUFFIX: usize = 2;
const MAX_DEOBFUSCATED_SUFFIX: usize = 1000;

fn with_stem_suffix(name: &str, suffix: &str) -> String {
    match name.rfind('.') {
        Some(dot) => format!("{}{suffix}{}", &name[..dot], &name[dot..]),
        None => format!("{name}{suffix}"),
    }
}

fn claim_key(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("/")
}

fn parents(key: &str) -> impl Iterator<Item = &str> {
    key.match_indices('/').map(|(index, _)| &key[..index])
}

#[derive(Default)]
struct Claims {
    files: HashMap<String, &'static str>,
    directories: HashSet<String>,
}

impl Claims {
    fn owner_of(&self, key: &str) -> Option<&'static str> {
        if let Some(owner) = self.files.get(key) {
            return Some(owner);
        }
        if self.directories.contains(key) {
            return Some(OWNER_INSIDE);
        }
        parents(key)
            .any(|parent| self.files.contains_key(parent))
            .then_some(OWNER_ABOVE)
    }

    fn claim(&mut self, key: String, owner: &'static str) {
        self.directories.extend(parents(&key).map(str::to_string));
        self.files.insert(key, owner);
    }
}

#[derive(Default)]
pub(crate) struct Plan {
    pub(crate) files: Vec<PlannedFile>,
    pub(crate) refused: Vec<PlanRefusal>,
    pub(crate) unwritten_modules: Vec<usize>,
}

struct Candidate {
    relative: String,
    contents: Vec<u8>,
    role: Role,
    owner: &'static str,
}

#[derive(Default)]
struct Planner {
    claims: Claims,
    refused: Vec<PlanRefusal>,
    unwritten_modules: Vec<usize>,
}

impl Planner {
    fn refusal_of(&self, relative: &str, key: &str) -> Option<PlanRefusal> {
        if !is_safe_relative(relative) || key.is_empty() {
            return Some(PlanRefusal::Unsafe(relative.to_string()));
        }
        self.claims
            .owner_of(key)
            .map(|owner| PlanRefusal::Collision {
                path: relative.to_string(),
                owner,
            })
    }

    fn admit(&mut self, dir: &Path, candidate: Candidate) -> Option<PlannedFile> {
        let key = claim_key(&candidate.relative);
        if let Some(refusal) = self.refusal_of(&candidate.relative, &key) {
            self.refused.push(refusal);
            if let Role::Module(index) = candidate.role {
                self.unwritten_modules.push(index);
            }
            return None;
        }
        self.claims.claim(key, candidate.owner);
        Some(PlannedFile {
            path: dir.join(&candidate.relative),
            relative: candidate.relative,
            contents: candidate.contents,
            role: candidate.role,
        })
    }

    fn free_output_name(&self, base: &str) -> Option<String> {
        let numbered = (FIRST_SUFFIX..=MAX_DEOBFUSCATED_SUFFIX)
            .map(|suffix| with_stem_suffix(base, &format!("-{suffix}")));
        std::iter::once(base.to_string())
            .chain(numbered)
            .find(|name| self.claims.owner_of(&claim_key(name)).is_none())
    }

    fn outputs(&mut self, dir: &Path, result: &Output, inputs: &[PathBuf]) -> Vec<PlannedFile> {
        let total = result.codes.len();
        let mut planned = Vec::with_capacity(total);
        for (index, code) in result.codes.iter().enumerate() {
            let base = deobfuscated_name(index, total, inputs.get(index).map(PathBuf::as_path));
            let Some(relative) = self.free_output_name(&base) else {
                self.refused.push(PlanRefusal::SuffixesExhausted(base));
                continue;
            };
            planned.extend(self.admit(
                dir,
                Candidate {
                    relative,
                    contents: code.as_bytes().to_vec(),
                    role: Role::Output,
                    owner: OWNER_OUTPUT,
                },
            ));
        }
        planned
    }

    fn listing(&mut self, dir: &Path, result: &Output) -> Option<PlannedFile> {
        result.bundle.as_ref()?;
        self.admit(
            dir,
            Candidate {
                relative: BUNDLE_JSON.to_string(),
                contents: Vec::new(),
                role: Role::Listing,
                owner: OWNER_LISTING,
            },
        )
    }

    fn wasm(&mut self, dir: &Path, result: &Output) -> Vec<PlannedFile> {
        let mut planned = Vec::with_capacity(result.wasm.len());
        for file in &result.wasm {
            planned.extend(self.admit(
                dir,
                Candidate {
                    relative: file.path.clone(),
                    contents: file.bytes.clone(),
                    role: Role::Wasm,
                    owner: OWNER_WASM,
                },
            ));
        }
        planned
    }

    fn modules(&mut self, dir: &Path, result: &Output) -> Vec<PlannedFile> {
        let Some(bundle) = result.bundle.as_ref() else {
            return Vec::new();
        };
        let mut planned = Vec::with_capacity(bundle.modules.len());
        for (index, module) in bundle.modules.iter().enumerate() {
            planned.extend(self.admit(
                dir,
                Candidate {
                    relative: module.path.clone(),
                    contents: module.code.clone().into_bytes(),
                    role: Role::Module(index),
                    owner: OWNER_MODULE,
                },
            ));
        }
        planned
    }
}

pub(crate) fn plan(dir: &Path, result: &Output, inputs: &[PathBuf]) -> Plan {
    if result.single_file {
        return Plan {
            files: single_file_plan(dir, result),
            ..Plan::default()
        };
    }
    plan_per_module(dir, result, inputs)
}

fn single_file_plan(dir: &Path, result: &Output) -> Vec<PlannedFile> {
    let mut planned = vec![PlannedFile {
        path: dir.join(DEOBFUSCATED),
        relative: DEOBFUSCATED.to_string(),
        contents: result.concatenated().into_owned().into_bytes(),
        role: Role::Output,
    }];
    if let Some(bundle) = result.bundle.as_ref() {
        planned.push(PlannedFile {
            path: dir.join(BUNDLE_JSON),
            relative: BUNDLE_JSON.to_string(),
            contents: bundle.bundle_json().into_bytes(),
            role: Role::Listing,
        });
    }
    planned.extend(result.wasm.iter().map(|file| PlannedFile {
        path: dir.join(&file.path),
        relative: file.path.clone(),
        contents: file.bytes.clone(),
        role: Role::Wasm,
    }));
    planned
}

fn listing_contents(result: &Output, unwritten_modules: &[usize]) -> Vec<u8> {
    result
        .bundle
        .as_ref()
        .map(|bundle| {
            bundle
                .listing_without(unwritten_modules)
                .json()
                .into_bytes()
        })
        .unwrap_or_default()
}

fn plan_per_module(dir: &Path, result: &Output, inputs: &[PathBuf]) -> Plan {
    let mut planner = Planner::default();
    let outputs = planner.outputs(dir, result, inputs);
    let listing = planner.listing(dir, result);
    let wasm = planner.wasm(dir, result);
    let modules = planner.modules(dir, result);
    let listing = listing.map(|file| PlannedFile {
        contents: listing_contents(result, &planner.unwritten_modules),
        ..file
    });
    Plan {
        files: outputs
            .into_iter()
            .chain(listing)
            .chain(modules)
            .chain(wasm)
            .collect(),
        refused: planner.refused,
        unwritten_modules: planner.unwritten_modules,
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
            memory_peak: 0,
            budget_hits: Vec::new(),
            stats: crate::RunStats::default(),
            snapshots: Vec::new(),
            decoded_strings: Vec::new(),
            statement_spans: None,
            #[cfg(feature = "refusal-spans")]
            refusal_spans: Vec::new(),
            #[cfg(feature = "refusal-spans")]
            refusal_sites: Vec::new(),
            wasm: Vec::new(),
            wasm_unresolved: Vec::new(),
            wasm_check: crate::WasmCheck::default(),
            wasm_unchecked: false,
            wasm_panicked: false,
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
                    span: None,
                })
                .collect(),
        }
    }

    fn planned_modules(paths: &[&str]) -> Plan {
        let result = output_with(
            vec!["deobfuscated code".to_string()],
            Some(bundle_with(paths)),
        );
        plan(Path::new("/out"), &result, &[])
    }

    fn relative_paths(plan: &Plan) -> Vec<&str> {
        plan.files
            .iter()
            .map(|file| file.relative.as_str())
            .collect()
    }

    fn collision(path: &str, owner: &'static str) -> PlanRefusal {
        PlanRefusal::Collision {
            path: path.to_string(),
            owner,
        }
    }

    #[test]
    fn a_collision_names_which_file_already_owns_the_path() {
        let against_output = planned_modules(&[DEOBFUSCATED]);
        assert_eq!(
            against_output.refused,
            vec![collision(DEOBFUSCATED, OWNER_OUTPUT)]
        );
        let against_module = planned_modules(&["shared.js", "shared.js"]);
        assert_eq!(
            against_module.refused,
            vec![collision("shared.js", OWNER_MODULE)]
        );
    }

    #[test]
    fn a_module_colliding_with_the_deobfuscated_output_is_refused() {
        let plan = planned_modules(&["deobfuscated.js"]);
        let deobfuscated = plan
            .files
            .iter()
            .find(|file| file.relative == DEOBFUSCATED)
            .expect("deobfuscated file planned");
        assert_eq!(deobfuscated.contents, b"deobfuscated code");
        assert!(plan.files.iter().all(|file| file.contents != b"module 0"));
        assert_eq!(plan.refused, vec![collision(DEOBFUSCATED, OWNER_OUTPUT)]);
    }

    #[test]
    fn a_module_at_the_path_of_the_bundle_listing_is_refused() {
        let plan = planned_modules(&[BUNDLE_JSON, "a.js"]);

        assert_eq!(plan.refused, vec![collision(BUNDLE_JSON, OWNER_LISTING)]);
        assert_eq!(
            relative_paths(&plan),
            vec![DEOBFUSCATED, BUNDLE_JSON, "a.js"]
        );
        assert_eq!(plan.unwritten_modules, vec![0]);
    }

    #[test]
    fn a_module_under_the_file_of_an_earlier_module_is_refused() {
        let plan = planned_modules(&["a.js", "a.js/b.js"]);

        assert_eq!(plan.refused, vec![collision("a.js/b.js", OWNER_ABOVE)]);
        assert_eq!(plan.unwritten_modules, vec![1]);
    }

    #[test]
    fn a_module_at_a_directory_earlier_modules_live_in_is_refused() {
        let plan = planned_modules(&["a.js/b.js", "a.js"]);

        assert_eq!(plan.refused, vec![collision("a.js", OWNER_INSIDE)]);
        assert_eq!(plan.unwritten_modules, vec![1]);
    }

    #[test]
    fn module_paths_differing_only_in_case_collide() {
        let plan = planned_modules(&["lib/Util.js", "LIB/util.js", "Deobfuscated.js"]);

        assert_eq!(
            plan.refused,
            vec![
                collision("LIB/util.js", OWNER_MODULE),
                collision("Deobfuscated.js", OWNER_OUTPUT)
            ]
        );
        assert_eq!(plan.unwritten_modules, vec![1, 2]);
    }

    #[test]
    fn a_module_colliding_with_a_wasm_file_is_refused() {
        let mut result = output_with(
            vec!["deobfuscated code".to_string()],
            Some(bundle_with(&["wasm/a.wasm", "wasm/b.js"])),
        );
        result.wasm = vec![crate::WasmFile {
            path: "wasm/a.wasm".to_string(),
            origin: crate::WasmOrigin::Embedded,
            url: None,
            byte_length: 0,
            imports: 0,
            exports: 0,
            renamed: 0,
            bytes: Vec::new(),
            summary: Default::default(),
        }];

        let plan = plan(Path::new("/out"), &result, &[]);

        assert_eq!(plan.refused, vec![collision("wasm/a.wasm", OWNER_WASM)]);
        assert_eq!(
            relative_paths(&plan),
            vec![DEOBFUSCATED, BUNDLE_JSON, "wasm/b.js", "wasm/a.wasm"]
        );
    }

    #[test]
    fn the_planned_listing_leaves_out_the_modules_the_plan_refused() {
        let plan = planned_modules(&["a.js", "../escape.js", "b.js"]);
        let listing = plan
            .files
            .iter()
            .find(|file| file.relative == BUNDLE_JSON)
            .expect("the listing is planned");

        let text = String::from_utf8(listing.contents.clone()).unwrap();

        assert!(text.contains("\"a.js\""), "{text}");
        assert!(text.contains("\"b.js\""), "{text}");
        assert!(!text.contains("escape"), "{text}");
    }

    #[test]
    fn a_deobfuscated_name_with_every_suffix_taken_is_refused_not_planned_twice() {
        let mut planner = Planner::default();
        planner
            .claims
            .claim("deobfuscated.a.js".to_string(), OWNER_OUTPUT);
        for suffix in FIRST_SUFFIX..=MAX_DEOBFUSCATED_SUFFIX {
            planner
                .claims
                .claim(format!("deobfuscated.a-{suffix}.js"), OWNER_OUTPUT);
        }

        assert_eq!(planner.free_output_name("deobfuscated.a.js"), None);
    }

    #[test]
    fn deobfuscated_names_are_uniquified_on_collision() {
        let result = output_with(vec!["first".to_string(), "second".to_string()], None);
        let inputs = vec![PathBuf::from("a/bundle.js"), PathBuf::from("b/Bundle.js")];

        let plan = plan(Path::new("/out"), &result, &inputs);

        assert!(plan.refused.is_empty());
        assert_eq!(
            relative_paths(&plan),
            vec!["deobfuscated.bundle.js", "deobfuscated.Bundle-2.js"]
        );
    }

    #[test]
    fn an_input_name_too_long_for_a_file_name_gets_a_numbered_output_name() {
        let result = output_with(vec!["first".to_string(), "second".to_string()], None);
        let long = format!("{}.js", "a".repeat(MAX_SEGMENT_BYTES - 3));
        let inputs = vec![PathBuf::from("a.js"), PathBuf::from(long)];

        let plan = plan(Path::new("/out"), &result, &inputs);

        assert_eq!(
            relative_paths(&plan),
            vec!["deobfuscated.a.js", "deobfuscated.input1.js"]
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
    fn a_path_holding_a_nul_is_unsafe() {
        assert!(!is_safe_relative("a\0b.js"));
    }

    #[test]
    fn a_segment_longer_than_a_file_name_may_be_is_unsafe() {
        let fits = "a".repeat(MAX_SEGMENT_BYTES);
        let too_long = "a".repeat(MAX_SEGMENT_BYTES + 1);
        assert!(is_safe_relative(&format!("sub/{fits}")));
        assert!(!is_safe_relative(&format!("sub/{too_long}")));
    }

    #[test]
    fn a_path_longer_than_the_whole_path_limit_is_unsafe() {
        assert!(is_safe_relative(&format!("{}a.js", "a/".repeat(500))));
        assert!(!is_safe_relative(&format!("{}a.js", "a/".repeat(520))));
    }

    #[test]
    fn safe_relative_path_is_accepted() {
        assert!(is_safe_relative("sub/index.js"));
    }
}
