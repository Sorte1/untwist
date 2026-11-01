use untwist_core::catalogue;

use super::model::Refusal;

pub const LAYER_PEELED: &str = "peeled whole-program layer";

pub const LAYER_NOT_A_STRING: &str = "a layer was kept: its argument does not evaluate to a string";

pub const LAYER_BAILED: &str = "a layer was kept: computing its text read the host, wrote outside \
     the evaluator or reached a form the evaluator does not model";

pub const LAYER_UNPARSABLE: &str = "a layer was kept: its text does not parse as a script";

pub const LAYER_INJECTION: &str = "a layer was kept: its Function parameters or body do not \
     parse alone, so joining them could run code outside the function";

pub const LAYER_TOO_DEEP: &str = "a layer was kept: its text nests deeper than the stack this \
     input was admitted with";

pub const LAYER_TOO_LARGE: &str = "a layer was kept: its text or its printed output is over the \
     budget this input was admitted with";

pub const LAYER_NEEDS_UNSAFE: &str = "a layer was kept: peeling it assumes nothing observes that its \
     top-level bindings change from eval bindings to script bindings; pass --unsafe to peel it";

pub const LAYER_SHAPE: &str = "a layer was kept: its call or its site is not a shape the layer \
     rules cover";

pub const LAYER_CAP: &str = "a layer was kept: this input already peeled the most whole-program \
     layers one input may peel";

pub const LAYER_CYCLE: &str = "a layer was kept: its text equals an earlier layer of this input";

pub const LAYER_FUEL: &str = "a layer was kept: decoding it would spend more steps than one \
     layer, or all layers of this input, may spend";

catalogue!(
    LAYER_NOTES,
    LAYER_PEELED,
    LAYER_NOT_A_STRING,
    LAYER_BAILED,
    LAYER_UNPARSABLE,
    LAYER_INJECTION,
    LAYER_TOO_DEEP,
    LAYER_TOO_LARGE,
    LAYER_NEEDS_UNSAFE,
    LAYER_SHAPE,
    LAYER_CAP,
    LAYER_CYCLE,
    LAYER_FUEL
);

impl Refusal {
    pub fn text(self) -> &'static str {
        match self {
            Refusal::NotAString => LAYER_NOT_A_STRING,
            Refusal::Bailed => LAYER_BAILED,
            Refusal::Unparsable => LAYER_UNPARSABLE,
            Refusal::Injection => LAYER_INJECTION,
            Refusal::TooDeep => LAYER_TOO_DEEP,
            Refusal::TooLarge => LAYER_TOO_LARGE,
            Refusal::NeedsUnsafe => LAYER_NEEDS_UNSAFE,
            Refusal::Shape => LAYER_SHAPE,
            Refusal::Cap => LAYER_CAP,
            Refusal::Cycle => LAYER_CYCLE,
            Refusal::Fuel => LAYER_FUEL,
        }
    }
}

pub fn refused_message(reason: Refusal, index: u32, form: &str) -> String {
    format!("{}; layer {index}, {form}", reason.text())
}

pub fn peeled_message(index: u32, form: &str, bytes_in: usize, bytes_out: usize) -> String {
    format!("{LAYER_PEELED} {index}: {form}, {bytes_in} → {bytes_out} bytes")
}
