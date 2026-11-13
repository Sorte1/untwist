use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::operator::{AssignmentOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::for_of::{captures, head_of, left_of, resolved_id, the_loop, unlabelled_mut, Head};
use super::identify::{resolved, Table};
use super::shapes::Kind;

pub const SHAPE: &str = "for-of/values/unsafe";

const ITERATOR_USES: usize = 4;
const STEP_USES: usize = 5;
const ERROR_USES: usize = 3;
const RETURN_USES: usize = 2;

#[derive(Clone, Copy)]
struct Plan {
    at: usize,
    step: SymbolId,
    head: Head,
    error: SymbolId,
    closer: SymbolId,
}

fn is(expression: &Expression<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    resolved(expression, scoping) == Some(symbol)
}

fn member_of<'k, 'a>(
    expression: &'k Expression<'a>,
    name: &str,
) -> Option<&'k StaticMemberExpression<'a>> {
    match expression.without_parentheses() {
        Expression::StaticMemberExpression(member)
            if !member.optional && member.property.name.as_str() == name =>
        {
            Some(member)
        }
        _ => None,
    }
}

fn next_of(expression: &Expression<'_>, iterator: SymbolId, scoping: &Scoping) -> bool {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return false;
    };
    !call.optional
        && call.arguments.is_empty()
        && member_of(&call.callee, "next").is_some_and(|m| is(&m.object, iterator, scoping))
}

fn binding(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
    match &declarator.id {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn single_expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(it) => Some(&it.expression),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [inner] => single_expression(inner),
            _ => None,
        },
        _ => None,
    }
}

fn single_throw<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ThrowStatement(it) => Some(&it.argument),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [inner] => single_throw(inner),
            _ => None,
        },
        _ => None,
    }
}

struct Head2 {
    iterator: SymbolId,
    step: SymbolId,
    reset: Option<SymbolId>,
}

fn is_void(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::UnaryExpression(it) => {
            it.operator == UnaryOperator::Void
                && matches!(
                    it.argument.without_parentheses(),
                    Expression::NumericLiteral(_)
                )
        }
        Expression::Identifier(id) => {
            id.name.as_str() == "undefined"
                && id
                    .reference_id
                    .get()
                    .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
        }
        _ => false,
    }
}

fn values_call<'k, 'a>(
    init: &'k Expression<'a>,
    scoping: &Scoping,
) -> Option<(&'k CallExpression<'a>, Option<SymbolId>)> {
    match init.without_parentheses() {
        Expression::CallExpression(call) => Some((call, None)),
        Expression::SequenceExpression(sequence) => {
            let [Expression::AssignmentExpression(reset), Expression::CallExpression(call)] =
                sequence.expressions.as_slice()
            else {
                return None;
            };
            let AssignmentTarget::AssignmentTargetIdentifier(id) = &reset.left else {
                return None;
            };
            let cleared =
                reset.operator == AssignmentOperator::Assign && is_void(&reset.right, scoping);
            cleared.then(|| Some((&**call, Some(resolved_id(id, scoping)?))))?
        }
        _ => None,
    }
}

fn values_call_mut<'k, 'a>(init: &'k mut Expression<'a>) -> Option<&'k mut CallExpression<'a>> {
    match init {
        Expression::CallExpression(call) => Some(call),
        Expression::SequenceExpression(sequence) => match sequence.expressions.last_mut()? {
            Expression::CallExpression(call) => Some(call),
            _ => None,
        },
        _ => None,
    }
}

fn header(looped: &ForStatement<'_>, table: &Table, scoping: &Scoping) -> Option<Head2> {
    let Some(ForStatementInit::VariableDeclaration(init)) = &looped.init else {
        return None;
    };
    let [made, first] = init.declarations.as_slice() else {
        return None;
    };
    let iterator = binding(made)?;
    let step = binding(first)?;
    let (call, reset) = values_call(made.init.as_ref()?, scoping)?;
    let values = init.kind == VariableDeclarationKind::Var
        && !call.optional
        && matches!(call.arguments.as_slice(), [argument] if !argument.is_spread())
        && table.kind_of_callee(&call.callee, scoping) == Some(Kind::Values)
        && first
            .init
            .as_ref()
            .is_some_and(|init| next_of(init, iterator, scoping));
    let Expression::UnaryExpression(not) = looped.test.as_ref()?.without_parentheses() else {
        return None;
    };
    let tested = not.operator == UnaryOperator::LogicalNot
        && member_of(&not.argument, "done").is_some_and(|m| is(&m.object, step, scoping));
    let Expression::AssignmentExpression(update) = looped.update.as_ref()?.without_parentheses()
    else {
        return None;
    };
    let stepped = update.operator == AssignmentOperator::Assign
        && matches!(&update.left, AssignmentTarget::AssignmentTargetIdentifier(id)
            if resolved_id(id, scoping) == Some(step))
        && next_of(&update.right, iterator, scoping);
    (values && tested && stepped).then_some(Head2 {
        iterator,
        step,
        reset,
    })
}

