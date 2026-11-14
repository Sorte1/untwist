use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use crate::maps::{link, MapLink, Target};
use crate::pipeline::Output;
use crate::write::{BUNDLE_JSON, DEOBFUSCATED};
use untwist_core::source_map::OwnedSourceMap;
use untwist_unpack::path::{is_unsafe_path, MAX_PATH_BYTES, MAX_SEGMENT_BYTES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Output,
    Listing,
    Module(usize),
    Wasm,
    SourceMap,
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

fn absolute_parts(input: &Path) -> Option<Vec<String>> {
    let absolute = std::path::absolute(input).ok()?;
    let mut parts: Vec<String> = Vec::new();
    for component in absolute.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => parts.clear(),
        }
    }
    Some(parts)
}

fn shared_directory_len(paths: &[Vec<String>]) -> usize {
    let Some((first, rest)) = paths.split_first() else {
        return 0;
    };
    let directory = first.len().saturating_sub(1);
    (0..directory)
        .take_while(|&at| {
            rest.iter()
                .all(|path| at + 1 < path.len() && path[at] == first[at])
        })
        .count()
}

pub fn input_names(inputs: &[PathBuf]) -> Vec<String> {
    if inputs.len() < 2 {
        return Vec::new();
    }
    let Some(paths) = inputs
        .iter()
        .map(|input| absolute_parts(input))
        .collect::<Option<Vec<_>>>()
    else {
        return Vec::new();
    };
    let shared = shared_directory_len(&paths);
    let names: Vec<String> = paths.iter().map(|path| path[shared..].join("/")).collect();
    match names.iter().all(|name| is_safe_relative(name)) {
        true => names,
        false => Vec::new(),
    }
}

fn typed_output(result: &Output, name: String, index: Option<usize>) -> String {
    let Some(report) = result.types.as_ref().filter(|report| report.typescript) else {
        return name;
    };
    let jsx = match index {
        Some(index) => report.jsx_outputs.get(index).copied().unwrap_or(false),
        None => report.jsx_outputs.iter().any(|jsx| *jsx),
    };
    crate::types::typed_path(&name, jsx)
}

fn mirrored_name(result: &Output, index: usize) -> Option<&str> {
    let names = &result.bundle.as_ref()?.mirrored_inputs;
    (names.len() == result.codes.len())
        .then(|| names.get(index))
        .flatten()
        .map(String::as_str)
}

const OWNER_OUTPUT: &str = "the deobfuscated output is already planned at that path";
const OWNER_MODULE: &str = "an earlier module is already planned at that path";
const OWNER_LISTING: &str = "the bundle listing is written at that path";
const OWNER_WASM: &str = "a wasm module is already planned at that path";
const OWNER_MAP: &str = "a source map is already planned at that path";
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

struct Linking<'l> {
    link: MapLink,
    inputs: &'l [PathBuf],
    total: usize,
}

impl Linking<'_> {
    fn of<'o>(result: &Output, inputs: &'o [PathBuf]) -> Option<Linking<'o>> {
        Some(Linking {
            link: result.source_map_link?,
            inputs,
            total: result.codes.len(),
        })
    }

    fn linked(
        &self,
        relative: &str,
        code: &str,
        map: &OwnedSourceMap,
    ) -> (Vec<u8>, Option<Candidate>) {
        let target = Target {
            relative,
            inputs: self.inputs,
            total: self.total,
        };
        let linked = link(code, map, self.link, &target);
        let file = linked.map.map(|(relative, json)| Candidate {
            relative,
            contents: json.into_bytes(),
            role: Role::SourceMap,
            owner: OWNER_MAP,
        });
        (linked.code.into_bytes(), file)
    }
}

#[derive(Default)]
struct Planner<'l> {
    claims: Claims,
    refused: Vec<PlanRefusal>,
    unwritten_modules: Vec<usize>,
    linking: Option<Linking<'l>>,
}

