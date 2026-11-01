use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;

pub fn undefined_expression<'a>(span: Span, b: &impl GetAstBuilder<'a>) -> Expression<'a> {
    Expression::new_unary_expression(
        span,
        UnaryOperator::Void,
        Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, b),
        b,
    )
}

pub fn is_safe_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::StringLiteral(_) => true,
        Expression::TemplateLiteral(t) => t.expressions.is_empty(),
        _ => false,
    }
}

pub fn is_pure_value(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::Identifier(id) => {
            matches!(id.name.as_str(), "undefined" | "NaN" | "Infinity")
        }
        Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => {
            match &u.argument {
                Expression::NumericLiteral(_) => true,
                Expression::Identifier(id) => id.name.as_str() == "Infinity",
                _ => false,
            }
        }
        _ => false,
    }
}

pub fn is_undefined_like(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name.as_str() == "undefined",
        Expression::UnaryExpression(u) => {
            u.operator == UnaryOperator::Void && matches!(u.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

pub fn is_true_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(b) => b.value,
        Expression::UnaryExpression(outer) if outer.operator == UnaryOperator::LogicalNot => {
            match &outer.argument {
                Expression::NumericLiteral(n) => n.value == 0.0,
                Expression::UnaryExpression(inner)
                    if inner.operator == UnaryOperator::LogicalNot =>
                {
                    is_truthy_double_negated_argument(&inner.argument)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

pub fn is_false_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(b) => !b.value,
        Expression::UnaryExpression(u) if u.operator == UnaryOperator::LogicalNot => {
            matches!(&u.argument, Expression::ArrayExpression(a) if a.elements.is_empty())
        }
        _ => false,
    }
}

pub fn is_truthy_literal(expr: &Expression<'_>) -> bool {
    if is_true_literal(expr) {
        return true;
    }
    matches!(expr.without_parentheses(), Expression::ArrayExpression(a) if a.elements.is_empty())
}

pub fn numeric_value(expr: &Expression<'_>) -> Option<f64> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(n) => Some(n.value),
        Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => {
            match &u.argument {
                Expression::NumericLiteral(n) => Some(-n.value),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_truthy_double_negated_argument(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::NumericLiteral(n) => n.value == 1.0,
        Expression::ArrayExpression(a) => a.elements.is_empty(),
        _ => false,
    }
}
