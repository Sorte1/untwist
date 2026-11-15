use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::{Note, NoteKind};

use crate::browserify::notes as browserify_notes;
use crate::browserify::resolve::left_request_reaching;
use crate::detect::{refused_kind, unpack_program_linked};
use crate::goal::written_module_syntax;
use crate::host_request::{host_uses, unproven_import, HostUse};
use crate::links::{groups, Groups, Links};
use crate::mappings::Mapping;
use crate::merge::merge_noted;
use crate::model::Bundle;
use crate::strict::{keep_strictness, mark_from_inputs, sloppy_written_module};
use crate::{finish_with, notes, turbopack, UnpackOptions};
use untwist_core::ast_util::naming_slot::scan_name_reads;
use untwist_core::semantic::closed_evals_kept;

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
    (notes, refused_any): (&mut Vec<Note>, &mut bool),
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
                *refused_any |= refused.is_some();
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

fn may_eval(text: &str) -> bool {
    text.contains("eval") || text.contains("\\u")
}

fn eval_name_rebound(bundle: &Bundle<'_>) -> Option<String> {
    bundle
        .modules
        .iter()
        .find(|(_, module)| !closed_evals_kept(module.program()))
        .map(|(id, _)| id.clone())
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

fn chunks_checked(
    bundle: &Bundle<'_>,
    options: UnpackOptions,
    refused_any: bool,
) -> Result<(), String> {
    if options.conservative {
        return Ok(());
    }
    let orphaned = refused_any
        && bundle.kind == crate::model::BundleKind::Webpack
        && bundle.runtime.is_none()
        && !bundle.chunk_modules.is_empty();
    if orphaned {
        return Err(notes::chunks_without_runtime());
    }
    if bundle.chunk_started && bundle.runtime.is_none() {
        return Err(notes::chunk_start_without_runtime());
    }
    if let Some(file) = bundle
        .chunk_files
        .iter()
        .find(|file| file.framed && bundle.runtime.is_some())
    {
        return Err(notes::chunk_framed(file.ids.join(", ")));
    }
    crate::webpack::gate::chunk_modules_routed(bundle, options.trust_names)?;
    crate::webpack::loads::chunks_supplied(bundle)
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

fn renamed_a_function_name(notes: &mut Vec<Note>) -> bool {
    let renamed = crate::rename::took_name_hazard();
    if renamed {
        notes.push(note(notes::RENAMED_FUNCTION_NAME.to_string()));
    }
    renamed
}

fn input_names(bundle: &Bundle<'_>, names: &[String]) -> Vec<String> {
    bundle
        .inputs
        .iter()
        .filter_map(|input| names.get(*input).cloned())
        .collect()
}

fn host_checked(bundle: &mut Bundle<'_>, options: UnpackOptions) -> Result<(), HostUse> {
    match options.conservative {
        true => Ok(()),
        false => host_uses(bundle),
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
    let groups = groups(programs, names);
    let mut notes = Vec::new();
    let mut refused_any = false;
    let _names = scan_name_reads(options.trust_names);
    crate::rename::clear_name_hazard();
    let mut found = detected(
        alloc,
        programs,
        &groups,
        options,
        (&mut notes, &mut refused_any),
    );
    let bundles = complete(alloc, programs, &groups, &mut found, &mut notes);
    if renamed_a_function_name(&mut notes) {
        return (None, notes);
    }
    if bundles.len() > 1 && bundles.iter().any(|bundle| bundle.around.is_some()) {
        let refused = bundles.iter().find_map(|bundle| match &bundle.around {
            Some(crate::beside::Around::Refused(why)) => Some(why.clone()),
            _ => None,
        });
        notes.push(note(refused.unwrap_or_else(|| {
            notes::code_beside_entry("the bundle is joined from several inputs")
        })));
        return (None, notes);
    }
    let (merged, merge_notes) = merge_noted(alloc, bundles);
    notes.extend(merge_notes);
    let Some(mut bundle) = merged else {
        return (None, notes);
    };
    if renamed_a_function_name(&mut notes) {
        return (None, notes);
    }
    if let Err(why) = chunks_checked(&bundle, options, refused_any) {
        notes.push(note(why));
        return (None, notes);
    }
    bundle.mirrored_inputs = mirrored_inputs(&mut bundle, names);
    bundle.input_names = input_names(&bundle, names);
    if let Err(found) = host_checked(&mut bundle, options) {
        notes.push(note(notes::host_route(&found.id, &found.what)));
        return (None, notes);
    }
    notes.extend(finish_with(alloc, &mut bundle, mappings, options));
    if renamed_a_function_name(&mut notes) {
        return (None, notes);
    }
    if let Some(reached) = left_request_reaching(&bundle) {
        notes.push(note(browserify_notes::left_request_reaches(
            &reached.id,
            reached.request.as_deref(),
            &reached.path,
        )));
        return (None, notes);
    }
    if !options.conservative {
        mark_from_inputs(programs, &mut bundle);
    }
    if let Some(found) = unproven_import(&bundle) {
        notes.push(note(notes::host_route(&found.id, &found.what)));
        return (None, notes);
    }
    if let Err(why) = crate::beside::keep_around(alloc, &mut bundle) {
        notes.push(note(why));
        return (None, notes);
    }
    if let Some(moved) = moved_fixed_path(&bundle) {
        notes.push(note(notes::group_refused(moved)));
        return (None, notes);
    }
    let may_eval = programs.iter().any(|program| may_eval(program.source_text));
    if let Some(id) = may_eval.then(|| eval_name_rebound(&bundle)).flatten() {
        notes.push(note(notes::eval_name_rebound(id)));
        return (None, notes);
    }
    if options.conservative {
        return (Some(bundle), notes);
    }
    if let Some((id, what)) = written_module_syntax(&bundle) {
        notes.push(note(notes::module_syntax_written(id, what)));
        return (None, notes);
    }
    keep_strictness(alloc, &mut bundle);
    if let Some(id) = options
        .prove_frames
        .then(|| sloppy_written_module(&bundle))
        .flatten()
    {
        notes.push(note(notes::sloppy_file(id)));
        return (None, notes);
    }
    (Some(bundle), notes)
}