fn records_error(handler: &CatchClause<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Some(BindingPattern::BindingIdentifier(caught)) =
        handler.param.as_ref().map(|p| &p.pattern)
    else {
        return None;
    };
    let caught = caught.symbol_id.get()?;
    let [statement] = handler.body.body.as_slice() else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = single_expression(statement)? else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(error) = &assign.left else {
        return None;
    };
    let Expression::ObjectExpression(record) = assign.right.without_parentheses() else {
        return None;
    };
    let wraps = matches!(record.properties.as_slice(),
        [ObjectPropertyKind::ObjectProperty(property)]
            if property.kind == PropertyKind::Init
                && !property.computed
                && !property.method
                && matches!(&property.key, PropertyKey::StaticIdentifier(key) if key.name.as_str() == "error")
                && is(&property.value, caught, scoping));
    let once = scoping.get_resolved_reference_ids(caught).len() == 1;
    (assign.operator == AssignmentOperator::Assign && wraps && once)
        .then(|| resolved_id(error, scoping))
        .flatten()
}

fn closes(statement: &Statement<'_>, ids: &Head2, scoping: &Scoping) -> Option<SymbolId> {
    let Statement::IfStatement(check) = statement else {
        return None;
    };
    if check.alternate.is_some() {
        return None;
    }
    let Expression::LogicalExpression(outer) = check.test.without_parentheses() else {
        return None;
    };
    let Expression::LogicalExpression(inner) = outer.left.without_parentheses() else {
        return None;
    };
    let Expression::UnaryExpression(not_done) = inner.right.without_parentheses() else {
        return None;
    };
    let Expression::AssignmentExpression(read) = outer.right.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(closer) = &read.left else {
        return None;
    };
    let closer = resolved_id(closer, scoping)?;
    let guarded = outer.operator == LogicalOperator::And
        && inner.operator == LogicalOperator::And
        && is(&inner.left, ids.step, scoping)
        && not_done.operator == UnaryOperator::LogicalNot
        && member_of(&not_done.argument, "done").is_some_and(|m| is(&m.object, ids.step, scoping))
        && read.operator == AssignmentOperator::Assign
        && member_of(&read.right, "return").is_some_and(|m| is(&m.object, ids.iterator, scoping));
    let Expression::CallExpression(call) = single_expression(&check.consequent)? else {
        return None;
    };
    let called = !call.optional
        && member_of(&call.callee, "call").is_some_and(|m| is(&m.object, closer, scoping))
        && matches!(call.arguments.as_slice(), [Argument::Identifier(id)]
            if resolved_id(id, scoping) == Some(ids.iterator));
    (guarded && called).then_some(closer)
}

fn rethrows(block: &BlockStatement<'_>, error: SymbolId, scoping: &Scoping) -> bool {
    let [Statement::IfStatement(check)] = block.body.as_slice() else {
        return false;
    };
    check.alternate.is_none()
        && is(&check.test, error, scoping)
        && single_throw(&check.consequent)
            .and_then(|thrown| member_of(thrown, "error"))
            .is_some_and(|m| is(&m.object, error, scoping))
}

fn cleanup(
    finalizer: &BlockStatement<'_>,
    ids: &Head2,
    error: SymbolId,
    scoping: &Scoping,
) -> Option<SymbolId> {
    let [Statement::TryStatement(inner)] = finalizer.body.as_slice() else {
        return None;
    };
    if inner.handler.is_some() || !rethrows(inner.finalizer.as_ref()?, error, scoping) {
        return None;
    }
    let [statement] = inner.block.body.as_slice() else {
        return None;
    };
    closes(statement, ids, scoping)
}

