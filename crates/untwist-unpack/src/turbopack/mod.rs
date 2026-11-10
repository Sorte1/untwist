mod build;
mod layout;
mod link;
pub mod links;
pub mod notes;
mod verify;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::{Note, NoteKind};

use crate::frame::Frame;
use crate::links::Links;
use crate::model::{Bodies, Bundle};

pub use build::loader_bundle;
pub use links::NAMESPACE;

pub const ENTRY_ID: &str = "index";
pub const MAX_PARAMS: usize = 3;
pub const NAMES: [&str; 2] = ["module", "exports"];

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    layout::probe(frame)
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    _frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
    links: &Links,
) -> Option<Bundle<'a>> {
    let view = links.turbopack.as_ref();
    let layout = match view {
        Some(_) => layout::server(source)?,
        None => layout::layout(source)?,
    };
    let foreign = view.map(|view| &view.ids);
    if !matches!(bodies, Bodies::Taken(_)) && !verify::verify(source, &layout, foreign) {
        return None;
    }
    let mut bundle = build::bundle(alloc, source, &layout, bodies)?;
    bundle.link = view.map(|_| NAMESPACE.to_string());
    Some(bundle)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    link::link(alloc, bundle);
    let premise = bundle.link.is_none().then(|| Note {
        pass: "unpack",
        kind: NoteKind::Progress,
        message: notes::browser_premise(),
        span: None,
    });
    premise.into_iter().collect()
}
