use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use untwist_core::pass::{Cx, Safety};

use super::define_chain;
use super::identify::Table;
use super::shapes::Kind;
use super::spread::{shape, Gate};
use super::values::{
    into_inner, is_inert, is_skipped_source, is_string_like, sets_prototype, take_arguments,
    uses_super,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Semantics {
    Assign,
    Define,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Copy,
    DefineAll,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Skip,
    Inline,
    Chain,
    Spread,
}

struct Plan {
    actions: Vec<Action>,
    safety: Safety,
}

fn semantics(kind: Kind) -> Semantics {
    if kind == Kind::Assign {
        Semantics::Assign
    } else {
        Semantics::Define
    }
}

fn positions(kind: Kind, sources: usize) -> Option<Vec<Position>> {
    let every = |position| Some(vec![position; sources]);
    match (kind, sources) {
        (_, 0) => None,
        (Kind::Assign | Kind::ObjectSpreadEach, _) => every(Position::Copy),
        (Kind::ObjectSpread2, _) => Some(
            (1..=sources)
                .map(|index| {
                    if index % 2 == 1 {
                        Position::Copy
                    } else {
                        Position::DefineAll
                    }
                })
                .collect(),
        ),
        (Kind::SpreadValues, 1) => every(Position::Copy),
        (Kind::ObjectSpreadProps | Kind::SpreadProps, 1) => every(Position::DefineAll),
        _ => None,
    }
}

fn is_static_key(property: &ObjectProperty<'_>) -> bool {
    !property.computed
        || matches!(
            property.key,
            PropertyKey::StringLiteral(_) | PropertyKey::NumericLiteral(_)
        )
}

fn copies_as_data(member: &ObjectPropertyKind<'_>, semantics: Semantics) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = member else {
        return true;
    };
    if property.kind != PropertyKind::Init
        || sets_prototype(property)
        || uses_super(&property.value)
    {
        return false;
    }
    semantics == Semantics::Define
        || (is_static_key(property) && !property.key.is_specific_static_name("__proto__"))
}

fn defines_as_written(member: &ObjectPropertyKind<'_>) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = member else {
        return true;
    };
    !sets_prototype(property) && !uses_super(&property.value)
}

fn target_holds(target: &ObjectExpression<'_>, semantics: Semantics) -> bool {
    target.properties.iter().all(|member| match member {
        ObjectPropertyKind::ObjectProperty(property) => {
            !sets_prototype(property)
                && (semantics == Semantics::Define || property.kind == PropertyKind::Init)
        }
        ObjectPropertyKind::SpreadProperty(_) => true,
    })
}

fn chain_holds(expression: &Expression<'_>, table: &Table, scoping: &Scoping) -> bool {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return false;
    };
    if call.optional || call.arguments.len() != 3 {
        return false;
    }
    if table.kind_of_callee(&call.callee, scoping) != Some(Kind::DefineProperty) {
        return false;
    }
    let parts: Vec<&Expression<'_>> = call
        .arguments
        .iter()
        .filter_map(Argument::as_expression)
        .collect();
    let [object, key, value] = parts.as_slice() else {
        return false;
    };
    let base = match object.without_parentheses() {
        Expression::ObjectExpression(empty) => empty.properties.is_empty(),
        other => chain_holds(other, table, scoping),
    };
    let key_first = matches!(
        key.without_parentheses(),
        Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    ) || is_inert(value, scoping);
    base && key_first
}

fn copy_action(source: &Expression<'_>, semantics: Semantics, scoping: &Scoping) -> (Action, bool) {
    if is_skipped_source(source, scoping) {
        return (Action::Skip, false);
    }
    match source.without_parentheses() {
        Expression::ObjectExpression(object)
            if object
                .properties
                .iter()
                .all(|member| copies_as_data(member, semantics)) =>
        {
            (Action::Inline, false)
        }
        other if is_string_like(other) => (Action::Spread, false),
        _ => (Action::Spread, true),
    }
}

