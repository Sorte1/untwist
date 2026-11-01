use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::{Span, SPAN};
use untwist_core::pass::{Cx, Safety};

use super::identify::Table;
use super::shapes::Kind;
use super::spread::{shape, Gate};
use super::values::{has_elision, into_inner, is_non_array_literal, take_arguments};

#[derive(Clone, Copy, PartialEq, Eq)]
enum From {
    Inline,
    Read,
    Spread,
}

fn elements_of<'a>(
    expression: Expression<'a>,
    cx: &Cx<'a, '_>,
) -> AVec<'a, ArrayExpressionElement<'a>> {
    match into_inner(expression) {
        Expression::ArrayExpression(array) => array.unbox().elements,
        other => AVec::from_iter_in([spread_of(other, cx)], cx),
    }
}

fn spread_of<'a>(argument: Expression<'a>, cx: &Cx<'a, '_>) -> ArrayExpressionElement<'a> {
    ArrayExpressionElement::new_spread_element(SPAN, argument, cx)
}

fn array_of<'a>(
    span: Span,
    elements: AVec<'a, ArrayExpressionElement<'a>>,
    cx: &Cx<'a, '_>,
) -> Expression<'a> {
    Expression::new_array_expression(span, elements, cx)
}

fn argument<'k, 'a>(call: &'k CallExpression<'a>, index: usize) -> Option<&'k Expression<'a>> {
    call.arguments
        .get(index)
        .and_then(Argument::as_expression)
        .map(Expression::without_parentheses)
}

pub fn to_consumable<'a>(
    node: &mut Expression<'a>,
    gate: &Gate<'_>,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    let Expression::CallExpression(call) = &*node else {
        return None;
    };
    if call.arguments.len() != 1 {
        return None;
    }
    let (safety, inline) = match argument(call, 0)? {
        Expression::ArrayExpression(array) if !has_elision(array) => (Safety::Safe, true),
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => (Safety::Safe, false),
        _ => (Safety::Unsafe, false),
    };
    if !gate.allows(safety) {
        return None;
    }
    let (span, mut arguments) = take_arguments(node, cx)?;
    let value = arguments.pop()?;
    let elements = if inline {
        elements_of(value, cx)
    } else {
        AVec::from_iter_in([spread_of(value, cx)], cx)
    };
    *node = array_of(span, elements, cx);
    Some(shape("to-consumable-array", safety))
}

fn pack_of(call: &CallExpression<'_>) -> Option<bool> {
    match call.arguments.len() {
        2 => Some(true),
        3 => match argument(call, 2)? {
            Expression::BooleanLiteral(literal) => Some(literal.value),
            _ => None,
        },
        _ => None,
    }
}

fn is_read_call(expression: &Expression<'_>, table: &Table, cx: &Cx<'_, '_>) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };
    !call.optional
        && call.arguments.len() == 1
        && call.arguments.first().is_some_and(Argument::is_expression)
        && table.kind_of_callee(&call.callee, cx.scoping()) == Some(Kind::Read)
}

fn from_of(call: &CallExpression<'_>, table: &Table, cx: &Cx<'_, '_>) -> Option<From> {
    let pack = pack_of(call)?;
    let from = argument(call, 1)?;
    match from {
        Expression::ArrayExpression(array) if !pack || !has_elision(array) => Some(From::Inline),
        Expression::ArrayExpression(_) => None,
        _ if is_read_call(from, table, cx) => Some(From::Read),
        _ if pack && !is_non_array_literal(from, cx.scoping()) => Some(From::Spread),
        _ => None,
    }
}

fn read_argument(expression: Expression<'_>) -> Option<Expression<'_>> {
    match into_inner(expression) {
        Expression::CallExpression(call) => call
            .unbox()
            .arguments
            .into_iter()
            .next()
            .and_then(|argument| Expression::try_from(argument).ok())
            .map(into_inner),
        _ => None,
    }
}

pub fn spread_array<'a>(
    node: &mut Expression<'a>,
    table: &Table,
    gate: &Gate<'_>,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    let Expression::CallExpression(call) = &*node else {
        return None;
    };
    if !matches!(argument(call, 0)?, Expression::ArrayExpression(_)) {
        return None;
    }
    let from = from_of(call, table, cx)?;
    let safety = if from == From::Inline {
        Safety::Safe
    } else {
        Safety::Unsafe
    };
    if !gate.allows(safety) {
        return None;
    }
    let (span, arguments) = take_arguments(node, cx)?;
    let mut arguments = arguments.into_iter();
    let mut elements = elements_of(arguments.next()?, cx);
    let value = arguments.next()?;
    match from {
        From::Inline => elements.extend(elements_of(value, cx)),
        From::Read => elements.push(spread_of(read_argument(value)?, cx)),
        From::Spread => elements.push(spread_of(value, cx)),
    }
    *node = array_of(span, elements, cx);
    let kind = if from == From::Read {
        "spread-array-read"
    } else {
        "spread-array"
    };
    Some(shape(kind, safety))
}

fn concat_receiver<'k, 'a>(call: &'k CallExpression<'a>) -> Option<&'k ArrayExpression<'a>> {
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    if member.optional || member.property.name.as_str() != "concat" {
        return None;
    }
    match member.object.without_parentheses() {
        Expression::ArrayExpression(array) => Some(array),
        _ => None,
    }
}

pub fn concat<'a>(
    node: &mut Expression<'a>,
    gate: &Gate<'_>,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    let Expression::CallExpression(call) = &*node else {
        return None;
    };
    concat_receiver(call)?;
    let all_arrays = !call.arguments.is_empty()
        && (0..call.arguments.len())
            .all(|index| matches!(argument(call, index), Some(Expression::ArrayExpression(_))));
    if !all_arrays || !gate.allows(Safety::Safe) {
        return None;
    }
    let Expression::CallExpression(call) = node else {
        return None;
    };
    let span = call.span;
    let callee = std::mem::replace(&mut call.callee, Expression::new_null_literal(SPAN, cx));
    let receiver = match into_inner(callee) {
        Expression::StaticMemberExpression(member) => member.unbox().object,
        _ => return None,
    };
    let (_, arguments) = take_arguments(node, cx)?;
    let mut elements = elements_of(receiver, cx);
    for value in arguments {
        elements.extend(elements_of(value, cx));
    }
    *node = array_of(span, elements, cx);
    Some(shape("concat", Safety::Safe))
}
