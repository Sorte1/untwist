use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::for_of::{captures, head_of, left_of, resolved_id, the_loop, unlabelled_mut, Head};
use super::identify::resolved;

pub const SHAPE: &str = "for-of/inline-protocol/unsafe";

const NORMAL_USES: usize = 3;
const FLAG_USES: usize = 2;
const ITERATOR_USES: usize = 3;
const STEP_USES: usize = 2;

#[derive(Clone, Copy)]
struct Names {
    normal: SymbolId,
    failed: SymbolId,
    error: SymbolId,
    iterator: SymbolId,
    step: SymbolId,
}

#[derive(Clone, Copy)]
struct Plan {
    at: usize,
    names: Names,
    head: Head,
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

fn binding(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
    match &declarator.id {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn global(expression: &Expression<'_>, name: &str, scoping: &Scoping) -> bool {
    matches!(expression.without_parentheses(), Expression::Identifier(id)
        if id.name.as_str() == name
            && id.reference_id.get().is_some_and(|r| scoping.get_reference(r).symbol_id().is_none()))
}

fn single<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Statement<'a>> {
    match statement {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [inner] => single(inner),
            _ => None,
        },
        other => Some(other),
    }
}

fn assigned(statement: &Statement<'_>, scoping: &Scoping) -> Option<(SymbolId, bool)> {
    let Statement::ExpressionStatement(it) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &it.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return None;
    };
    let truth = match assign.right.without_parentheses() {
        Expression::BooleanLiteral(literal) => literal.value,
        _ => return None,
    };
    (assign.operator == AssignmentOperator::Assign)
        .then(|| resolved_id(id, scoping).map(|symbol| (symbol, truth)))
        .flatten()
}

fn iterated<'k, 'a>(init: &'k Expression<'a>, scoping: &Scoping) -> Option<&'k Expression<'a>> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let Expression::ComputedMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let symbol_iterator = member_of(&member.expression, "iterator")
        .is_some_and(|key| global(&key.object, "Symbol", scoping));
    (!call.optional && !member.optional && call.arguments.is_empty() && symbol_iterator)
        .then_some(&member.object)
}

fn header(looped: &ForStatement<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId, SymbolId)> {
    let Some(ForStatementInit::VariableDeclaration(init)) = &looped.init else {
        return None;
    };
    let [made, step] = init.declarations.as_slice() else {
        return None;
    };
    if init.kind != VariableDeclarationKind::Var || step.init.is_some() {
        return None;
    }
    iterated(made.init.as_ref()?, scoping)?;
    let (iterator, step) = (binding(made)?, binding(step)?);
    let Expression::UnaryExpression(not) = looped.test.as_ref()?.without_parentheses() else {
        return None;
    };
    let Expression::AssignmentExpression(normal) = not.argument.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(normal_id) = &normal.left else {
        return None;
    };
    let normal_symbol = resolved_id(normal_id, scoping)?;
    let done = member_of(&normal.right, "done")?;
    let Expression::AssignmentExpression(next) = done.object.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(step_id) = &next.left else {
        return None;
    };
    let Expression::CallExpression(call) = next.right.without_parentheses() else {
        return None;
    };
    let stepped = not.operator == UnaryOperator::LogicalNot
        && normal.operator == AssignmentOperator::Assign
        && next.operator == AssignmentOperator::Assign
        && resolved_id(step_id, scoping) == Some(step)
        && !call.optional
        && call.arguments.is_empty()
        && member_of(&call.callee, "next").is_some_and(|m| is(&m.object, iterator, scoping));
    let Expression::AssignmentExpression(reset) = looped.update.as_ref()?.without_parentheses()
    else {
        return None;
    };
    let resets = matches!(&reset.left, AssignmentTarget::AssignmentTargetIdentifier(id)
            if resolved_id(id, scoping) == Some(normal_symbol))
        && matches!(reset.right.without_parentheses(), Expression::BooleanLiteral(it) if it.value);
    (stepped && resets).then_some((normal_symbol, iterator, step))
}

