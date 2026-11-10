pub mod amd;
pub mod assign;
pub mod browserify;
pub mod bun;
pub mod closed;
pub mod closure;
pub mod detect;
pub mod esbuild;
pub mod frame;
pub mod inputs;
pub mod links;
pub mod mappings;
pub mod matcher;
pub mod merge;
pub mod metro;
pub mod model;
pub mod notes;
pub mod parcel;
pub mod path;
pub mod rename;
pub mod rollup;
pub mod runtime;
pub mod scan;
pub mod share;
pub mod specifier;
pub mod systemjs;
pub mod turbopack;
pub mod umd;
pub mod wasm;
pub mod webpack;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::{Note, NoteKind};

pub use detect::{
    holds_a_bundle, holds_a_bundle_with, unpack_program, unpack_program_taking,
    unpack_program_taking_with, unpack_program_with,
};
pub use inputs::unpack_inputs;
pub use mappings::{Mapping, MappingMatcher};
pub use merge::{merge_bundles, merge_noted};
pub use model::{Bundle, BundleKind, Module};

const PASS_NAME: &str = "unpack";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnpackOptions {
    pub split_hoisted: bool,
    pub conservative: bool,
}

pub struct UnpackResult<'a> {
    pub bundle: Bundle<'a>,
    pub notes: Vec<Note>,
}

fn entry_candidates_note(bundle: &Bundle<'_>) -> Option<Note> {
    let id = bundle.entry_id.as_deref()?;
    if bundle.entry_candidates <= 1 {
        return None;
    }
    Some(Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message: notes::entry_candidates(id, bundle.entry_candidates),
        span: None,
    })
}

fn kind_transforms<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    options: UnpackOptions,
) -> Vec<Note> {
    match bundle.kind {
        BundleKind::Webpack => {
            webpack::transforms::apply_transforms_with(alloc, bundle, options.conservative)
        }
        BundleKind::Browserify => Vec::new(),
        BundleKind::Esbuild => esbuild::transforms(alloc, bundle),
        BundleKind::Rollup => rollup::transforms(alloc, bundle),
        BundleKind::SystemJs => systemjs::transforms(alloc, bundle),
        BundleKind::Amd => amd::transforms(alloc, bundle),
        BundleKind::Umd => umd::transforms(alloc, bundle),
        BundleKind::Parcel => parcel::transforms(alloc, bundle),
        BundleKind::Metro => metro::transforms(alloc, bundle),
        BundleKind::Turbopack => turbopack::transforms(alloc, bundle),
        BundleKind::Closure => closure::transforms(alloc, bundle),
    }
}

pub fn finish<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    mappings: &[Mapping],
) -> Vec<Note> {
    finish_with(alloc, bundle, mappings, UnpackOptions::default())
}

pub fn finish_with<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    mappings: &[Mapping],
    options: UnpackOptions,
) -> Vec<Note> {
    let mut notes = mappings::apply_mappings(bundle, mappings);
    notes.extend(entry_candidates_note(bundle));
    notes.extend(assign::assign_paths(bundle));
    notes.extend(kind_transforms(alloc, bundle, options));
    notes
}

pub fn unpack<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
    mappings: &[Mapping],
) -> Option<UnpackResult<'a>> {
    let mut bundle = unpack_program(alloc, program)?;
    let notes = finish(alloc, &mut bundle, mappings);
    Some(UnpackResult { bundle, notes })
}

pub fn unpack_taking<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    mappings: &[Mapping],
) -> Option<UnpackResult<'a>> {
    let mut bundle = unpack_program_taking(alloc, program)?;
    let notes = finish(alloc, &mut bundle, mappings);
    Some(UnpackResult { bundle, notes })
}
