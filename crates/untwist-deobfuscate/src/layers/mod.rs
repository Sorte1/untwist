pub mod decode;
pub mod eval;
pub mod function;
pub mod inline;
pub mod model;
pub mod notes;
pub mod packer;
pub mod probe;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;

pub use model::{DecodeCx, Decoded, Layer, LayerForm, Refusal, Refused};
pub use probe::{probe, Probe};

pub const LAYER_PASS: &str = "layers";

pub const INLINE_LAYER_PASS: &str = "inline-layers";

pub const LAYER_CAP: u32 = 16;

pub const FUEL_LAYERS: u64 = 4;

pub const TEXT_COPIES: u64 = 4;

pub fn decode(
    program: &Program<'_>,
    scoping: &Scoping,
    probe: Probe<'_, '_>,
    cx: &DecodeCx,
) -> Decoded {
    let decoded = match probe.form {
        LayerForm::Eval => eval::decode(program, scoping, probe, cx),
        LayerForm::Function => function::decode(program, scoping, probe, cx),
        LayerForm::Packer => packer::decode(program, scoping, probe, cx),
        LayerForm::InlineStmt | LayerForm::InlineExpr | LayerForm::InlineFunction => None,
    }?;
    if is_strict(program) {
        return Some(Err(Refused {
            form: probe.form,
            reason: Refusal::Shape,
        }));
    }
    Some(decoded)
}

pub fn verdict(
    form: LayerForm,
    program: &Program<'_>,
    unsafe_transforms: bool,
) -> Result<(), Refusal> {
    match form {
        LayerForm::Eval | LayerForm::Packer => eval::verdict(program, unsafe_transforms),
        _ => Ok(()),
    }
}

fn is_strict(program: &Program<'_>) -> bool {
    program
        .directives
        .iter()
        .any(|directive| directive.directive == "use strict")
}
