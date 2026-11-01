#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerForm {
    Eval,
    Function,
    Packer,
    InlineStmt,
    InlineExpr,
    InlineFunction,
}

impl LayerForm {
    pub fn name(self) -> &'static str {
        match self {
            LayerForm::Eval => "W-eval",
            LayerForm::Function => "W-Function",
            LayerForm::Packer => "W-packer",
            LayerForm::InlineStmt => "I-stmt",
            LayerForm::InlineExpr => "I-expr",
            LayerForm::InlineFunction => "I-Function",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Refusal {
    NotAString,
    Bailed,
    Unparsable,
    Injection,
    TooDeep,
    TooLarge,
    NeedsUnsafe,
    Shape,
    Cap,
    Cycle,
    Fuel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeCx {
    pub unsafe_transforms: bool,
    pub fuel: u64,
    pub heap: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub form: LayerForm,
    pub text: String,
    pub steps: u64,
    pub heap: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refused {
    pub form: LayerForm,
    pub reason: Refusal,
}

pub type Decoded = Option<Result<Layer, Refused>>;
