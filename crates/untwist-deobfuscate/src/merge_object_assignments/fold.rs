use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::respan::{fresh_spans, synthetic_spans_exhausted};
use untwist_core::pass::Cx;
use untwist_core::semantic::{is_constant_binding, write_reference_count};

use crate::bindings::{reference_symbol, resolved_symbol};
use crate::control_flow_object::single_declarator;
use crate::notes;
use crate::probe::expression_contains;

pub(super) const PROTO_KEY: &str = "__proto__";

pub(super) fn empty_object_binding(
    stmt: &Statement<'_>,
    scoping: &Scoping,
) -> Option<(SymbolId, bool)> {
    let (id, init) = single_declarator(stmt)?;
    let Expression::ObjectExpression(object) = init? else {
        return None;
    };
    if !object.properties.is_empty() {
        return None;
    }
    let symbol = id.symbol_id.get()?;
    Some((symbol, is_constant_binding(scoping, symbol)))
}

pub(super) fn plain_assignment<'k, 'a>(
    stmt: &'k Statement<'a>,
) -> Option<&'k AssignmentExpression<'a>> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign).then_some(&**assignment)
}

pub(crate) enum MergedKey<'k, 'a> {
    Named(&'k str),
    Numeric,
    Dynamic(&'k Expression<'a>),
}

pub(super) fn property_target<'k, 'a>(
    target: &'k AssignmentTarget<'a>,
) -> Option<(&'k Expression<'a>, MergedKey<'k, 'a>)> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) if !member.optional => Some((
            &member.object,
            MergedKey::Named(member.property.name.as_str()),
        )),
        AssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
            let key = match &member.expression {
                Expression::StringLiteral(literal) => MergedKey::Named(literal.value.as_str()),
                Expression::NumericLiteral(_) => MergedKey::Numeric,
                other => MergedKey::Dynamic(other),
            };
            Some((&member.object, key))
        }
        _ => None,
    }
}

fn is_inert_operand(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier(_)
        | Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        _ => false,
    }
}

pub(super) fn dynamic_key_symbol(key: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::Identifier(reference) = key else {
        return None;
    };
    resolved_symbol(reference, scoping)
}

fn runs_code(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::CallExpression(_)
            | Expression::NewExpression(_)
            | Expression::TaggedTemplateExpression(_)
            | Expression::ImportExpression(_)
            | Expression::AwaitExpression(_)
            | Expression::YieldExpression(_)
    )
}

fn is_unsafe_value(expr: &Expression<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    expression_contains(expr, |inner| {
        runs_code(inner) || reference_symbol(inner, scoping) == Some(symbol)
    })
}

pub(crate) struct KeyRule<'k> {
    pub(crate) conservative: bool,
    pub(crate) resolved: &'k FxHashSet<SymbolId>,
}

impl KeyRule<'_> {
    fn accepts(&self, key: &MergedKey<'_, '_>, scoping: &Scoping) -> Option<bool> {
        match key {
            MergedKey::Named(name) => (*name != PROTO_KEY).then_some(false),
            MergedKey::Numeric => Some(false),
            MergedKey::Dynamic(_) if self.conservative => Some(true),
            MergedKey::Dynamic(expr) => dynamic_key_symbol(expr, scoping)
                .filter(|symbol| self.resolved.contains(symbol))
                .map(|_| false),
        }
    }
}

fn folds_into_object(
    stmt: &Statement<'_>,
    symbol: SymbolId,
    scoping: &Scoping,
    rule: &KeyRule<'_>,
) -> Option<bool> {
    let assignment = plain_assignment(stmt)?;
    let (object, key) = property_target(&assignment.left)?;
    let adopted = rule.accepts(&key, scoping)?;
    if reference_symbol(object, scoping) != Some(symbol) {
        return None;
    }
    if reorders_key_conversion(&key, &assignment.right) {
        return None;
    }
    if is_unsafe_value(&assignment.right, symbol, scoping) {
        return None;
    }
    Some(adopted)
}

fn is_object_expression(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ParenthesizedExpression(inner) => is_object_expression(&inner.expression),
        Expression::ObjectExpression(_)
        | Expression::ArrayExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::ClassExpression(_)
        | Expression::NewExpression(_)
        | Expression::RegExpLiteral(_) => true,
        _ => false,
    }
}

fn reorders_key_conversion(key: &MergedKey<'_, '_>, value: &Expression<'_>) -> bool {
    match key {
        MergedKey::Dynamic(expr) => is_object_expression(expr) && !is_inert_operand(value),
        _ => false,
    }
}

