use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;

use super::identify::{resolved, Table};
use super::shapes::Kind;

pub const SHAPE: &str = "for-of/iterator-helper/unsafe";

const ITERATOR_USES: usize = 4;
const STEP_USES: usize = 2;
const ERROR_USES: usize = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Head {
    Declared,
    Assigned,
    Step,
}

#[derive(Clone, Copy)]
struct Plan {
    at: usize,
    declared_at: usize,
    iterator: SymbolId,
    step: SymbolId,
    head: Head,
}

fn method_call<'k, 'a>(
    expression: &'k Expression<'a>,
    object: SymbolId,
    method: &str,
    scoping: &Scoping,
) -> Option<&'k CallExpression<'a>> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let matches = !call.optional
        && !member.optional
        && member.property.name.as_str() == method
        && resolved(&member.object, scoping) == Some(object);
    matches.then_some(call)
}

fn bare_call(expression: &Expression<'_>, object: SymbolId, method: &str, s: &Scoping) -> bool {
    method_call(expression, object, method, s).is_some_and(|call| call.arguments.is_empty())
}

fn only_expression<'k, 'a>(block: &'k BlockStatement<'a>) -> Option<&'k Expression<'a>> {
    match block.body.as_slice() {
        [Statement::ExpressionStatement(statement)] => Some(&statement.expression),
        _ => None,
    }
}

fn catch_forwards(handler: &CatchClause<'_>, iterator: SymbolId, scoping: &Scoping) -> bool {
    let Some(BindingPattern::BindingIdentifier(error)) = handler.param.as_ref().map(|p| &p.pattern)
    else {
        return false;
    };
    let Some(error) = error.symbol_id.get() else {
        return false;
    };
    let Some(call) = only_expression(&handler.body)
        .and_then(|expression| method_call(expression, iterator, "e", scoping))
    else {
        return false;
    };
    let forwarded = matches!(call.arguments.as_slice(),
        [Argument::Identifier(id)] if resolved_id(id, scoping) == Some(error));
    forwarded && scoping.get_resolved_reference_ids(error).len() == ERROR_USES
}

fn resolved_id(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn finally_closes(block: &BlockStatement<'_>, iterator: SymbolId, scoping: &Scoping) -> bool {
    only_expression(block).is_some_and(|expression| bare_call(expression, iterator, "f", scoping))
}

fn unlabelled<'k, 'a>(statement: &'k Statement<'a>) -> &'k Statement<'a> {
    match statement {
        Statement::LabeledStatement(labelled) => unlabelled(&labelled.body),
        other => other,
    }
}

fn unlabelled_mut<'k, 'a>(statement: &'k mut Statement<'a>) -> &'k mut Statement<'a> {
    match statement {
        Statement::LabeledStatement(labelled) => unlabelled_mut(&mut labelled.body),
        other => other,
    }
}

fn the_loop<'k, 'a>(block: &'k BlockStatement<'a>) -> Option<&'k ForStatement<'a>> {
    match block.body.as_slice() {
        [statement] => match unlabelled(statement) {
            Statement::ForStatement(it) => Some(it),
            _ => None,
        },
        _ => None,
    }
}

fn started_on(init: &ForStatementInit<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::CallExpression(call) = init.as_expression()?.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let starts = call.arguments.is_empty() && member.property.name.as_str() == "s";
    starts.then(|| resolved(&member.object, scoping)).flatten()
}

fn stepped(test: &Expression<'_>, iterator: SymbolId, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::UnaryExpression(not) = test.without_parentheses() else {
        return None;
    };
    if not.operator != UnaryOperator::LogicalNot {
        return None;
    }
    let Expression::StaticMemberExpression(done) = not.argument.without_parentheses() else {
        return None;
    };
    if done.optional || done.property.name.as_str() != "done" {
        return None;
    }
    let Expression::AssignmentExpression(assign) = done.object.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(step) = &assign.left else {
        return None;
    };
    let fresh = assign.operator == AssignmentOperator::Assign
        && bare_call(&assign.right, iterator, "n", scoping);
    fresh.then(|| resolved_id(step, scoping)).flatten()
}

fn is_step_value(expression: &Expression<'_>, step: SymbolId, scoping: &Scoping) -> bool {
    matches!(expression.without_parentheses(), Expression::StaticMemberExpression(member)
        if !member.optional
            && member.property.name.as_str() == "value"
            && resolved(&member.object, scoping) == Some(step))
}

