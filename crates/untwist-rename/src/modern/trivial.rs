use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    True,
    Other,
}

fn identifier<'e>(expr: &'e Expression<'_>) -> Option<&'e str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) if id.reference_id.get().is_some() => Some(id.name.as_str()),
        _ => None,
    }
}

fn returned<'s>(statement: &'s Statement<'_>) -> Option<&'s str> {
    match statement {
        Statement::ReturnStatement(ret) => identifier(ret.argument.as_ref()?),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => returned(only),
            _ => None,
        },
        _ => None,
    }
}

fn guard<'s>(statement: &'s Statement<'_>) -> Option<(&'s str, &'s str)> {
    let Statement::IfStatement(stmt) = statement else {
        return None;
    };
    if stmt.alternate.is_some() {
        return None;
    }
    Some((identifier(&stmt.test)?, returned(&stmt.consequent)?))
}

fn assignment<'s, 'a>(statement: &'s Statement<'a>) -> Option<(&'s str, &'s Expression<'a>)> {
    let Statement::ExpressionStatement(stmt) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = stmt.expression.without_parentheses() else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    Some((target.name.as_str(), &assign.right))
}

fn flag(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(number) => number.value == 1.0,
        Expression::BooleanLiteral(boolean) => boolean.value,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::LogicalNot
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(number) if number.value == 0.0)
        }
        _ => false,
    }
}

fn literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_) => true,
        Expression::ObjectExpression(object) => object.properties.is_empty(),
        Expression::ArrayExpression(array) => array.elements.is_empty(),
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::LogicalNot | UnaryOperator::Void | UnaryOperator::UnaryNegation
            ) && literal(&unary.argument)
        }
        _ => false,
    }
}

fn truthy(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(boolean) => boolean.value,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::LogicalNot
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(number) if number.value == 0.0)
        }
        _ => false,
    }
}

pub fn wrapper(function: &Function<'_>) -> Option<Value> {
    if !function.params.items.is_empty()
        || function.params.rest.is_some()
        || function.r#async
        || function.generator
    {
        return None;
    }
    let body = function.body.as_ref()?;
    if !body.directives.is_empty() {
        return None;
    }
    let [first, second, middle @ .., last] = body.statements.as_slice() else {
        return None;
    };
    let (state, exports) = guard(first)?;
    if state == exports || returned(last)? != exports {
        return None;
    }
    let (flagged, set) = assignment(second)?;
    if flagged != state || !flag(set) || middle.is_empty() {
        return None;
    }
    let mut value = Value::Other;
    for statement in middle {
        let (target, right) = assignment(statement)?;
        if target != exports || !literal(right) {
            return None;
        }
        if truthy(right) {
            value = Value::True;
        }
    }
    Some(value)
}