fn merged_operands_are_inert(stmt: &Statement<'_>) -> bool {
    let Some(assignment) = plain_assignment(stmt) else {
        return false;
    };
    let Some((_, key)) = property_target(&assignment.left) else {
        return false;
    };
    let key_is_inert = match key {
        MergedKey::Dynamic(expr) => is_inert_operand(expr),
        _ => true,
    };
    key_is_inert && is_inert_operand(&assignment.right)
}

fn rebinds_the_object(stmt: &Statement<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    let Some(assignment) = plain_assignment(stmt) else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return false;
    };
    resolved_symbol(target, scoping) == Some(symbol)
}

fn every_write_follows_an_inert_run(
    stmts: &[Statement<'_>],
    fold: &Fold,
    scoping: &Scoping,
) -> bool {
    let run = fold.index + 1..fold.index + 1 + fold.count;
    if !stmts[run.clone()].iter().all(merged_operands_are_inert) {
        return false;
    }
    let after = stmts
        .iter()
        .skip(run.end)
        .filter(|stmt| rebinds_the_object(stmt, fold.symbol, scoping))
        .count();
    after == write_reference_count(scoping, fold.symbol)
}

struct Fold {
    index: usize,
    count: usize,
    symbol: SymbolId,
    adopted: bool,
}

fn plan_at(
    stmts: &[Statement<'_>],
    index: usize,
    scoping: &Scoping,
    rule: &KeyRule<'_>,
) -> Option<Fold> {
    let (symbol, constant) = empty_object_binding(stmts.get(index)?, scoping)?;
    let mut count = 0usize;
    let mut adopted = false;
    while let Some(taken) = stmts
        .get(index + 1 + count)
        .and_then(|stmt| folds_into_object(stmt, symbol, scoping, rule))
    {
        adopted |= taken;
        count += 1;
    }
    if count == 0 {
        return None;
    }
    let fold = Fold {
        index,
        count,
        symbol,
        adopted,
    };
    (constant || every_write_follows_an_inert_run(stmts, &fold, scoping)).then_some(fold)
}

fn plan_fold(stmts: &[Statement<'_>], scoping: &Scoping, rule: &KeyRule<'_>) -> Option<Fold> {
    (0..stmts.len()).find_map(|index| plan_at(stmts, index, scoping, rule))
}

fn merged_key<'a>(
    target: AssignmentTarget<'a>,
    cx: &Cx<'a, '_>,
) -> Option<(PropertyKey<'a>, bool)> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            let property = member.unbox().property;
            Some((
                PropertyKey::new_static_identifier(property.span, property.name, cx),
                false,
            ))
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            Some(match member.unbox().expression {
                Expression::StringLiteral(literal) => (PropertyKey::StringLiteral(literal), false),
                Expression::NumericLiteral(literal) => {
                    (PropertyKey::NumericLiteral(literal), false)
                }
                other => (PropertyKey::from(other), true),
            })
        }
        _ => None,
    }
}

fn merged_property<'a>(
    stmt: &mut Statement<'a>,
    span: Span,
    cx: &Cx<'a, '_>,
) -> Option<ObjectPropertyKind<'a>> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let placeholder = Expression::new_null_literal(SPAN, cx);
    let assignment = match std::mem::replace(&mut statement.expression, placeholder) {
        Expression::AssignmentExpression(assignment) => assignment.unbox(),
        restored => {
            statement.expression = restored;
            return None;
        }
    };
    let (key, computed) = merged_key(assignment.left, cx)?;
    Some(ObjectPropertyKind::new_object_property(
        span,
        PropertyKind::Init,
        key,
        assignment.right,
        false,
        false,
        computed,
        cx,
    ))
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

fn restore_object_literal<'a>(
    stmt: &mut Statement<'a>,
    object: ABox<'a, ObjectExpression<'a>>,
) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    let Some(declarator) = declaration.declarations.first_mut() else {
        return false;
    };
    declarator.init = Some(Expression::ObjectExpression(object));
    true
}

fn take_object_literal_with_spans<'a>(
    declaration: &mut Statement<'a>,
    count: usize,
    reserve: impl FnOnce(usize) -> Option<Vec<Span>>,
) -> Option<(ABox<'a, ObjectExpression<'a>>, Vec<Span>)> {
    let object = take_object_literal(declaration)?;
    match reserve(count) {
        Some(spans) => Some((object, spans)),
        None => {
            restore_object_literal(declaration, object);
            None
        }
    }
}

