use oxc_ast::ast::{Expression, IdentifierReference};
use oxc_syntax::operator::UnaryOperator;

pub fn literal_truthiness(
    expr: &Expression<'_>,
    is_global_undefined: &dyn Fn(&IdentifierReference<'_>) -> bool,
) -> Option<bool> {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        Expression::NumericLiteral(literal) => Some(number_is_truthy(literal.value)),
        Expression::StringLiteral(literal) => Some(literal.value.len_utf16() != 0),
        Expression::NullLiteral(_) => Some(false),
        Expression::Identifier(id) => {
            (id.name == "undefined" && is_global_undefined(id)).then_some(false)
        }
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::Void => {
                literal_truthiness(&unary.argument, is_global_undefined).map(|_| false)
            }
            UnaryOperator::LogicalNot => {
                literal_truthiness(&unary.argument, is_global_undefined).map(|value| !value)
            }
            UnaryOperator::UnaryNegation => match unary.argument.without_parentheses() {
                Expression::NumericLiteral(literal) => Some(number_is_truthy(literal.value)),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn number_is_truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}
