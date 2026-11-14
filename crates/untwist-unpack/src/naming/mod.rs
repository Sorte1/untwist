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

struct Taken(FxHashSet<String>);

impl Taken {
    fn numbered(&mut self) -> Option<PosixPath> {
        (1usize..)
            .map(|n| format!("{CHUNK_PREFIX}{RUN_PREFIX}{n}.js"))
            .find(|path| !self.0.contains(path))
            .and_then(|path| PosixPath::parse(&path))
            .map(|path| self.claim(path))
    }

    fn claim(&mut self, path: PosixPath) -> PosixPath {
        if self.0.insert(path.as_str().to_lowercase()) {
            return path;
        }
        let mut suffix = FIRST_SUFFIX;
        loop {
            let candidate = path.with_stem_suffix(&format!("-{suffix}"));
            if self.0.insert(candidate.as_str().to_lowercase()) {
                return candidate;
            }
            suffix += 1;
        }
    }
}

pub fn name_modules(bundle: &mut Bundle<'_>) {
    let kind = bundle.kind;
    let (counts, importers) = match kind {
        BundleKind::Rollup => (import_counts(bundle), Importers::default()),
        _ => (FxHashMap::default(), importers::scan(bundle)),
    };
    let planned: Vec<Option<(String, bool)>> = bundle
        .modules
        .iter()
        .map(|(id, module)| {
            let guessed = guess(kind, id, module)?;
            wanted(guessed, id, module, (&counts, &importers))
        })
        .collect();
    if planned.iter().all(Option::is_none) {
        return;
    }
    let mut taken = Taken(
        RESERVED_FILES
            .iter()
            .chain([&ENTRY_PATH])
            .map(|name| name.to_lowercase())
            .collect(),
    );
    for (module, plan) in bundle.modules.values().zip(&planned) {
        if plan.is_none() {
            taken.0.insert(module.path.as_str().to_lowercase());
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
