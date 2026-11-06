use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};

use super::writes::member_method_name;

const SOURCE_DEFINERS: &[&str] = &["assign", "defineProperties"];

const DEFINER_NAMES: &[&str] = &[
    "defineProperty",
    "defineProperties",
    "assign",
    "set",
    "deleteProperty",
    "setPrototypeOf",
    "__defineGetter__",
    "__defineSetter__",
];

fn canonical_definer(key: &str) -> Option<&'static str> {
    DEFINER_NAMES.iter().copied().find(|name| *name == key)
}

fn is_definer_name(key: &str) -> bool {
    canonical_definer(key).is_some()
}

fn is_owner(scoping: &Scoping, object: &Expression<'_>) -> bool {
    let Expression::Identifier(id) = object.without_parentheses() else {
        return false;
    };
    matches!(id.name.as_str(), "Object" | "Reflect")
        && id
            .reference_id
            .get()
            .is_none_or(|r| scoping.get_reference(r).symbol_id().is_none())
}

pub(super) fn is_definer_read(scoping: &Scoping, object: &Expression<'_>, key: &str) -> bool {
    is_definer_name(key) && (key.starts_with("__define") || is_owner(scoping, object))
}

pub(super) fn direct_definer(
    scoping: &Scoping,
    callee: &Expression<'_>,
) -> Option<(&'static str, Span)> {
    let member = callee.without_parentheses().as_member_expression()?;
    let method = canonical_definer(&member_method_name(member)?)?;
    is_definer_read(scoping, member.object(), method).then(|| (method, member.span()))
}

fn is_fresh(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
    )
}

fn unlisted_source(argument: &Argument<'_>) -> bool {
    match argument
        .as_expression()
        .map(Expression::without_parentheses)
    {
        Some(Expression::ObjectExpression(object)) => object
            .properties
            .iter()
            .any(|property| matches!(property, ObjectPropertyKind::SpreadProperty(_))),
        _ => true,
    }
}

pub(super) fn unkeyed(method: &str, call: &CallExpression<'_>) -> bool {
    if call.arguments.iter().any(Argument::is_spread) {
        return true;
    }
    let target = call.arguments.first().and_then(Argument::as_expression);
    SOURCE_DEFINERS.contains(&method)
        && !target.is_some_and(is_fresh)
        && call.arguments.iter().skip(1).any(unlisted_source)
}

fn takes_a_definer(key: Option<Cow<'_, str>>) -> bool {
    key.is_none_or(|name| is_definer_name(&name))
}

pub(super) fn destructures_a_definer(scoping: &Scoping, it: &VariableDeclarator<'_>) -> bool {
    let (BindingPattern::ObjectPattern(pattern), Some(init)) = (&it.id, &it.init) else {
        return false;
    };
    is_owner(scoping, init)
        && (pattern.rest.is_some()
            || pattern.properties.iter().any(|property| {
                takes_a_definer(
                    property
                        .key
                        .static_name()
                        .and_then(|name| name.into_cow_str()),
                )
            }))
}

fn assigned_property_takes_a_definer(property: &AssignmentTargetProperty<'_>) -> bool {
    match property {
        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(id) => {
            is_definer_name(id.binding.name.as_str())
        }
        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => takes_a_definer(
            property
                .name
                .static_name()
                .and_then(|name| name.into_cow_str()),
        ),
    }
}

pub(super) fn assigns_a_definer(scoping: &Scoping, it: &AssignmentExpression<'_>) -> bool {
    let AssignmentTarget::ObjectAssignmentTarget(pattern) = &it.left else {
        return false;
    };
    is_owner(scoping, &it.right)
        && (pattern.rest.is_some()
            || pattern
                .properties
                .iter()
                .any(assigned_property_takes_a_definer))
}