fn leads(expression: &Expression<'_>, step: SymbolId, scoping: &Scoping) -> bool {
    if is_step_value(expression, step, scoping) {
        return true;
    }
    let next = match expression.without_parentheses() {
        Expression::StaticMemberExpression(member) => &member.object,
        Expression::ComputedMemberExpression(member) => &member.object,
        Expression::CallExpression(call) if !call.optional => {
            match call.callee.without_parentheses() {
                Expression::Identifier(_) => match call.arguments.first() {
                    Some(argument) if !argument.is_spread() => {
                        return argument
                            .as_expression()
                            .is_some_and(|first| leads(first, step, scoping))
                    }
                    _ => return false,
                },
                callee => callee,
            }
        }
        Expression::AssignmentExpression(assign)
            if matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_)) =>
        {
            &assign.right
        }
        Expression::AssignmentExpression(assign) => match &assign.left {
            AssignmentTarget::StaticMemberExpression(member) => &member.object,
            AssignmentTarget::ComputedMemberExpression(member) => &member.object,
            _ => return false,
        },
        Expression::BinaryExpression(binary) => &binary.left,
        Expression::LogicalExpression(logical) => &logical.left,
        Expression::ConditionalExpression(conditional) => &conditional.test,
        Expression::UnaryExpression(unary) => &unary.argument,
        Expression::SequenceExpression(sequence) => match sequence.expressions.first() {
            Some(first) => first,
            None => return false,
        },
        _ => return false,
    };
    leads(next, step, scoping)
}

fn statement_leads(statement: &Statement<'_>, step: SymbolId, scoping: &Scoping) -> bool {
    let lead = match statement {
        Statement::ExpressionStatement(it) => Some(&it.expression),
        Statement::VariableDeclaration(it) => it.declarations.first().and_then(|d| d.init.as_ref()),
        Statement::ReturnStatement(it) => it.argument.as_ref(),
        Statement::IfStatement(it) => Some(&it.test),
        Statement::ThrowStatement(it) => Some(&it.argument),
        _ => None,
    };
    lead.is_some_and(|expression| leads(expression, step, scoping))
}

fn head_of(first: &Statement<'_>, step: SymbolId, scoping: &Scoping) -> Option<Head> {
    match first {
        Statement::VariableDeclaration(declaration) if !declaration.declare => {
            if let [declarator] = declaration.declarations.as_slice() {
                if declarator
                    .init
                    .as_ref()
                    .is_some_and(|init| is_step_value(init, step, scoping))
                {
                    return Some(Head::Declared);
                }
            }
        }
        Statement::ExpressionStatement(statement) => {
            if let Expression::AssignmentExpression(assign) = &statement.expression {
                let plain = assign.operator == AssignmentOperator::Assign
                    && is_step_value(&assign.right, step, scoping);
                if let (true, AssignmentTarget::AssignmentTargetIdentifier(target)) =
                    (plain, &assign.left)
                {
                    if resolved_id(target, scoping) != Some(step) {
                        return Some(Head::Assigned);
                    }
                }
            }
        }
        _ => {}
    }
    statement_leads(first, step, scoping).then_some(Head::Step)
}

fn helper_source<'k, 'a>(
    declaration: &'k VariableDeclaration<'a>,
    iterator: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<&'k Expression<'a>> {
    let at = declaration.declarations.iter().position(|declarator| {
        matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
            if id.symbol_id.get() == Some(iterator))
    })?;
    let later_free = declaration.declarations[at + 1..]
        .iter()
        .all(|declarator| declarator.init.is_none());
    let Some(Expression::CallExpression(call)) = declaration.declarations[at].init.as_ref() else {
        return None;
    };
    let helper = later_free
        && !call.optional
        && table.kind_of_callee(&call.callee, scoping) == Some(Kind::ForOfIterator);
    match call.arguments.as_slice() {
        [argument] if helper && !argument.is_spread() => argument.as_expression(),
        _ => None,
    }
}

struct Names<'n> {
    names: &'n [&'n str],
    hit: bool,
}

impl<'a> oxc_ast_visit::Visit<'a> for Names<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.hit |= self.names.contains(&it.name.as_str());
    }
}

fn mentions(expression: &Expression<'_>, names: &[&str]) -> bool {
    let mut visitor = Names { names, hit: false };
    oxc_ast_visit::Visit::visit_expression(&mut visitor, expression);
    visitor.hit
}

