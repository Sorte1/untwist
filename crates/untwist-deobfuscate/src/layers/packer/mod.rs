use oxc_ast::ast::Program;
use oxc_semantic::Scoping;

use super::model::{DecodeCx, Decoded};
use super::probe::Probe;

pub fn decode(
    _program: &Program<'_>,
    _scoping: &Scoping,
    _probe: Probe<'_, '_>,
    _cx: &DecodeCx,
) -> Decoded {
    None
}
