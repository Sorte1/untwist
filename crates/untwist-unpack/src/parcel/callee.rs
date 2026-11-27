use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use untwist_core::ast_util::prop::member_parts;

use super::layout::NAME_PREFIX;

pub const GLOBAL_NAMES: [&str; 4] = ["globalThis", "self", "window", "global"];
const REGISTER: &str = "register";

#[derive(Clone, Copy)]
pub enum Callee<'n> {
    Alias(&'n str),
    Member(&'n str),
    Global,
}

pub fn is_name(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

pub fn is_global_name(text: &str) -> bool {
    text.len() > NAME_PREFIX.len() && text.starts_with(NAME_PREFIX)
}

fn string_is(expr: &Expression<'_>, text: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::StringLiteral(s) if s.value == text)
}

fn typeof_of(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::Typeof && is_name(&unary.argument, name))
}

fn defined_test(test: &Expression<'_>, name: &str) -> bool {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return false;
    };
    match binary.operator {
        BinaryOperator::StrictInequality => {
            typeof_of(&binary.left, name) && string_is(&binary.right, "undefined")
        }
        BinaryOperator::GreaterThan => {
            string_is(&binary.left, "u") && typeof_of(&binary.right, name)
        }
        _ => false,
    }
}

pub fn global_chain(expr: &Expression<'_>) -> bool {
    let mut current = expr.without_parentheses();
    for name in GLOBAL_NAMES {
        let Expression::ConditionalExpression(conditional) = current else {
            return false;
        };
        if !defined_test(&conditional.test, name) || !is_name(&conditional.consequent, name) {
            return false;
        }
        current = conditional.alternate.without_parentheses();
    }
    matches!(current, Expression::ObjectExpression(object) if object.properties.is_empty())
}

pub fn global_object(expr: &Expression<'_>) -> bool {
    is_name(expr, GLOBAL_NAMES[0]) || global_chain(expr)
}

fn zero_wrapped<'k, 'a>(expr: &'k Expression<'a>) -> &'k Expression<'a> {
    match expr.without_parentheses() {
        Expression::SequenceExpression(sequence) => match sequence.expressions.as_slice() {
            [Expression::NumericLiteral(zero), inner] if zero.value == 0.0 => {
                inner.without_parentheses()
            }
            _ => expr,
        },
        other => other,
    }
}

fn register_object<'k, 'a>(callee: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match zero_wrapped(callee) {
        Expression::StaticMemberExpression(member)
            if !member.optional && member.property.name == REGISTER =>
        {
            Some(&member.object)
        }
        _ => None,
    }
}

pub fn global_slot(expr: &Expression<'_>) -> Option<String> {
    let expr = expr.without_parentheses();
    if expr
        .as_member_expression()
        .is_some_and(|member| member.optional())
    {
        return None;
    }
    let (object, name) = member_parts(expr)?;
    (global_object(object) && is_global_name(&name)).then(|| name.into_owned())
}

pub fn registers(callee: &Expression<'_>, how: Callee<'_>) -> Option<Option<String>> {
    match how {
        Callee::Alias(name) => is_name(callee, name).then_some(None),
        Callee::Member(require) => register_object(callee)
            .is_some_and(|object| is_name(object, require))
            .then_some(None),
        Callee::Global => Some(Some(global_slot(register_object(callee)?)?)),
    }
}