fn captures(
    head: Head,
    first: &Statement<'_>,
    step: SymbolId,
    source: &Expression<'_>,
    scoping: &Scoping,
) -> bool {
    match (head, first) {
        (Head::Declared, Statement::VariableDeclaration(declaration)) => {
            if declaration.kind == VariableDeclarationKind::Var {
                return false;
            }
            let names: Vec<&str> = declaration
                .declarations
                .iter()
                .flat_map(|declarator| declarator.id.get_binding_identifiers())
                .map(|id| id.name.as_str())
                .collect();
            mentions(source, &names)
        }
        (Head::Step, _) => mentions(source, &[scoping.symbol_name(step)]),
        _ => false,
    }
}

fn holds(declaration: &VariableDeclaration<'_>, symbol: SymbolId) -> bool {
    declaration.declarations.iter().any(|declarator| {
        matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
            if id.symbol_id.get() == Some(symbol))
    })
}

fn declared_at(statements: &[Statement<'_>], at: usize, iterator: SymbolId) -> Option<usize> {
    let mut index = at.checked_sub(1)?;
    loop {
        let Statement::VariableDeclaration(declaration) = &statements[index] else {
            return None;
        };
        if holds(declaration, iterator) {
            return Some(index);
        }
        if declaration.declarations.iter().any(|d| d.init.is_some()) {
            return None;
        }
        index = index.checked_sub(1)?;
    }
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
    let iterator = started_on(looped.init.as_ref()?, scoping)?;
    let declared_at = declared_at(statements, at, iterator)?;
    let Statement::VariableDeclaration(declaration) = &statements[declared_at] else {
        return None;
    };
    let step = stepped(looped.test.as_ref()?, iterator, scoping)?;
    let closes = looped.update.is_none()
        && attempt
            .handler
            .as_ref()
            .is_some_and(|handler| catch_forwards(handler, iterator, scoping))
        && attempt
            .finalizer
            .as_ref()
            .is_some_and(|block| finally_closes(block, iterator, scoping));
    let Statement::BlockStatement(body) = &looped.body else {
        return None;
    };
    let first = body.body.first()?;
    let counted = scoping.get_resolved_reference_ids(iterator).len() == ITERATOR_USES
        && scoping.get_resolved_reference_ids(step).len() == STEP_USES
        && scoping.symbol_redeclarations(iterator).is_empty()
        && scoping.symbol_redeclarations(step).is_empty();
    if !(closes && counted) {
        return None;
    }
    let source = helper_source(declaration, iterator, table, scoping)?;
    let head = head_of(first, step, scoping)?;
    (!captures(head, first, step, source, scoping)).then_some(Plan {
        at,
        declared_at,
        iterator,
        step,
        head,
    })
}

fn take_source<'a>(
    declaration: &mut VariableDeclaration<'a>,
    iterator: SymbolId,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let declarator = declaration.declarations.iter_mut().find(|declarator| {
        matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
            if id.symbol_id.get() == Some(iterator))
    })?;
    let Some(Expression::CallExpression(call)) = declarator.init.as_mut() else {
        return None;
    };
    let first = call.arguments.first_mut()?;
    let placeholder = Argument::from(Expression::new_null_literal(SPAN, cx));
    Expression::try_from(std::mem::replace(first, placeholder)).ok()
}

struct StepValue<'s, 'a, 'c> {
    step: SymbolId,
    cx: &'s Cx<'a, 'c>,
    done: bool,
}

impl<'a> VisitMut<'a> for StepValue<'_, 'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.done {
            return;
        }
        let found = matches!(it, Expression::StaticMemberExpression(_))
            && is_step_value(it, self.step, self.cx.scoping());
        if let (true, Expression::StaticMemberExpression(member)) = (found, &mut *it) {
            let placeholder = Expression::new_null_literal(SPAN, self.cx);
            let object = std::mem::replace(&mut member.object, placeholder);
            *it = object;
            self.done = true;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

fn step_name<'a>(test: &Expression<'a>) -> Option<Ident<'a>> {
    let Expression::UnaryExpression(not) = test.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(done) = not.argument.without_parentheses() else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = done.object.without_parentheses() else {
        return None;
    };
    match &assign.left {
        AssignmentTarget::AssignmentTargetIdentifier(step) => Some(step.name),
        _ => None,
    }
}