fn records(handler: &CatchClause<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId)> {
    let Some(BindingPattern::BindingIdentifier(caught)) =
        handler.param.as_ref().map(|p| &p.pattern)
    else {
        return None;
    };
    let caught = caught.symbol_id.get()?;
    let [flag, kept] = handler.body.body.as_slice() else {
        return None;
    };
    let (failed, truth) = assigned(flag, scoping)?;
    let Statement::ExpressionStatement(it) = kept else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &it.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(error) = &assign.left else {
        return None;
    };
    let fits = truth
        && assign.operator == AssignmentOperator::Assign
        && is(&assign.right, caught, scoping)
        && scoping.get_resolved_reference_ids(caught).len() == 1;
    fits.then(|| Some((failed, resolved_id(error, scoping)?)))?
}

fn has_return(test: &Expression<'_>, iterator: SymbolId, scoping: &Scoping) -> bool {
    match test.without_parentheses() {
        Expression::BinaryExpression(binary) => {
            binary.operator == BinaryOperator::Inequality
                && matches!(
                    binary.right.without_parentheses(),
                    Expression::NullLiteral(_)
                )
                && member_of(&binary.left, "return")
                    .is_some_and(|m| is(&m.object, iterator, scoping))
        }
        other => member_of(other, "return").is_some_and(|m| is(&m.object, iterator, scoping)),
    }
}

fn closes(statement: &Statement<'_>, names: &Names, scoping: &Scoping) -> bool {
    let Some(Statement::IfStatement(check)) = single(statement) else {
        return false;
    };
    let Expression::LogicalExpression(test) = check.test.without_parentheses() else {
        return false;
    };
    let Expression::UnaryExpression(not) = test.left.without_parentheses() else {
        return false;
    };
    let guarded = check.alternate.is_none()
        && test.operator == LogicalOperator::And
        && not.operator == UnaryOperator::LogicalNot
        && is(&not.argument, names.normal, scoping)
        && has_return(&test.right, names.iterator, scoping);
    let Some(Statement::ExpressionStatement(call)) = single(&check.consequent) else {
        return false;
    };
    let Expression::CallExpression(call) = &call.expression else {
        return false;
    };
    guarded
        && !call.optional
        && call.arguments.is_empty()
        && member_of(&call.callee, "return").is_some_and(|m| is(&m.object, names.iterator, scoping))
}

fn rethrows(block: &BlockStatement<'_>, names: &Names, scoping: &Scoping) -> bool {
    let [statement] = block.body.as_slice() else {
        return false;
    };
    let Some(Statement::IfStatement(check)) = single(statement) else {
        return false;
    };
    let Some(Statement::ThrowStatement(thrown)) = single(&check.consequent) else {
        return false;
    };
    check.alternate.is_none()
        && is(&check.test, names.failed, scoping)
        && is(&thrown.argument, names.error, scoping)
}

fn cleanup(finalizer: &BlockStatement<'_>, names: &Names, scoping: &Scoping) -> bool {
    let [Statement::TryStatement(inner)] = finalizer.body.as_slice() else {
        return false;
    };
    let [statement] = inner.block.body.as_slice() else {
        return false;
    };
    inner.handler.is_none()
        && inner
            .finalizer
            .as_ref()
            .is_some_and(|block| rethrows(block, names, scoping))
        && closes(statement, names, scoping)
}

fn literal_init(
    declarator: &VariableDeclarator<'_>,
    expected: Option<bool>,
    scoping: &Scoping,
) -> bool {
    match (
        declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses),
        expected,
    ) {
        (Some(Expression::BooleanLiteral(literal)), Some(value)) => literal.value == value,
        (Some(init), None) => {
            global(init, "undefined", scoping)
                || matches!(init, Expression::UnaryExpression(it)
                    if it.operator == UnaryOperator::Void
                        && matches!(it.argument.without_parentheses(), Expression::NumericLiteral(_)))
        }
        (None, None) => true,
        _ => false,
    }
}

