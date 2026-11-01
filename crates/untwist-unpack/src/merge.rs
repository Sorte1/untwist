use oxc_allocator::Allocator;
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::{Note, NoteKind};

use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{declared_names, runtime_id};
use crate::share::fold_runtime;

const PASS_NAME: &str = "unpack";

fn note(message: String) -> Note {
    Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message,
        span: None,
    }
}

fn retag_entry(bundle: &mut Bundle<'_>) {
    let entry_id = bundle.entry_id.clone();
    for (id, module) in bundle.modules.iter_mut() {
        let is_entry = entry_id.as_deref() == Some(id.as_str());
        if is_entry == module.is_entry {
            continue;
        }
        module.is_entry = is_entry;
        module.assign_default_path(is_entry);
    }
}

fn helper_texts<'m>(module: &'m Module<'_>) -> FxHashMap<String, &'m str> {
    let program = module.program();
    program
        .body
        .iter()
        .filter(|statement| statement.span().end < SYNTHETIC_BASE)
        .flat_map(|statement| {
            let text = statement.span().source_text(program.source_text);
            declared_names(std::slice::from_ref(statement))
                .into_iter()
                .map(move |name| (name, text))
        })
        .collect()
}

fn same_helpers(kept: &Module<'_>, later: &Module<'_>) -> bool {
    let kept = helper_texts(kept);
    let later = helper_texts(later);
    later
        .iter()
        .all(|(name, text)| kept.get(name).is_some_and(|known| known == text))
}

fn same_runtime(merged: &Bundle<'_>, bundle: &Bundle<'_>) -> bool {
    match (&merged.runtime, &bundle.runtime) {
        (Some(kept), Some(later)) => {
            kept.require_name == later.require_name && later.helpers.is_empty()
        }
        _ => true,
    }
}

fn linked_merge<'a>(alloc: &'a Allocator, merged: &mut Bundle<'a>, mut bundle: Bundle<'a>) -> bool {
    if merged.entry_id.is_some() && bundle.entry_id.is_some() || !same_runtime(merged, &bundle) {
        return false;
    }
    let key = runtime_id(bundle.kind);
    if let (Some(kept), Some(later)) = (merged.modules.get(&key), bundle.modules.get(&key)) {
        let folded = if same_helpers(kept, later) {
            fold_runtime(alloc, merged, &mut bundle, &key)
        } else if same_helpers(later, kept) {
            fold_runtime(alloc, &mut bundle, merged, &key)
        } else {
            None
        };
        if folded.is_none() {
            return false;
        }
    }
    if bundle
        .modules
        .keys()
        .any(|id| merged.modules.contains_key(id))
    {
        return false;
    }
    if merged.entry_id.is_none() {
        merged.entry_id = bundle.entry_id;
        merged.entry_candidates = bundle.entry_candidates;
    }
    if merged.runtime.is_none() {
        merged.runtime = bundle.runtime;
    }
    merged.fresh_requires.extend(bundle.fresh_requires);
    merged.fixed_paths.extend(bundle.fixed_paths);
    merged.modules.extend(bundle.modules);
    true
}

fn linked_groups<'a>(
    alloc: &'a Allocator,
    bundles: Vec<Bundle<'a>>,
    notes: &mut Vec<Note>,
) -> Vec<Bundle<'a>> {
    let mut out: Vec<Bundle<'a>> = Vec::with_capacity(bundles.len());
    let mut failed: Vec<String> = Vec::new();
    for bundle in bundles {
        let Some(link) = bundle.link.clone() else {
            out.push(bundle);
            continue;
        };
        if failed.contains(&link) {
            continue;
        }
        let Some(at) = out
            .iter()
            .position(|kept| kept.link.as_ref() == Some(&link))
        else {
            out.push(bundle);
            continue;
        };
        if out[at].kind != bundle.kind || !linked_merge(alloc, &mut out[at], bundle) {
            notes.push(note(crate::notes::group_refused(&link)));
            out.remove(at);
            failed.push(link);
        }
    }
    out
}

fn merge_webpack<'a>(merged: &mut Bundle<'a>, bundle: Bundle<'a>, notes: &mut Vec<Note>) {
    if merged.entry_id.is_none() {
        merged.entry_id = bundle.entry_id;
        merged.entry_candidates = bundle.entry_candidates;
    } else if let (Some(id), Some(kept)) = (&bundle.entry_id, &merged.entry_id) {
        notes.push(note(crate::notes::later_entry_ignored(id, kept)));
    }
    merged.fresh_requires.extend(bundle.fresh_requires);
    if merged.runtime.is_none() {
        merged.runtime = bundle.runtime;
    }
    for id in bundle.chunk_ids {
        if !merged.chunk_ids.contains(&id) {
            merged.chunk_ids.push(id);
        }
    }
    for (id, module) in bundle.modules {
        if merged.modules.contains_key(&id) {
            notes.push(note(crate::notes::duplicate_module(&id)));
            continue;
        }
        merged.modules.insert(id, module);
    }
}

pub fn merge_noted<'a>(
    alloc: &'a Allocator,
    bundles: Vec<Bundle<'a>>,
) -> (Option<Bundle<'a>>, Vec<Note>) {
    let mut notes = Vec::new();
    let mut rest = linked_groups(alloc, bundles, &mut notes).into_iter();
    let Some(mut merged) = rest.next() else {
        return (None, notes);
    };
    for bundle in rest {
        if bundle.kind != merged.kind {
            notes.push(note(crate::notes::merge_kind_mismatch(
                bundle.kind.as_str(),
                merged.kind.as_str(),
            )));
        } else if bundle.kind != BundleKind::Webpack {
            notes.push(note(crate::notes::second_self_contained(
                bundle.kind.as_str(),
            )));
        } else {
            merge_webpack(&mut merged, bundle, &mut notes);
        }
    }
    if merged.kind == BundleKind::Webpack {
        retag_entry(&mut merged);
    }
    (Some(merged), notes)
}

pub fn merge_bundles<'a>(
    alloc: &'a Allocator,
    bundles: Vec<Bundle<'a>>,
) -> Option<(Bundle<'a>, Vec<Note>)> {
    let (merged, notes) = merge_noted(alloc, bundles);
    merged.map(|bundle| (bundle, notes))
}