fn define_action(source: &Expression<'_>, table: &Table, scoping: &Scoping) -> Option<Action> {
    if is_skipped_source(source, scoping) {
        return Some(Action::Skip);
    }
    match source.without_parentheses() {
        Expression::ObjectExpression(object)
            if object.properties.iter().all(defines_as_written) =>
        {
            Some(Action::Inline)
        }
        other if chain_holds(other, table, scoping) => Some(Action::Chain),
        _ => None,
    }
}

fn plan(call: &CallExpression<'_>, kind: Kind, table: &Table, scoping: &Scoping) -> Option<Plan> {
    let semantics = semantics(kind);
    let arguments: Vec<&Expression<'_>> = call
        .arguments
        .iter()
        .filter_map(Argument::as_expression)
        .collect();
    let (target, sources) = arguments.split_first()?;
    let Expression::ObjectExpression(target) = target.without_parentheses() else {
        return None;
    };
    if !target_holds(target, semantics) {
        return None;
    }
    let positions = positions(kind, sources.len())?;
    let mut actions = Vec::with_capacity(sources.len());
    let mut runs_code = false;
    let mut safety = Safety::Safe;
    for (source, position) in sources.iter().zip(positions) {
        if runs_code && !is_inert(source, scoping) {
            return None;
        }
        let action = match position {
            Position::Copy => {
                let (action, assumed) = copy_action(source, semantics, scoping);
                if assumed {
                    safety = Safety::Unsafe;
                    runs_code = true;
                }
                action
            }
            Position::DefineAll => define_action(source, table, scoping)?,
        };
        actions.push(action);
    }
    Some(Plan { actions, safety })
}

fn members_of<'a>(expression: Expression<'a>, cx: &Cx<'a, '_>) -> AVec<'a, ObjectPropertyKind<'a>> {
    match into_inner(expression) {
        Expression::ObjectExpression(object) => object.unbox().properties,
        other => AVec::from_iter_in(
            [ObjectPropertyKind::new_spread_property(SPAN, other, cx)],
            cx,
        ),
    }
}

fn chain_members<'a>(
    expression: Expression<'a>,
    out: &mut AVec<'a, ObjectPropertyKind<'a>>,
    cx: &Cx<'a, '_>,
) {
    let Expression::CallExpression(call) = into_inner(expression) else {
        return;
    };
    let mut parts = call
        .unbox()
        .arguments
        .into_iter()
        .filter_map(|argument| Expression::try_from(argument).ok())
        .map(into_inner);
    let (Some(object), Some(key), Some(value)) = (parts.next(), parts.next(), parts.next()) else {
        return;
    };
    if matches!(object, Expression::CallExpression(_)) {
        chain_members(object, out, cx);
    }
    let key = PropertyKey::from(key);
    out.push(ObjectPropertyKind::new_object_property(
        SPAN,
        PropertyKind::Init,
        key,
        define_chain::unnamed(value, cx),
        false,
        false,
        true,
        cx,
    ));
}

fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::Assign => "assign",
        Kind::ObjectSpread2 => "object-spread2",
        Kind::ObjectSpreadEach => "object-spread-each",
        Kind::SpreadValues => "spread-values",
        _ => "spread-props",
    }
}

pub fn spread<'a>(
    node: &mut Expression<'a>,
    kind: Kind,
    table: &Table,
    gate: &Gate<'_>,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    let Expression::CallExpression(call) = &*node else {
        return None;
    };
    let plan = plan(call, kind, table, cx.scoping())?;
    if !gate.allows(plan.safety) {
        return None;
    }
    let (span, arguments) = take_arguments(node, cx)?;
    let mut arguments = arguments.into_iter();
    let mut members = members_of(arguments.next()?, cx);
    for (action, source) in plan.actions.into_iter().zip(arguments) {
        match action {
            Action::Skip => {}
            Action::Inline => members.extend(members_of(source, cx)),
            Action::Chain => chain_members(source, &mut members, cx),
            Action::Spread => {
                members.push(ObjectPropertyKind::new_spread_property(SPAN, source, cx));
            }
        }
    }
    *node = Expression::new_object_expression(span, members, cx);
    Some(shape(label(kind), plan.safety))
}
