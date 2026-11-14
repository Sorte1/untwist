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

fn exports_value<'e, 'a>(
    statement: &'e Statement<'a>,
    exports: &str,
) -> Option<&'e Expression<'a>> {
    let (target, right) = assignment(statement)?;
    (target == exports && literal(right)).then_some(right)
}

fn returned_value<'e, 'a>(
    statement: &'e Statement<'a>,
    exports: &str,
) -> Option<&'e Expression<'a>> {
    let Statement::ReturnStatement(ret) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = ret.argument.as_ref()?.without_parentheses()
    else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    let assigns = assign.operator == AssignmentOperator::Assign && target.name == exports;
    (assigns && literal(&assign.right)).then_some(&assign.right)
}

fn sets_flag(statement: &Statement<'_>, state: &str) -> bool {
    assignment(statement).is_some_and(|(flagged, set)| flagged == state && flag(set))
}

fn value_of<'e, 'a: 'e>(
    values: impl IntoIterator<Item = Option<&'e Expression<'a>>>,
) -> Option<Value> {
    let values: Vec<&Expression<'a>> = values.into_iter().collect::<Option<_>>()?;
    match values.as_slice() {
        [] => None,
        values if values.iter().any(|right| truthy(right)) => Some(Value::True),
        _ => Some(Value::Other),
    }
}

fn initialised(tail: &[Statement<'_>], state: &str, exports: &str) -> Option<Value> {
    let [second, middle @ .., last] = tail else {
        return None;
    };
    if !sets_flag(second, state) {
        return None;
    }
    let assigned = middle
        .iter()
        .map(|statement| exports_value(statement, exports));
    match returned(last) {
        Some(name) if name == exports => value_of(assigned),
        _ => value_of(assigned.chain(std::iter::once(returned_value(last, exports)))),
    }
}

fn guarded(statements: &[Statement<'_>]) -> Option<Value> {
    let (Statement::IfStatement(guard), rest) = statements.split_first()? else {
        return None;
    };
    let state = identifier(&guard.test)?;
    let exports = returned(&guard.consequent)?;
    if state == exports {
        return None;
    }
    let tail = match (&guard.alternate, rest) {
        (None, rest) => rest,
        (Some(Statement::BlockStatement(block)), []) => block.body.as_slice(),
        _ => return None,
    };
    initialised(tail, state, exports)
}

fn negated(statements: &[Statement<'_>]) -> Option<Value> {
    let [Statement::IfStatement(guard), last] = statements else {
        return None;
    };
    if guard.alternate.is_some() {
        return None;
    }
    let Expression::UnaryExpression(not) = guard.test.without_parentheses() else {
        return None;
    };
    if not.operator != UnaryOperator::LogicalNot {
        return None;
    }
    let state = identifier(&not.argument)?;
    let exports = returned(last)?;
    let Statement::BlockStatement(block) = &guard.consequent else {
        return None;
    };
    let [second, middle @ ..] = block.body.as_slice() else {
        return None;
    };
    if state == exports || !sets_flag(second, state) {
        return None;
    }
    value_of(
        middle
            .iter()
            .map(|statement| exports_value(statement, exports)),
    )
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
    guarded(&body.statements).or_else(|| negated(&body.statements))
}