fn merge_run_into_declaration<'a>(run: &mut [Statement<'a>], cx: &Cx<'a, '_>) -> bool {
    let Some((declaration, assignments)) = run.split_first_mut() else {
        return false;
    };
    let Some((mut object, spans)) =
        take_object_literal_with_spans(declaration, assignments.len(), fresh_spans)
    else {
        return false;
    };
    for (statement, span) in assignments.iter_mut().zip(spans) {
        let Some(property) = merged_property(statement, span, cx) else {
            return false;
        };
        object.properties.push(property);
    }
    restore_object_literal(declaration, object)
}

fn commit_fold<'a>(stmts: &mut AVec<'a, Statement<'a>>, plan: &Fold, cx: &Cx<'a, '_>) -> bool {
    let end = plan.index + plan.count + 1;
    if stmts.len() < end || !merge_run_into_declaration(&mut stmts[plan.index..end], cx) {
        return false;
    }
    let run: Vec<Statement<'a>> = stmts.drain(plan.index..end).collect();
    let Some(declaration) = run.into_iter().next() else {
        return false;
    };
    stmts.insert(plan.index, declaration);
    true
}

pub(super) fn fold_assignments<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    folded: &mut FxHashSet<Span>,
    resolved: &FxHashSet<SymbolId>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let rule = KeyRule {
        conservative: !cx.opts.extended,
        resolved,
    };
    let Some(plan) = plan_fold(stmts, cx.scoping(), &rule) else {
        return 0;
    };
    let statement = stmts.get(plan.index).map(GetSpan::span);
    if synthetic_spans_exhausted() {
        cx.note(notes::MERGE_SPANS_EXHAUSTED, statement);
        return 0;
    }
    if !commit_fold(stmts, &plan, &*cx) {
        cx.note(
            notes::planned_and_applied_edits(super::PASS, plan.count, 0),
            None,
        );
        return 0;
    }
    if plan.adopted {
        cx.progress(notes::MERGE_ADOPTED_DYNAMIC_KEY, statement);
    }
    if let Some(statement) = statement {
        folded.insert(statement);
    }
    plan.count
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use oxc_allocator::Allocator;
    use oxc_span::Span;
    use untwist_core::ast_util::respan::{fresh_span_with, SpanCounter};
    use untwist_core::parse::parse_program;
    use untwist_core::print::{print, PrintStyle};

    use super::take_object_literal_with_spans;

    const DECLARATION: &str = "var o = { a: 1, b: [2, 3] };";

    fn reserve_from(counter: &SpanCounter, count: usize) -> Option<Vec<Span>> {
        (0..count).map(|_| fresh_span_with(counter)).collect()
    }

    #[test]
    fn a_failed_reservation_puts_the_literal_back_unchanged() {
        let alloc = Allocator::default();
        let mut parsed = parse_program(&alloc, DECLARATION);
        let before = print(&parsed.program, PrintStyle::Compact);
        let counter = SpanCounter::starting_at(u64::from(u32::MAX));
        assert!(counter.is_exhausted());
        let taken = take_object_literal_with_spans(&mut parsed.program.body[0], 2, |count| {
            reserve_from(&counter, count)
        });
        assert!(taken.is_none());
        assert_eq!(print(&parsed.program, PrintStyle::Compact), before);
    }

    #[test]
    fn no_span_is_reserved_when_there_is_no_literal_to_take() {
        let alloc = Allocator::default();
        let mut parsed = parse_program(&alloc, "var o = f();");
        let before = print(&parsed.program, PrintStyle::Compact);
        let reserved = Cell::new(false);
        let taken = take_object_literal_with_spans(&mut parsed.program.body[0], 2, |_| {
            reserved.set(true);
            None
        });
        assert!(taken.is_none());
        assert!(!reserved.get());
        assert_eq!(print(&parsed.program, PrintStyle::Compact), before);
    }

    #[test]
    fn a_granted_reservation_hands_over_the_literal_and_every_span() {
        let alloc = Allocator::default();
        let mut parsed = parse_program(&alloc, DECLARATION);
        let counter = SpanCounter::new();
        let taken = take_object_literal_with_spans(&mut parsed.program.body[0], 2, |count| {
            reserve_from(&counter, count)
        });
        assert_eq!(
            taken.map(|(object, spans)| (object.properties.len(), spans.len())),
            Some((2, 2))
        );
    }
}
