use oxc_ast::ast::{CallExpression, Expression};

#[derive(Default)]
pub(crate) struct Hazards {
    pub(crate) eval: bool,
    pub(crate) with: bool,
}

impl Hazards {
    pub(crate) fn name(&self) -> Option<&'static str> {
        match (self.eval, self.with) {
            (true, true) => Some("direct eval and with statement"),
            (true, false) => Some("direct eval"),
            (false, true) => Some("with statement"),
            (false, false) => None,
        }
    }
}

pub(crate) fn is_direct_eval(call: &CallExpression<'_>) -> bool {
    !call.optional
        && matches!(
            call.callee.without_parentheses(),
            Expression::Identifier(id) if id.name.as_str() == "eval"
        )
}
