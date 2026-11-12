use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Note, NoteKind};

use crate::model::{fallback_path, Bundle};
use crate::path::{PosixPath, ENTRY_PATH, MAX_SEGMENT_BYTES, RESERVED_DIRECTORIES, RESERVED_FILES};

const PASS_NAME: &str = "unpack";
const FIRST_SUFFIX: usize = 2;
const LONGEST_SUFFIX_BYTES: usize = 21;

fn note(message: String) -> Note {
    Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message,
        span: None,
    }
}

fn folded(path: &str) -> String {
    path.to_lowercase()
}

fn parents(key: &str) -> impl Iterator<Item = &str> {
    key.match_indices('/').map(|(index, _)| &key[..index])
}

fn has_suffix_room(path: &PosixPath) -> bool {
    let name = path.as_str().rsplit('/').next().unwrap_or_default();
    name.len() + LONGEST_SUFFIX_BYTES <= MAX_SEGMENT_BYTES
}

#[derive(Default)]
struct Taken {
    files: FxHashSet<String>,
    directories: FxHashSet<String>,
    next_suffix: FxHashMap<String, usize>,
}

impl Taken {
    fn reserved(has_entry: bool, inputs: &[String]) -> Self {
        let mut taken = Self::default();
        taken
            .files
            .extend(RESERVED_FILES.iter().map(|name| folded(name)));
        taken
            .directories
            .extend(RESERVED_DIRECTORIES.iter().map(|name| folded(name)));
        for input in inputs {
            taken.claim(folded(input));
        }
        if has_entry {
            taken.files.insert(folded(ENTRY_PATH));
        }
        taken
    }

    fn is_under_a_file(&self, key: &str) -> bool {
        parents(key).any(|parent| self.files.contains(parent))
    }

    fn is_free(&self, key: &str) -> bool {
        !self.files.contains(key) && !self.directories.contains(key) && !self.is_under_a_file(key)
    }

    fn claim(&mut self, key: String) {
        for parent in parents(&key) {
            if !self.directories.contains(parent) {
                self.directories.insert(parent.to_string());
            }
        }
        self.files.insert(key);
    }

    fn next_free(&mut self, base: &PosixPath) -> PosixPath {
        let base_key = folded(base.as_str());
        let mut suffix = self
            .next_suffix
            .get(&base_key)
            .copied()
            .unwrap_or(FIRST_SUFFIX);
        loop {
            let candidate = base.with_stem_suffix(&format!("-{suffix}"));
            suffix += 1;
            if self.is_free(&folded(candidate.as_str())) {
                self.next_suffix.insert(base_key, suffix);
                return candidate;
            }
        }
    }

    fn unique(&mut self, id: &str, base: PosixPath, notes: &mut Vec<Note>) -> PosixPath {
        let key = folded(base.as_str());
        if self.is_free(&key) {
            self.claim(key);
            return base;
        }
        let flat = self.is_under_a_file(&key) || !has_suffix_room(&base);
        let stem = if flat {
            fallback_path(id)
        } else {
            base.clone()
        };
        let path = if self.is_free(&folded(stem.as_str())) {
            stem
        } else {
            self.next_free(&stem)
        };
        notes.push(note(crate::notes::path_collision(
            id,
            path.as_str(),
            base.as_str(),
        )));
        self.claim(folded(path.as_str()));
        path
    }
}

pub fn assign_paths(bundle: &mut Bundle<'_>) -> Vec<Note> {
    assign_paths_around(bundle, &[])
}

fn is_input_name(inputs: &[String], path: &str) -> bool {
    inputs.iter().any(|input| folded(input) == folded(path))
}

pub fn assign_paths_around(bundle: &mut Bundle<'_>, inputs: &[String]) -> Vec<Note> {
    let mut notes = Vec::new();
    let entry_is_input = is_input_name(inputs, ENTRY_PATH);
    let mut entry_reserved =
        !entry_is_input && bundle.modules.values().any(|module| module.is_entry);
    let mut taken = Taken::reserved(entry_reserved, inputs);
    for module in bundle.modules.values_mut() {
        if let Some(source) = &module.unsafe_path_source {
            notes.push(note(crate::notes::unsafe_path_replaced(
                &module.id,
                source,
                module.path.as_str(),
            )));
        }
        let parsed = match entry_is_input && module.is_entry && module.path.as_str() == ENTRY_PATH {
            true => PosixPath::from_module_id(&module.id, false),
            false => PosixPath::parse(module.path.as_str()),
        };
        let base = match parsed {
            Some(path) => path,
            None => {
                notes.push(note(crate::notes::unsafe_output_path_replaced(
                    &module.id,
                    module.path.as_str(),
                )));
                fallback_path(&module.id)
            }
        };
        let claims_entry = entry_reserved && module.is_entry && base.as_str() == ENTRY_PATH;
        module.path = if claims_entry {
            entry_reserved = false;
            base
        } else {
            taken.unique(&module.id, base, &mut notes)
        };
    }
    notes
}