fn plan_at(
    statements: &[Statement<'_>],
    at: usize,
    table: &Table,
    scoping: &Scoping,
) -> Option<Plan> {
    let Statement::TryStatement(attempt) = statements.get(at)? else {
        return None;
    };
    let looped = the_loop(&attempt.block)?;
    looped.scope_id.get()?;
    let ids = header(looped, table, scoping)?;
    let error = records_error(attempt.handler.as_ref()?, scoping)?;
    let closer = cleanup(attempt.finalizer.as_ref()?, &ids, error, scoping)?;
    let counted = scoping.get_resolved_reference_ids(ids.iterator).len() == ITERATOR_USES
        && scoping.get_resolved_reference_ids(ids.step).len() == STEP_USES
        && ids.reset.is_none_or(|reset| reset == error)
        && scoping.get_resolved_reference_ids(error).len()
            == ERROR_USES + usize::from(ids.reset.is_some())
        && scoping.get_resolved_reference_ids(closer).len() == RETURN_USES
        && [ids.iterator, ids.step, error, closer]
            .iter()
            .all(|symbol| scoping.symbol_redeclarations(*symbol).is_empty());
    if !counted {
        return None;
    }
    let Statement::BlockStatement(body) = &looped.body else {
        return None;
    };
    let first = body.body.first()?;
    let Some(ForStatementInit::VariableDeclaration(init)) = &looped.init else {
        return None;
    };
    let (call, _) = values_call(init.declarations.first()?.init.as_ref()?, scoping)?;
    let source = call.arguments.first()?.as_expression()?;
    let head = head_of(first, ids.step, scoping)?;
    (!captures(head, first, ids.step, source, scoping)).then_some(Plan {
        at,
        step: ids.step,
        head,
        error,
        closer,
    })
}

fn rewrite<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    plan: Plan,
    cx: &mut Cx<'a, '_>,
) -> Option<()> {
    let placeholder = Statement::new_empty_statement(SPAN, &*cx);
    let Statement::TryStatement(attempt) =
        std::mem::replace(statements.get_mut(plan.at)?, placeholder)
    else {
        return None;
    };
    let mut attempt = attempt.unbox();
    let mut slot = attempt.block.body.pop()?;
    let looped = unlabelled_mut(&mut slot);
    let Statement::ForStatement(for_loop) = looped else {
        return None;
    };
    let span = for_loop.span;
    let scope = for_loop.scope_id.get()?;
    let Some(ForStatementInit::VariableDeclaration(init)) = for_loop.init.as_mut() else {
        return None;
    };
    let name = match &init.declarations.get(1)?.id {
        BindingPattern::BindingIdentifier(id) => id.name,
        _ => return None,
    };
    let call = values_call_mut(init.declarations.first_mut()?.init.as_mut()?)?;
    let first = call.arguments.first_mut()?;
    let filler = Argument::from(Expression::new_null_literal(SPAN, cx));
    let source = Expression::try_from(std::mem::replace(first, filler)).ok()?;
    let Statement::BlockStatement(body) = &mut for_loop.body else {
        return None;
    };
    let left = left_of(plan.head, &mut body.body, (plan.step, name), cx)?;
    let body = std::mem::replace(
        &mut for_loop.body,
        Statement::new_empty_statement(SPAN, &*cx),
    );
    *looped =
        Statement::new_for_of_statement_with_scope_id(span, false, left, source, body, scope, &*cx);
    *statements.get_mut(plan.at)? = slot;
    Some(())
}

fn unused_declarator(declarator: &VariableDeclarator<'_>, dropped: &[SymbolId]) -> bool {
    declarator.init.is_none() && binding(declarator).is_some_and(|symbol| dropped.contains(&symbol))
}

fn prune<'a>(statements: &mut AVec<'a, Statement<'a>>, dropped: &[SymbolId]) {
    let mut emptied = false;
    for statement in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement {
            let before = declaration.declarations.len();
            declaration
                .declarations
                .retain(|declarator| !unused_declarator(declarator, dropped));
            emptied |= before > 0 && declaration.declarations.is_empty();
        }
    }
    if emptied {
        statements.retain(|statement| {
            !matches!(statement, Statement::VariableDeclaration(declaration)
                if declaration.declarations.is_empty())
        });
    }
}

pub fn has_site(statements: &[Statement<'_>], table: &Table, scoping: &Scoping) -> bool {
    (0..statements.len()).any(|at| plan_at(statements, at, table, scoping).is_some())
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let plans: Vec<Plan> = (0..statements.len())
        .filter_map(|at| plan_at(statements, at, table, cx.scoping()))
        .collect();
    let mut dropped = Vec::new();
    for plan in plans {
        if rewrite(statements, plan, cx).is_some() {
            dropped.extend([plan.error, plan.closer]);
        }
    }
    if dropped.is_empty() {
        return 0;
    }
    let done = dropped.len() / 2;
    prune(statements, &dropped);
    cx.binding_moved();
    done
}
