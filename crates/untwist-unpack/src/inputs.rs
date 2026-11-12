use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::{Note, NoteKind};

use crate::detect::{refused_kind, unpack_program_linked};
use crate::links::{groups, Groups, Links};
use crate::mappings::Mapping;
use crate::merge::merge_noted;
use crate::model::Bundle;
use crate::{finish_with, notes, turbopack, UnpackOptions};

const PASS_NAME: &str = "unpack";

fn note(message: String) -> Note {
    Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message,
        span: None,
    }
}

fn detected<'a>(
    alloc: &'a Allocator,
    programs: &mut [Program<'a>],
    groups: &Groups,
    options: UnpackOptions,
    notes: &mut Vec<Note>,
) -> Vec<Option<Bundle<'a>>> {
    let none = Links::default();
    programs
        .iter_mut()
        .enumerate()
        .map(|(input, program)| {
            if groups.skipped(input) {
                return None;
            }
            let links = groups.views.get(input).unwrap_or(&none);
            let mut unproven = None;
            let mut bundle = unpack_program_linked(alloc, program, options, links, &mut unproven);
            if let Some(bundle) = bundle.as_mut() {
                bundle.inputs = vec![input];
            }
            if bundle.is_none() {
                let refused = unproven.or_else(|| {
                    refused_kind(program, options).map(|kind| notes::shape_refused(kind.as_str()))
                });
                notes.extend(refused.map(note));
            }
            bundle
        })
        .collect()
}

fn complete<'a>(
    alloc: &'a Allocator,
    programs: &mut [Program<'a>],
    groups: &Groups,
    found: &mut [Option<Bundle<'a>>],
    notes: &mut Vec<Note>,
) -> Vec<Bundle<'a>> {
    let mut extra = Vec::new();
    for group in &groups.members {
        let whole = group.inputs.iter().all(|&input| {
            found[input]
                .as_ref()
                .is_some_and(|bundle| bundle.link.as_deref() == Some(group.namespace.as_str()))
        });
        let root = match (&group.turbopack, whole) {
            (Some(server), true) => turbopack::loader_bundle(alloc, programs, server),
            (None, true) => continue,
            (_, false) => None,
        };
        match root {
            Some(mut root) => {
                root.inputs = group.inputs.clone();
                extra.push(root);
            }
            None => {
                notes.push(note(notes::group_refused(&group.namespace)));
                for &input in &group.inputs {
                    found[input] = None;
                }
            }
        }
    }
    found
        .iter_mut()
        .filter_map(Option::take)
        .chain(extra)
        .collect()
}

fn moved_fixed_path(bundle: &Bundle<'_>) -> Option<String> {
    bundle
        .fixed_paths
        .iter()
        .find(|id| {
            bundle
                .modules
                .get(id.as_str())
                .is_none_or(|module| module.path.as_str() != id.as_str())
        })
        .cloned()
}

pub fn unpack_inputs<'a>(
    alloc: &'a Allocator,
    programs: &mut [Program<'a>],
    mappings: &[Mapping],
    options: UnpackOptions,
) -> (Option<Bundle<'a>>, Vec<Note>) {
    unpack_named_inputs(alloc, programs, mappings, options, &[])
}

fn mirrored_inputs(bundle: &mut Bundle<'_>, names: &[String]) -> Vec<String> {
    bundle.inputs.sort_unstable();
    bundle.inputs.dedup();
    let partial = names.len() > 1 && bundle.inputs.len() < names.len();
    match partial {
        true => names.to_vec(),
        false => Vec::new(),
    }
}

pub fn unpack_named_inputs<'a>(
    alloc: &'a Allocator,
    programs: &mut [Program<'a>],
    mappings: &[Mapping],
    options: UnpackOptions,
    names: &[String],
) -> (Option<Bundle<'a>>, Vec<Note>) {
    let names = match names.len() == programs.len() {
        true => names,
        false => &[],
    };
    let groups = groups(programs);
    let mut notes = Vec::new();
    let mut found = detected(alloc, programs, &groups, options, &mut notes);
    let bundles = complete(alloc, programs, &groups, &mut found, &mut notes);
    let (merged, merge_notes) = merge_noted(alloc, bundles);
    notes.extend(merge_notes);
    let Some(mut bundle) = merged else {
        return (None, notes);
    };
    bundle.mirrored_inputs = mirrored_inputs(&mut bundle, names);
    notes.extend(finish_with(alloc, &mut bundle, mappings, options));
    if let Some(moved) = moved_fixed_path(&bundle) {
        notes.push(note(notes::group_refused(moved)));
        return (None, notes);
    }
    (Some(bundle), notes)
}
