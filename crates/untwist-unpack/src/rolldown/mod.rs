pub mod inline;
pub mod notes;
pub mod receiver;
pub mod shape;

use oxc_ast::ast::*;

pub fn marked(source: &Program<'_>) -> bool {
    source
        .comments
        .iter()
        .any(|comment| shape::runtime_marker(comment.span.source_text(source.source_text)))
}