fn flags_before(statements: &[Statement<'_>], at: usize, names: &Names, scoping: &Scoping) -> bool {
    let wanted = [
        (names.normal, Some(true)),
        (names.failed, Some(false)),
        (names.error, None),
    ];
    wanted.iter().all(|(symbol, value)| {
        statements[..at].iter().any(|statement| {
            matches!(statement, Statement::VariableDeclaration(declaration)
            if declaration.declarations.iter().any(|declarator| {
                binding(declarator) == Some(*symbol) && literal_init(declarator, *value, scoping)
            }))
        })
    })
}

fn plan_at(statements: &[Statement<'_>], at: usize, scoping: &Scoping) -> Option<Plan> {
    let Statement::TryStatement(attempt) = statements.get(at)? else {
        return None;
    };
    let looped = the_loop(&attempt.block)?;
    looped.scope_id.get()?;
    let (normal, iterator, step) = header(looped, scoping)?;
    let (failed, error) = records(attempt.handler.as_ref()?, scoping)?;
    let names = Names {
        normal,
        failed,
        error,
        iterator,
        step,
    };
    let counted = scoping.get_resolved_reference_ids(normal).len() == NORMAL_USES
        && scoping.get_resolved_reference_ids(failed).len() == FLAG_USES
        && scoping.get_resolved_reference_ids(error).len() == FLAG_USES
        && scoping.get_resolved_reference_ids(iterator).len() == ITERATOR_USES
        && scoping.get_resolved_reference_ids(step).len() == STEP_USES
        && [normal, failed, error, iterator, step]
            .iter()
            .all(|symbol| scoping.symbol_redeclarations(*symbol).is_empty());
    let closed = cleanup(attempt.finalizer.as_ref()?, &names, scoping);
    if !(counted && closed && flags_before(statements, at, &names, scoping)) {
        return None;
    }
    let Statement::BlockStatement(body) = &looped.body else {
        return None;
    };
    let first = body.body.first()?;
    let Some(ForStatementInit::VariableDeclaration(init)) = &looped.init else {
        return None;
    };
    let source = iterated(init.declarations.first()?.init.as_ref()?, scoping)?;
    let head = head_of(first, step, scoping)?;
    (!captures(head, first, step, source, scoping)).then_some(Plan { at, names, head })
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
    let Some(Expression::CallExpression(call)) = init.declarations.first_mut()?.init.as_mut()
    else {
        return None;
    };
    let Expression::ComputedMemberExpression(member) = &mut call.callee else {
        return None;
    };
    let filler = Expression::new_null_literal(SPAN, cx);
    let source = std::mem::replace(&mut member.object, filler);
    let Statement::BlockStatement(body) = &mut for_loop.body else {
        return None;
    };
    let left = left_of(plan.head, &mut body.body, (plan.names.step, name), cx)?;
    let body = std::mem::replace(
        &mut for_loop.body,
        Statement::new_empty_statement(SPAN, &*cx),
    );
    *looped =
        Statement::new_for_of_statement_with_scope_id(span, false, left, source, body, scope, &*cx);
    *statements.get_mut(plan.at)? = slot;
    Some(())
}

fn prune<'a>(statements: &mut AVec<'a, Statement<'a>>, dropped: &[SymbolId]) {
    let mut emptied = false;
    for statement in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement {
            let before = declaration.declarations.len();
            declaration.declarations.retain(|declarator| {
                !binding(declarator).is_some_and(|symbol| dropped.contains(&symbol))
            });
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

pub fn may_hold_site(statements: &[Statement<'_>]) -> bool {
    statements.iter().any(|statement| {
        matches!(statement, Statement::TryStatement(attempt)
        if attempt.handler.is_some() && attempt.finalizer.is_some())
    })
}

pub fn has_site(statements: &[Statement<'_>], scoping: &Scoping) -> bool {
    may_hold_site(statements)
        && (0..statements.len()).any(|at| plan_at(statements, at, scoping).is_some())
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    if !may_hold_site(statements) {
        return 0;
    }
    let plans: Vec<Plan> = (0..statements.len())
        .filter_map(|at| plan_at(statements, at, cx.scoping()))
        .collect();
    let mut dropped = Vec::new();
    let mut done = 0;
    for plan in plans {
        if rewrite(statements, plan, cx).is_some() {
            dropped.extend([plan.names.normal, plan.names.failed, plan.names.error]);
            done += 1;
        }
    }
    if done > 0 {
        prune(statements, &dropped);
        cx.binding_moved();
    }
    done
}