fn left_of<'a>(
    head: Head,
    body: &mut AVec<'a, Statement<'a>>,
    (step, name): (SymbolId, Ident<'a>),
    cx: &Cx<'a, '_>,
) -> Option<ForStatementLeft<'a>> {
    match head {
        Head::Declared => {
            let Statement::VariableDeclaration(mut declaration) = body.remove(0) else {
                return None;
            };
            declaration.declarations.first_mut()?.init = None;
            Some(ForStatementLeft::VariableDeclaration(declaration))
        }
        Head::Assigned => {
            let Statement::ExpressionStatement(statement) = body.remove(0) else {
                return None;
            };
            let Expression::AssignmentExpression(assign) = statement.unbox().expression else {
                return None;
            };
            Some(ForStatementLeft::from(assign.unbox().left))
        }
        Head::Step => {
            let mut visitor = StepValue {
                step,
                cx,
                done: false,
            };
            visitor.visit_statement(body.first_mut()?);
            if !visitor.done {
                return None;
            }
            let id = BindingPattern::new_binding_identifier(SPAN, name, &*cx);
            let declarator = VariableDeclarator::new(SPAN, id, None, None, false, &*cx);
            let declarations = AVec::from_iter_in([declarator], &*cx);
            Some(ForStatementLeft::VariableDeclaration(
                VariableDeclaration::boxed(
                    SPAN,
                    VariableDeclarationKind::Const,
                    declarations,
                    false,
                    &*cx,
                ),
            ))
        }
    }
}

fn rewrite<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    plan: Plan,
    cx: &mut Cx<'a, '_>,
) -> Option<()> {
    let Statement::VariableDeclaration(declaration) = statements.get_mut(plan.declared_at)? else {
        return None;
    };
    let source = take_source(declaration, plan.iterator, cx)?;
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
    let name = step_name(for_loop.test.as_ref()?)?;
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

fn declared_free(statements: &[Statement<'_>], before: usize, step: SymbolId) -> bool {
    statements[..before].iter().any(|statement| {
        matches!(statement, Statement::VariableDeclaration(declaration)
            if declaration.declarations.iter().any(|declarator| declarator.init.is_none()
                && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                    if id.symbol_id.get() == Some(step))))
    })
}

fn dropped_declarator(declarator: &VariableDeclarator<'_>, dropped: &Dropped) -> bool {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return false;
    };
    id.symbol_id.get().is_some_and(|symbol| {
        dropped.iterators.contains(&symbol)
            || (declarator.init.is_none() && dropped.steps.contains(&symbol))
    })
}

fn prune<'a>(statements: &mut AVec<'a, Statement<'a>>, dropped: &Dropped) {
    let mut emptied = false;
    for statement in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement {
            let before = declaration.declarations.len();
            declaration
                .declarations
                .retain(|declarator| !dropped_declarator(declarator, dropped));
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

#[derive(Default)]
struct Dropped {
    iterators: FxHashSet<SymbolId>,
    steps: FxHashSet<SymbolId>,
}

pub fn may_hold_site(statements: &[Statement<'_>]) -> bool {
    statements
        .iter()
        .any(|statement| matches!(statement, Statement::TryStatement(attempt) if attempt.handler.is_some() && attempt.finalizer.is_some()))
}

pub fn has_site(statements: &[Statement<'_>], table: &Table, scoping: &Scoping) -> bool {
    may_hold_site(statements)
        && (1..statements.len()).any(|at| plan_at(statements, at, table, scoping).is_some())
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    table: &Table,
    cx: &mut Cx<'a, '_>,
) -> usize {
    if !may_hold_site(statements) {
        return 0;
    }
    let plans: Vec<Plan> = (1..statements.len())
        .filter_map(|at| plan_at(statements, at, table, cx.scoping()))
        .collect();
    if plans.is_empty() {
        return 0;
    }
    let mut dropped = Dropped::default();
    let mut done = 0;
    for plan in plans {
        let step_free = declared_free(statements, plan.at, plan.step);
        if rewrite(statements, plan, cx).is_none() {
            continue;
        }
        dropped.iterators.insert(plan.iterator);
        if step_free {
            dropped.steps.insert(plan.step);
        }
        done += 1;
    }
    if done > 0 {
        prune(statements, &dropped);
        cx.binding_moved();
    }
    done
}
