pub mod inline;
pub mod notes;
pub mod receiver;
pub mod shape;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::esbuild;
use crate::frame::Frame;
use crate::model::{Bodies, Bundle, BundleKind};

fn marked(source: &Program<'_>) -> bool {
    source
        .comments
        .iter()
        .any(|comment| shape::runtime_marker(comment.span.source_text(source.source_text)))
}

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    esbuild::probe_kind(frame, false) == Some(BundleKind::Rolldown)
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let plan = esbuild::found(source, frame, marked(source))?;
    (plan.layout.kind == BundleKind::Rolldown).then_some(())?;
    esbuild::detect_plan(alloc, source, plan, bodies)
}
