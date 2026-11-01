use rustc_hash::FxHashSet;
use untwist_core::pass::{Note, NoteKind};

use crate::model::{fallback_path, Bundle};
use crate::path::{PosixPath, ENTRY_PATH};

const PASS_NAME: &str = "unpack";
const MAX_COLLISION_SUFFIX: usize = 1000;

fn note(message: String) -> Note {
    Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message,
        span: None,
    }
}

pub fn assign_paths(bundle: &mut Bundle<'_>) -> Vec<Note> {
    let mut notes = Vec::new();
    let mut taken: FxHashSet<String> = FxHashSet::default();
    let mut entry_reserved = bundle.modules.values().any(|module| module.is_entry);
    if entry_reserved {
        taken.insert(ENTRY_PATH.to_string());
    }
    for module in bundle.modules.values_mut() {
        if let Some(source) = &module.unsafe_path_source {
            notes.push(note(crate::notes::unsafe_path_replaced(
                &module.id,
                source,
                module.path.as_str(),
            )));
        }
        let base = match PosixPath::parse(module.path.as_str()) {
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
            unique(&mut taken, &module.id, base, &mut notes)
        };
    }
    notes
}

fn unique(
    taken: &mut FxHashSet<String>,
    id: &str,
    base: PosixPath,
    notes: &mut Vec<Note>,
) -> PosixPath {
    if !taken.contains(base.as_str()) {
        taken.insert(base.as_str().to_string());
        return base;
    }
    let mut path = base.clone();
    let mut suffix = 2usize;
    while taken.contains(path.as_str()) && suffix <= MAX_COLLISION_SUFFIX {
        path = base.with_stem_suffix(&format!("-{suffix}"));
        suffix += 1;
    }
    if taken.contains(path.as_str()) {
        notes.push(note(crate::notes::uniquify_exhausted(
            id,
            base.as_str(),
            MAX_COLLISION_SUFFIX,
        )));
    } else if suffix > 2 {
        notes.push(note(crate::notes::path_collision(
            id,
            path.as_str(),
            base.as_str(),
        )));
    }
    taken.insert(path.as_str().to_string());
    path
}
