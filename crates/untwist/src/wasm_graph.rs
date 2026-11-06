use std::collections::HashMap;

use untwist_core::pass::{Note, NoteKind};
use untwist_unpack::wasm::glue::chosen_name;
use untwist_unpack::wasm::{
    self, CopiedRenames, GlueImport, ImportValue, ModuleFacts, Span, WasmSummary,
};

use crate::pipeline::{admits_reparse, Output};

const HASHED_SUFFIX: &str = ".module.wasm";
const MISSING_SHOWN: usize = 3;

type Renames = HashMap<usize, Vec<(String, String)>>;

struct Graph<'o> {
    paths: Vec<&'o str>,
    ids: Vec<&'o str>,
    facts: Vec<ModuleFacts>,
    runtime: Vec<ModuleFacts>,
}

fn normalized(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

fn joined(host: &str, spec: &str) -> String {
    let directory = host.rsplit_once('/').map_or("", |(directory, _)| directory);
    normalized(&format!("{directory}/{spec}"))
}

fn facts_of(text: &str) -> ModuleFacts {
    match admits_reparse(text) {
        true => wasm::module_facts(text).unwrap_or_default(),
        false => ModuleFacts::default(),
    }
}

impl Graph<'_> {
    fn resolve(&self, host: usize, spec: &str) -> Option<usize> {
        let path = joined(self.paths[host], spec);
        self.paths
            .iter()
            .position(|candidate| *candidate == path)
            .or_else(|| self.ids.iter().position(|id| *id == spec))
    }

    fn all_facts(&self) -> impl Iterator<Item = &ModuleFacts> {
        self.facts.iter().chain(&self.runtime)
    }

    fn hash_of(&self, id: &str) -> Option<&str> {
        self.all_facts()
            .flat_map(|facts| &facts.id_hashes)
            .find(|(known, _)| known == id)
            .map(|(_, hash)| hash.as_str())
    }

    fn sync_imports(&self, id: &str) -> Vec<GlueImport> {
        self.all_facts()
            .flat_map(|facts| &facts.import_objects)
            .find(|(known, _)| known == id)
            .map(|(_, imports)| imports.clone())
            .unwrap_or_default()
    }

    fn wrapper(&self, module: usize) -> Option<(String, Vec<GlueImport>)> {
        let facts = &self.facts[module];
        if let Some(wrapper) = &facts.async_wrapper {
            return Some(wrapper.clone());
        }
        if !facts.sync_wrapper {
            return None;
        }
        let id = self.ids[module];
        Some((self.hash_of(id)?.to_string(), self.sync_imports(id)))
    }

    fn export_local(&self, module: usize, export: &str) -> Option<&str> {
        self.facts[module]
            .exports
            .iter()
            .find(|(name, _)| name == export)
            .map(|(_, local)| local.as_str())
    }

    fn import_target(&self, wrapper: usize, value: &ImportValue) -> Option<(usize, String)> {
        match value {
            ImportValue::Required { local, export } => {
                let spec = self.facts[wrapper]
                    .requires
                    .iter()
                    .find(|(name, _)| name == local)
                    .map(|(_, spec)| spec)?;
                let target = self.resolve(wrapper, spec)?;
                Some((target, self.export_local(target, export)?.to_string()))
            }
            ImportValue::Cached { id, export } => {
                let target = self.ids.iter().position(|known| known == id)?;
                Some((target, self.export_local(target, export)?.to_string()))
            }
            ImportValue::Local(name) => Some((wrapper, name.clone())),
            ImportValue::Opaque => None,
        }
    }

    fn plan(
        &self,
        wrapper: usize,
        imports: &[GlueImport],
        summary: &WasmSummary,
    ) -> Result<Renames, Vec<String>> {
        let mut renames: Renames = HashMap::new();
        let mut missing = Vec::new();
        for import in imports {
            let Some(found) = summary.import(&import.module, &import.field) else {
                missing.push(format!("import {}.{}", import.module, import.field));
                continue;
            };
            let name = chosen_name(&found.name, found.function_name.as_ref());
            if let (Some(name), Some((target, local))) =
                (name, self.import_target(wrapper, &import.value))
            {
                renames.entry(target).or_default().push((local, name));
            }
        }
        for (consumer, facts) in self.facts.iter().enumerate() {
            for (holder, spec) in &facts.requires {
                if self.resolve(consumer, spec) != Some(wrapper) {
                    continue;
                }
                for (_, read) in facts.reads.iter().filter(|(by, _)| by == holder) {
                    if summary.export(read).is_none() {
                        missing.push(format!("export {read}"));
                    }
                }
                for (binding, _, read) in facts.bindings.iter().filter(|(_, by, _)| by == holder) {
                    let name = summary.export(read).and_then(|export| {
                        chosen_name(&export.name, export.function_name.as_ref())
                    });
                    if let Some(name) = name {
                        renames
                            .entry(consumer)
                            .or_default()
                            .push((binding.clone(), name));
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        match missing.is_empty() {
            true => Ok(renames),
            false => Err(missing),
        }
    }
}

impl Output {
    pub(crate) fn name_bundle_glue(
        &mut self,
        resolved: &HashMap<String, usize>,
        notes: &mut Vec<Note>,
    ) -> Vec<(usize, Option<CopiedRenames>)> {
        let Some(bundle) = &self.bundle else {
            return Vec::new();
        };
        let hashed = |code: &String| code.contains(HASHED_SUFFIX);
        if resolved.is_empty()
            || !(bundle.modules.iter().any(|module| hashed(&module.code))
                || self.codes.iter().any(hashed))
        {
            return Vec::new();
        }
        let graph = Graph {
            paths: bundle
                .modules
                .iter()
                .map(|module| module.path.as_str())
                .collect(),
            ids: bundle
                .modules
                .iter()
                .map(|module| module.id.as_str())
                .collect(),
            facts: bundle
                .modules
                .iter()
                .map(|module| facts_of(&module.code))
                .collect(),
            runtime: self
                .codes
                .iter()
                .filter(|code| code.contains(HASHED_SUFFIX))
                .map(|code| facts_of(code))
                .collect(),
        };
        let mut all: Vec<(usize, Renames)> = Vec::new();
        for module in 0..graph.facts.len() {
            let Some((hash, imports)) = graph.wrapper(module) else {
                continue;
            };
            let name = format!("{hash}{HASHED_SUFFIX}");
            let Some(&file) = resolved.get(&name) else {
                continue;
            };
            match graph.plan(module, &imports, &self.wasm[file].summary) {
                Ok(renames) => all.push((file, renames)),
                Err(missing) => notes.push(mismatch(
                    &self.wasm[file].path,
                    graph.paths[module],
                    &missing,
                )),
            }
        }
        self.apply_bundle_renames(all)
    }

    fn apply_bundle_renames(
        &mut self,
        all: Vec<(usize, Renames)>,
    ) -> Vec<(usize, Option<CopiedRenames>)> {
        let mut copies = Vec::with_capacity(all.len());
        for (file, renames) in all {
            let mut targets: Vec<(usize, Vec<(String, String)>)> = renames.into_iter().collect();
            targets.sort_by_key(|(module, _)| *module);
            let mut copy = Some(CopiedRenames::default());
            for (module, wanted) in targets {
                let Some(bundle) = self.bundle.as_mut() else {
                    break;
                };
                let module = &mut bundle.modules[module];
                if !admits_reparse(&module.code) {
                    continue;
                }
                let Some((renamed, applied)) = wasm::rename_names(&module.code, &wanted) else {
                    continue;
                };
                module.code = renamed;
                self.wasm[file].renamed += applied.len();
                if applied.is_empty() {
                    continue;
                }
                copy = match (copy, module.span) {
                    (Some(mut copy), Some((start, end))) => {
                        copy.functions.push((Span::new(start, end), applied));
                        Some(copy)
                    }
                    _ => None,
                };
            }
            if copy.as_ref().is_none_or(|copy| !copy.functions.is_empty()) {
                copies.push((file, copy));
            }
        }
        copies
    }
}

fn mismatch(file: &str, wrapper: &str, missing: &[String]) -> Note {
    let shown: Vec<&str> = missing
        .iter()
        .take(MISSING_SHOWN)
        .map(String::as_str)
        .collect();
    Note {
        pass: "wasm",
        kind: NoteKind::Refusal,
        message: format!(
            "{file} does not match the glue around {wrapper}: {} glue name(s) missing from the module, e.g. {}; no names were taken from it",
            missing.len(),
            shown.join(", ")
        ),
        span: None,
    }
}
