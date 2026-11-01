use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{Span, SPAN};
use oxc_str::Ident;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::ast_util::respan::{fresh_spans, fresh_spans_expression};
use untwist_core::pass::Cx;

use std::borrow::Cow;

use crate::notes;

use super::{accepted_keys, cff_value, object_declaration, single_declarator, CffValue};

struct KeyFold {
    index: usize,
    object: Span,
    keys: Vec<String>,
    duplicate: Option<String>,
}

fn key_assignment<'k, 'a>(
    stmt: &'k Statement<'a>,
    object: &str,
) -> Option<(Cow<'k, str>, CffValue<'k, 'a>)> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let target = assignment.left.as_member_expression()?.as_expression();
    let (target_object, name) = member_parts(target)?;
    if identifier_name(target_object) != Some(object) {
        return None;
    }
    Some((name, cff_value(&assignment.right)?))
}

fn alias_declaration(stmt: &Statement<'_>, object: &str) -> bool {
    let Some((_, init)) = single_declarator(stmt) else {
        return false;
    };
    init.and_then(identifier_name) == Some(object)
}

fn duplicate_key(declared: &[Cow<'_, str>], assigned: &[String]) -> Option<String> {
    let mut seen: FxHashSet<&str> = declared.iter().map(|key| key.as_ref()).collect();
    assigned
        .iter()
        .find(|key| !seen.insert(key.as_str()))
        .cloned()
}

fn plan_at(
    stmts: &[Statement<'_>],
    index: usize,
    scoping: &Scoping,
    extended: bool,
) -> Option<KeyFold> {
    let temp = object_declaration(stmts.get(index)?, extended)?;
    let mut assigned = Vec::new();
    while let Some(pair) = stmts
        .get(index + 1 + assigned.len())
        .and_then(|stmt| key_assignment(stmt, temp.name))
    {
        assigned.push(pair);
    }
    if assigned.is_empty() {
        return None;
    }
    if !accepted_keys(&assigned, extended) {
        return None;
    }
    if !stmts
        .get(index + 1 + assigned.len())
        .is_some_and(|stmt| alias_declaration(stmt, temp.name))
    {
        return None;
    }
    if scoping.get_resolved_reference_ids(temp.symbol).len() != assigned.len() + 1 {
        return None;
    }
    let keys: Vec<String> = assigned
        .into_iter()
        .map(|(key, _)| key.into_owned())
        .collect();
    Some(KeyFold {
        index,
        object: temp.object,
        duplicate: duplicate_key(&temp.keys, &keys),
        keys,
    })
}

fn plan_key_fold(stmts: &[Statement<'_>], scoping: &Scoping, extended: bool) -> Option<KeyFold> {
    let mut refused: Option<KeyFold> = None;
    for index in 0..stmts.len() {
        let Some(plan) = plan_at(stmts, index, scoping, extended) else {
            continue;
        };
        if plan.duplicate.is_none() {
            return Some(plan);
        }
        refused = refused.or(Some(plan));
    }
    refused
}

fn respan_declared_object(stmts: &mut [Statement<'_>], index: usize) -> bool {
    let Some(Statement::VariableDeclaration(declaration)) = stmts.get_mut(index) else {
        return false;
    };
    let Some(declarator) = declaration.declarations.first_mut() else {
        return false;
    };
    let Some(init) = declarator.init.as_mut() else {
        return false;
    };
    fresh_spans_expression(init)
}

fn assigned_property<'a>(
    key: &str,
    value: Expression<'a>,
    key_span: Span,
    property_span: Span,
    cx: &Cx<'a, '_>,
) -> ObjectPropertyKind<'a> {
    let key = PropertyKey::new_static_identifier(key_span, Ident::from_str_in(key, cx), cx);
    ObjectPropertyKind::new_object_property(
        property_span,
        PropertyKind::Init,
        key,
        value,
        false,
        false,
        false,
        cx,
    )
}

fn take_object_literal<'a>(stmt: &mut Statement<'a>) -> Option<ABox<'a, ObjectExpression<'a>>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let declarator = declaration.declarations.first_mut()?;
    match declarator.init.take()? {
        Expression::ObjectExpression(object) => Some(object),
        restored => {
            declarator.init = Some(restored);
            None
        }
    }
}

fn take_assigned_value<'a>(stmt: &mut Statement<'a>, cx: &Cx<'a, '_>) -> Option<Expression<'a>> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &mut statement.expression else {
        return None;
    };
    let placeholder = Expression::new_null_literal(SPAN, cx);
    Some(std::mem::replace(&mut assignment.right, placeholder))
}

fn give_alias_the_object<'a>(
    stmt: &mut Statement<'a>,
    object: ABox<'a, ObjectExpression<'a>>,
) -> Option<()> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let declarator = declaration.declarations.first_mut()?;
    declarator.init = Some(Expression::ObjectExpression(object));
    Some(())
}

fn fold_run_into_alias<'a>(run: &mut [Statement<'a>], keys: &[String], cx: &Cx<'a, '_>) -> bool {
    let Some((declaration, rest)) = run.split_first_mut() else {
        return false;
    };
    if rest.len() != keys.len() + 1 {
        return false;
    }
    let (Some(key_spans), Some(property_spans)) =
        (fresh_spans(keys.len()), fresh_spans(keys.len()))
    else {
        return false;
    };
    let Some(mut object) = take_object_literal(declaration) else {
        return false;
    };
    let (assignments, alias) = rest.split_at_mut(keys.len());
    let pairs = keys.iter().zip(assignments.iter_mut());
    for ((key, statement), (key_span, property_span)) in
        pairs.zip(key_spans.into_iter().zip(property_spans))
    {
        let Some(value) = take_assigned_value(statement, cx) else {
            return false;
        };
        object
            .properties
            .push(assigned_property(key, value, key_span, property_span, cx));
    }
    let Some(alias) = alias.first_mut() else {
        return false;
    };
    give_alias_the_object(alias, object).is_some()
}

fn commit_key_fold<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    plan: &KeyFold,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let end = plan.index + plan.keys.len() + 2;
    if stmts.len() < end || !fold_run_into_alias(&mut stmts[plan.index..end], &plan.keys, &*cx) {
        return false;
    }
    let mut run: Vec<Statement<'a>> = stmts.drain(plan.index..end).collect();
    let Some(alias) = run.pop() else {
        return false;
    };
    stmts.insert(plan.index, alias);
    true
}

pub(super) fn fold_key_assignments<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let Some(plan) = plan_key_fold(stmts, cx.scoping(), cx.opts.extended) else {
        return 0;
    };
    if let Some(duplicate) = &plan.duplicate {
        cx.note(
            notes::control_flow_duplicate_key(duplicate),
            Some(plan.object),
        );
        return 0;
    }
    if !respan_declared_object(stmts, plan.index) {
        cx.note(notes::CONTROL_FLOW_SPANS_EXHAUSTED, Some(plan.object));
        return 0;
    }
    if !commit_key_fold(stmts, &plan, cx) {
        cx.note(notes::CONTROL_FLOW_SHAPE_LOST, Some(plan.object));
        return 0;
    }
    plan.keys.len() + 2
}
