mod build;
mod exports;
mod layout;
mod link;
pub mod notes;
mod scan;
mod verify;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::Note;

use crate::frame::Frame;
use crate::model::{Bodies, Bundle, BundleKind};

pub const DYNAMIC: &str = "import()";

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    layout::probe(frame)
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let layout = layout::layout(frame, source)?;
    if !matches!(bodies, Bodies::Taken(_)) && !verify::verify(source, &layout) {
        return None;
    }
    let names = layout.names();
    let mut shared = build::Shared {
        names: &names,
        prefix: None,
        text: source.source_text,
        names_read: None,
    };
    let mut bundle = Bundle::new(
        BundleKind::SystemJs,
        Some(layout.id(layout.entry).to_string()),
    );
    for &at in &layout.live {
        let module = build::module(alloc, (source, &layout), at, bodies, &mut shared)?;
        bundle.modules.insert(module.id.clone(), module);
    }
    Some(bundle)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    link::link(alloc, bundle);
    Vec::new()
}
