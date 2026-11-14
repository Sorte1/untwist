pub mod exports;
pub mod importers;
pub mod library;
pub mod stem;

use oxc_ast::ast::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::model::{Bundle, BundleKind, Module};
use crate::path::{module_id_path, PosixPath, ENTRY_PATH, RESERVED_FILES};
use crate::rollup::build::CHUNK_PREFIX;
use exports::{candidates, default_export, main_export};
use importers::Importers;
use stem::is_numbered;

const NODE_MODULES: &str = "node_modules";
const RUN_PREFIX: &str = "module";
const SHARED_CHUNK: &str = "chunk_shared";
const FIRST_SUFFIX: usize = 2;
const MIN_CHUNK_STEM: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Guess {
    Numbered,
    Run,
    Chunk,
}

fn guess(kind: BundleKind, id: &str, module: &Module<'_>) -> Option<Guess> {
    if module.is_entry || module.path.as_str() != module_id_path(id, false) {
        return None;
    }
    match kind {
        BundleKind::Webpack | BundleKind::Browserify => (!id.is_empty()
            && id.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some(Guess::Numbered),
        BundleKind::Rollup if is_numbered(id, RUN_PREFIX) => Some(Guess::Run),
        BundleKind::Rollup if id.starts_with(CHUNK_PREFIX) && id != SHARED_CHUNK => {
            Some(Guess::Chunk)
        }
        _ => None,
    }
}

fn import_counts(bundle: &Bundle<'_>) -> FxHashMap<(String, String), usize> {
    let mut counts: FxHashMap<(String, String), usize> = FxHashMap::default();
    for module in bundle.modules.values() {
        for statement in &module.program().body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            let Some(source) = import.source.value.as_str() else {
                continue;
            };
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier {
                    let key = (source.to_string(), specifier.imported.name().to_string());
                    *counts.entry(key).or_default() += 1;
                }
            }
        }
    }
    counts
}

fn undeconflicted(name: &str) -> &str {
    match name.rsplit_once('$') {
        Some((base, suffix))
            if !base.is_empty()
                && !suffix.is_empty()
                && suffix.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    }
}

fn chunk_stem(id: &str) -> Option<&str> {
    let stem = id.strip_prefix(CHUNK_PREFIX)?;
    let plain = stem.trim_end_matches(|ch: char| ch.is_ascii_digit() || ch == '-');
    let source_name = plain.len() >= MIN_CHUNK_STEM
        && plain
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$');
    (source_name || is_numbered(stem, RUN_PREFIX)).then_some(stem)
}

fn wanted(
    kind: Guess,
    id: &str,
    module: &Module<'_>,
    (counts, importers): (&FxHashMap<(String, String), usize>, &Importers),
) -> Option<(String, bool)> {
    if let Some(path) = library::library_path(module.program()) {
        return Some((format!("{NODE_MODULES}/{path}"), true));
    }
    match kind {
        Guess::Chunk => chunk_stem(id).is_none().then(|| (String::new(), false)),
        Guess::Run => {
            let imported = |name: &str| {
                counts
                    .get(&(id.to_string(), name.to_string()))
                    .copied()
                    .unwrap_or_default()
            };
            let found = candidates(module.program(), true, imported);
            main_export(&found).map(|name| (format!("{}.js", undeconflicted(name)), false))
        }
        Guess::Numbered => {
            if let Some(path) = importers.path(id) {
                return Some((format!("{path}.js"), false));
            }
            let found = candidates(module.program(), false, |name| importers.read(id, name));
            default_export(&found)
                .or_else(|| importers.name(id))
                .or_else(|| main_export(&found))
                .map(|name| (format!("{name}.js"), false))
        }
    }
}

#[derive(Default)]
struct Taken {
    paths: FxHashSet<String>,
    next_number: usize,
    next_suffix: FxHashMap<String, usize>,
}

impl Taken {
    fn insert(&mut self, path: &str) -> bool {
        self.paths.insert(path.to_lowercase())
    }

    fn numbered(&mut self) -> Option<PosixPath> {
        loop {
            self.next_number += 1;
            let path = format!("{CHUNK_PREFIX}{RUN_PREFIX}{}.js", self.next_number);
            if !self.paths.contains(&path) {
                return PosixPath::parse(&path).map(|path| self.claim(path));
            }
        }
    }

    fn claim(&mut self, path: PosixPath) -> PosixPath {
        if self.insert(path.as_str()) {
            return path;
        }
        let key = path.as_str().to_lowercase();
        let mut suffix = self.next_suffix.get(&key).copied().unwrap_or(FIRST_SUFFIX);
        loop {
            let candidate = path.with_stem_suffix(&format!("-{suffix}"));
            suffix += 1;
            if self.insert(candidate.as_str()) {
                self.next_suffix.insert(key, suffix);
                return candidate;
            }
        }
    }
}

pub fn name_modules(bundle: &mut Bundle<'_>) {
    let kind = bundle.kind;
    let guesses: Vec<Option<Guess>> = bundle
        .modules
        .iter()
        .map(|(id, module)| guess(kind, id, module))
        .collect();
    if guesses.iter().all(Option::is_none) {
        return;
    }
    let (counts, importers) = match kind {
        BundleKind::Rollup => (import_counts(bundle), Importers::default()),
        _ => (FxHashMap::default(), importers::scan(bundle)),
    };
    let planned: Vec<Option<(String, bool)>> = bundle
        .modules
        .iter()
        .zip(&guesses)
        .map(|((id, module), guessed)| wanted((*guessed)?, id, module, (&counts, &importers)))
        .collect();
    if planned.iter().all(Option::is_none) {
        return;
    }
    let mut taken = Taken::default();
    for name in RESERVED_FILES.iter().chain([&ENTRY_PATH]) {
        taken.insert(name);
    }
    for (module, plan) in bundle.modules.values().zip(&planned) {
        if plan.is_none() {
            taken.insert(module.path.as_str());
        }
    }
    for (module, plan) in bundle.modules.values_mut().zip(planned) {
        let Some((wanted, library)) = plan else {
            continue;
        };
        let path = match wanted.is_empty() {
            true => taken.numbered(),
            false => PosixPath::parse(&wanted).map(|path| taken.claim(path)),
        };
        let Some(path) = path else {
            continue;
        };
        module.path = path;
        module.named_path = library;
    }
}
