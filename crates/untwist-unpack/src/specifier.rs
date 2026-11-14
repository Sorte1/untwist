use rustc_hash::{FxHashMap, FxHashSet};

use crate::model::Bundle;
use crate::path::{dirname_posix, normalize_posix, relative_posix};

const NODE_MODULES: &str = "node_modules/";
const CORE_SCHEME: &str = "node:";
const PACKAGE_IMPORT: char = '#';
const RESOLVED_SUFFIXES: [&str; 4] = ["", ".js", ".json", ".node"];

#[derive(Clone, Debug, Default)]
pub struct ModulePaths {
    by_id: FxHashMap<String, String>,
    taken: FxHashSet<String>,
    named: FxHashSet<String>,
    reference: bool,
}

fn with_ancestors(path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(path), |path| Some(dirname_posix(path)))
        .take_while(|path| !path.is_empty())
}

fn may_name_a_core_module(tail: &str) -> bool {
    tail.starts_with(CORE_SCHEME)
        || tail
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '/')
}

impl ModulePaths {
    pub fn of(bundle: &Bundle<'_>) -> Self {
        let paths = Self::from_pairs(
            bundle
                .modules
                .iter()
                .map(|(id, module)| (id.as_str(), module.path.as_str())),
        );
        let named = bundle
            .modules
            .values()
            .filter(|module| module.named_path)
            .map(|module| normalize_posix(module.path.as_str()))
            .collect();
        Self { named, ..paths }
    }

    pub fn from_pairs<'p>(pairs: impl IntoIterator<Item = (&'p str, &'p str)>) -> Self {
        let by_id: FxHashMap<String, String> = pairs
            .into_iter()
            .map(|(id, path)| (id.to_string(), normalize_posix(path)))
            .collect();
        let taken = by_id
            .values()
            .flat_map(|path| with_ancestors(path))
            .map(str::to_string)
            .collect();
        Self {
            by_id,
            taken,
            named: FxHashSet::default(),
            reference: false,
        }
    }

    pub fn matching_reference(self, reference: bool) -> Self {
        Self { reference, ..self }
    }

    pub fn get(&self, id: &str) -> Option<&str> {
        self.by_id.get(id).map(String::as_str)
    }

    pub fn contains_key(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn specifier(&self, from: &str, to: &str) -> String {
        let to = normalize_posix(to);
        match to.strip_prefix(NODE_MODULES) {
            Some(_) if self.named.contains(&to) => self.relative(from, &to),
            Some(tail) if self.reference || self.resolves_bare(from, &to, tail) => tail.to_string(),
            _ => self.relative(from, &to),
        }
    }

    fn resolves_bare(&self, from: &str, to: &str, tail: &str) -> bool {
        self.taken.contains(to)
            && !tail.starts_with(PACKAGE_IMPORT)
            && !may_name_a_core_module(tail)
            && !self.captured(from, tail)
    }

    fn captured(&self, from: &str, tail: &str) -> bool {
        with_ancestors(dirname_posix(from)).any(|directory| {
            RESOLVED_SUFFIXES.iter().any(|suffix| {
                self.taken
                    .contains(&format!("{directory}/{NODE_MODULES}{tail}{suffix}"))
            })
        })
    }

    fn relative(&self, from: &str, to: &str) -> String {
        let relative = relative_posix(dirname_posix(from), to);
        let climbs = relative == ".." || relative.starts_with("../");
        if climbs || (self.reference && relative.starts_with('.')) {
            relative
        } else {
            format!("./{relative}")
        }
    }
}