impl Planner<'_> {
    fn admit_mapped(
        &mut self,
        dir: &Path,
        candidate: Candidate,
        map: Option<&OwnedSourceMap>,
    ) -> Vec<PlannedFile> {
        let refused = self
            .refusal_of(&candidate.relative, &claim_key(&candidate.relative))
            .is_some();
        let (Some(linking), Some(map), false) = (self.linking.as_ref(), map, refused) else {
            return self.admit(dir, candidate).into_iter().collect();
        };
        let code = String::from_utf8_lossy(&candidate.contents).into_owned();
        let (contents, map_file) = linking.linked(&candidate.relative, &code, map);
        let mut planned: Vec<PlannedFile> = self
            .admit(
                dir,
                Candidate {
                    contents,
                    ..candidate
                },
            )
            .into_iter()
            .collect();
        if let Some(map_file) = map_file {
            planned.extend(self.admit(dir, map_file));
        }
        planned
    }

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
        let unsafe_wasm = candidate.role == Role::Wasm && is_unsafe_path(&candidate.relative);
        let refusal = match unsafe_wasm {
            true => Some(PlanRefusal::Unsafe(candidate.relative.clone())),
            false => self.refusal_of(&candidate.relative, &key),
        };
        if let Some(refusal) = refusal {
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

    fn output_name(&mut self, index: usize, result: &Output, inputs: &[PathBuf]) -> Option<String> {
        if let Some(name) = mirrored_name(result, index) {
            return Some(typed_output(result, name.to_string(), Some(index)));
        }
        let total = result.codes.len();
        let base = deobfuscated_name(index, total, inputs.get(index).map(PathBuf::as_path));
        let base = typed_output(result, base, Some(index));
        let name = self.free_output_name(&base);
        if name.is_none() {
            self.refused.push(PlanRefusal::SuffixesExhausted(base));
        }
        name
    }

    fn outputs(&mut self, dir: &Path, result: &Output, inputs: &[PathBuf]) -> Vec<PlannedFile> {
        let mut planned = Vec::with_capacity(result.codes.len());
        for (index, code) in result.codes.iter().enumerate() {
            let Some(relative) = self.output_name(index, result, inputs) else {
                continue;
            };
            let map = result.source_maps.get(index).and_then(Option::as_ref);
            planned.extend(self.admit_mapped(
                dir,
                Candidate {
                    relative,
                    contents: code.as_bytes().to_vec(),
                    role: Role::Output,
                    owner: OWNER_OUTPUT,
                },
                map,
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
            planned.extend(self.admit_mapped(
                dir,
                Candidate {
                    relative: module.path.clone(),
                    contents: module.code.clone().into_bytes(),
                    role: Role::Module(index),
                    owner: OWNER_MODULE,
                },
                module.map.as_ref(),
            ));
        }
        planned
    }
}

pub(crate) fn plan(dir: &Path, result: &Output, inputs: &[PathBuf]) -> Plan {
    if result.single_file {
        return single_file_plan(dir, result, inputs);
    }
    plan_per_module(dir, result, inputs)
}

fn single_file_plan(dir: &Path, result: &Output, inputs: &[PathBuf]) -> Plan {
    let concatenated = result.concatenated();
    let map = match (result.bundle.as_ref(), result.source_maps.as_slice()) {
        (None, [Some(map)]) => Some(map),
        _ => None,
    };
    let linked = Linking::of(result, inputs)
        .zip(map)
        .map(|(linking, map)| linking.linked(DEOBFUSCATED, &concatenated, map));
    let (contents, map_file) = match linked {
        Some(linked) => linked,
        None => (concatenated.into_owned().into_bytes(), None),
    };
    let mut planner = Planner::default();
    let output = Candidate {
        relative: typed_output(result, DEOBFUSCATED.to_string(), None),
        contents,
        role: Role::Output,
        owner: OWNER_OUTPUT,
    };
    let mut files: Vec<PlannedFile> = planner.admit(dir, output).into_iter().collect();
    files.extend(map_file.and_then(|file| planner.admit(dir, file)));
    if let Some(bundle) = result.bundle.as_ref() {
        let listing = Candidate {
            relative: BUNDLE_JSON.to_string(),
            contents: bundle.bundle_json().into_bytes(),
            role: Role::Listing,
            owner: OWNER_LISTING,
        };
        files.extend(planner.admit(dir, listing));
    }
    files.extend(planner.wasm(dir, result));
    Plan {
        files,
        refused: planner.refused,
        unwritten_modules: planner.unwritten_modules,
    }
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
    let mut planner = Planner {
        linking: Linking::of(result, inputs),
        ..Planner::default()
    };
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
            unpack_budget: None,
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
            source_maps: Vec::new(),
            source_map_link: None,
            types: None,
        }
    }

    fn bundle_with(paths: &[&str]) -> BundleOutput {
        BundleOutput {
            kind: "webpack".to_string(),
            entry_id: None,
            chunk_ids: Vec::new(),
            mirrored_inputs: Vec::new(),
            modules: paths
                .iter()
                .enumerate()
                .map(|(index, path)| ModuleOutput {
                    id: index.to_string(),
                    path: (*path).to_string(),
                    is_entry: false,
                    code: format!("module {index}"),
                    span: None,
                    map: None,
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

    fn wasm_at(path: &str) -> crate::WasmFile {
        crate::WasmFile {
            path: path.to_string(),
            origin: crate::wasm::WasmOrigin::Supplied,
            url: None,
            byte_length: 1,
            imports: 0,
            exports: 0,
            renamed: 0,
            bytes: path.as_bytes().to_vec(),
            summary: Default::default(),
        }
    }

    #[test]
    fn a_single_file_plan_refuses_colliding_and_unsafe_wasm_names() {
        let mut result = output_with(vec!["code".to_string()], None);
        result.single_file = true;
        result.wasm = vec![
            wasm_at("wasm/a.wasm"),
            wasm_at("wasm/A.wasm"),
            wasm_at("wasm/../../x.wasm"),
            wasm_at("wasm/nul.wasm"),
        ];
        let plan = plan(Path::new("/out"), &result, &[]);
        assert_eq!(relative_paths(&plan), [DEOBFUSCATED, "wasm/a.wasm"]);
        assert_eq!(plan.refused.len(), 3, "{:?}", plan.refused);
        assert!(plan.refused.contains(&collision("wasm/A.wasm", OWNER_WASM)));
    }

    #[test]
    fn a_per_module_plan_refuses_a_wasm_name_the_module_checks_refuse() {
        let mut result = output_with(vec!["code".to_string()], None);
        result.wasm = vec![wasm_at("wasm/aux.wasm"), wasm_at("wasm/b~1.wasm")];
        let plan = plan(Path::new("/out"), &result, &[]);
        assert_eq!(relative_paths(&plan), [DEOBFUSCATED]);
        assert_eq!(plan.refused.len(), 2, "{:?}", plan.refused);
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

    fn mirrored(names: &[&str], modules: &[&str]) -> Plan {
        let mut bundle = bundle_with(modules);
        bundle.mirrored_inputs = names.iter().map(|name| (*name).to_string()).collect();
        let codes = names.iter().map(|name| format!("input {name}")).collect();
        plan(Path::new("/out"), &output_with(codes, Some(bundle)), &[])
    }

    #[test]
    fn a_partial_bundle_writes_every_input_under_its_own_name() {
        let plan = mirrored(&["index.js", "chunks/lazy.js"], &["4NXGQ.js"]);

        assert!(plan.refused.is_empty(), "{:?}", plan.refused);
        assert_eq!(
            relative_paths(&plan),
            ["index.js", "chunks/lazy.js", BUNDLE_JSON, "4NXGQ.js"]
        );
        let lazy = plan
            .files
            .iter()
            .find(|file| file.relative == "chunks/lazy.js")
            .unwrap();
        assert_eq!(lazy.contents, b"input chunks/lazy.js");
    }

    #[test]
    fn a_mirrored_input_name_that_is_taken_is_refused_not_renamed() {
        let plan = mirrored(&["a.js", "A.js"], &["b.js"]);

        assert_eq!(plan.refused, vec![collision("A.js", OWNER_OUTPUT)]);
        assert_eq!(relative_paths(&plan), ["a.js", BUNDLE_JSON, "b.js"]);
    }

    #[test]
    fn mirrored_names_are_used_only_when_they_name_every_output() {
        let mut bundle = bundle_with(&["m.js"]);
        bundle.mirrored_inputs = vec!["a.js".to_string()];
        let result = output_with(vec!["one".to_string(), "two".to_string()], Some(bundle));
        let inputs = vec![PathBuf::from("a.js"), PathBuf::from("b.js")];

        let plan = plan(Path::new("/out"), &result, &inputs);

        assert_eq!(
            relative_paths(&plan),
            [
                "deobfuscated.a.js",
                "deobfuscated.b.js",
                BUNDLE_JSON,
                "m.js"
            ]
        );
    }

    fn named(inputs: &[&str]) -> Vec<String> {
        input_names(&inputs.iter().map(PathBuf::from).collect::<Vec<_>>())
    }

    #[test]
    fn input_names_are_relative_to_the_directory_the_inputs_share() {
        assert_eq!(
            named(&["/w/dist/index.js", "/w/dist/chunks/a.js"]),
            ["index.js", "chunks/a.js"]
        );
        assert_eq!(named(&["/w/a/x.js", "/w/b/x.js"]), ["a/x.js", "b/x.js"]);
        assert_eq!(
            named(&["/w/dist/../dist/index.js", "/w/dist/./lazy.js"]),
            ["index.js", "lazy.js"]
        );
    }

    #[test]
    fn a_single_input_has_no_input_names() {
        assert!(named(&["/w/index.js"]).is_empty());
        assert!(named(&[]).is_empty());
    }

    #[test]
    fn an_input_that_is_a_parent_of_another_keeps_its_file_name() {
        assert_eq!(named(&["/w/a", "/w/a/b.js"]), ["a", "a/b.js"]);
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

    fn mapped_modules(paths: &[&str]) -> Plan {
        let mut bundle = bundle_with(paths);
        for module in &mut bundle.modules {
            module.map = Some(untwist_core::source_map::identity(&module.code));
        }
        let mut result = output_with(vec!["deobfuscated code".to_string()], Some(bundle));
        result.source_map_link = Some(MapLink::File);
        plan(Path::new("/out"), &result, &[])
    }

    #[test]
    fn a_written_module_gets_its_map_beside_it() {
        let plan = mapped_modules(&["sub/a.js"]);
        let map = plan
            .files
            .iter()
            .find(|file| file.role == Role::SourceMap)
            .expect("a planned map");
        assert_eq!(map.relative, "sub/a.js.map");
        let module = plan
            .files
            .iter()
            .find(|file| file.relative == "sub/a.js")
            .expect("the module");
        assert!(
            String::from_utf8_lossy(&module.contents).ends_with("//# sourceMappingURL=a.js.map")
        );
    }

    #[test]
    fn a_refused_module_name_gets_no_map_and_no_link() {
        let plan = mapped_modules(&["../escape.js", "a\\b.js"]);
        assert!(plan.files.iter().all(|file| file.role != Role::SourceMap));
        assert_eq!(plan.unwritten_modules, vec![0, 1]);
    }
}
