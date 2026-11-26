use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator, UpdateOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::array_source::{proven_array, returnless, Context};
use super::for_of::{mentions, resolved_id, unlabelled, unlabelled_mut};
use super::identify::{resolved, Table};
use super::shapes::Kind;

pub const HELPER_SHAPE: &str = "for-of/loose-helper/unsafe";
pub const INDEXED_SHAPE: &str = "for-of/array-index/unsafe";

const ITERATOR_USES: usize = 1;
const STEP_USES: usize = 2;
const INDEX_USES: usize = 3;
const ARRAY_USES: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lead {
    Declared,
    Assigned,
}

#[derive(Clone, Copy)]
enum Form {
    Helper { iterator: SymbolId, step: SymbolId },
    Indexed { index: SymbolId, array: SymbolId },
}

impl Form {
    fn source_holder(self) -> SymbolId {
        match self {
            Form::Helper { iterator, .. } => iterator,
            Form::Indexed { array, .. } => array,
        }
    }

    fn is_item(self, expression: &Expression<'_>, scoping: &Scoping) -> bool {
        match (self, expression.without_parentheses()) {
            (Form::Helper { step, .. }, Expression::StaticMemberExpression(member)) => {
                !member.optional
                    && member.property.name.as_str() == "value"
                    && resolved(&member.object, scoping) == Some(step)
            }
            (Form::Indexed { index, array }, Expression::ComputedMemberExpression(member)) => {
                !member.optional
                    && resolved(&member.object, scoping) == Some(array)
                    && resolved(&member.expression, scoping) == Some(index)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
struct Plan {
    at: usize,
    form: Form,
    lead: Lead,
}

fn binding(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
    match &declarator.id {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn init_declaration<'k, 'a>(looped: &'k ForStatement<'a>) -> Option<&'k VariableDeclaration<'a>> {
    match looped.init.as_ref()? {
        ForStatementInit::VariableDeclaration(declaration)
            if !declaration.declare
                && matches!(
                    declaration.kind,
                    VariableDeclarationKind::Var | VariableDeclarationKind::Let
                ) =>
        {
            Some(declaration)
        }
        _ => None,
    }
}

fn helper_source<'k, 'a>(
    declarator: &'k VariableDeclarator<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<&'k Expression<'a>> {
    let Some(Expression::CallExpression(call)) = declarator.init.as_ref() else {
        return None;
    };
    let helper = !call.optional
        && table.kind_of_callee(&call.callee, scoping) == Some(Kind::ForOfIteratorLoose);
    match call.arguments.as_slice() {
        [argument] if helper => argument.as_expression(),
        _ => None,
    }
}

fn stepped(test: &Expression<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId)> {
    let Expression::UnaryExpression(not) = test.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(done) = not.argument.without_parentheses() else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = done.object.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(step) = &assign.left else {
        return None;
    };
    let Expression::CallExpression(call) = assign.right.without_parentheses() else {
        return None;
    };
    let shaped = not.operator == UnaryOperator::LogicalNot
        && !done.optional
        && done.property.name.as_str() == "done"
        && assign.operator == AssignmentOperator::Assign
        && !call.optional
        && call.arguments.is_empty();
    shaped.then_some(())?;
    Some((
        resolved(&call.callee, scoping)?,
        resolved_id(step, scoping)?,
    ))
}

fn others_hoistable(declaration: &VariableDeclaration<'_>, at: usize, skip: &[SymbolId]) -> bool {
    let others = declaration
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declarator)| binding(declarator).is_none_or(|symbol| !skip.contains(&symbol)));
    let mut any = false;
    for (index, declarator) in others {
        any = true;
        if index > at && declarator.init.is_some() {
            return false;
        }
    }
    !any || declaration.kind == VariableDeclarationKind::Var
}

fn helper_form<'k, 'a>(
    looped: &'k ForStatement<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<(Form, &'k Expression<'a>)> {
    if looped.update.is_some() {
        return None;
    }
    let (iterator, step) = stepped(looped.test.as_ref()?, scoping)?;
    let declaration = init_declaration(looped)?;
    let at = declaration
        .declarations
        .iter()
        .position(|declarator| binding(declarator) == Some(iterator))?;
    let source = helper_source(&declaration.declarations[at], table, scoping)?;
    let step_free = declaration
        .declarations
        .iter()
        .any(|declarator| binding(declarator) == Some(step) && declarator.init.is_none());
    (step_free && others_hoistable(declaration, at, &[iterator, step])).then_some(())?;
    Some((Form::Helper { iterator, step }, source))
}

fn indexed_test(test: &Expression<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId)> {
    let Expression::BinaryExpression(less) = test.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(length) = less.right.without_parentheses() else {
        return None;
    };
    let shaped = less.operator == BinaryOperator::LessThan
        && !length.optional
        && length.property.name.as_str() == "length";
    shaped.then_some(())?;
    Some((
        resolved(&less.left, scoping)?,
        resolved(&length.object, scoping)?,
    ))
}

fn increments(update: &Expression<'_>, index: SymbolId, scoping: &Scoping) -> bool {
    matches!(update.without_parentheses(), Expression::UpdateExpression(step)
        if step.operator == UpdateOperator::Increment
            && matches!(&step.argument, SimpleAssignmentTarget::AssignmentTargetIdentifier(id)
                if resolved_id(id, scoping) == Some(index)))
}

fn indexed_form<'k, 'a>(
    looped: &'k ForStatement<'a>,
    scoping: &Scoping,
) -> Option<(Form, &'k Expression<'a>)> {
    let (index, array) = indexed_test(looped.test.as_ref()?, scoping)?;
    if index == array || !increments(looped.update.as_ref()?, index, scoping) {
        return None;
    }
    let declaration = init_declaration(looped)?;
    let [first, second] = declaration.declarations.as_slice() else {
        return None;
    };
    let (counter, holder) = match (binding(first), binding(second)) {
        (Some(a), Some(b)) if a == index && b == array => (first, second),
        (Some(a), Some(b)) if a == array && b == index => (second, first),
        _ => return None,
    };
    let zero = matches!(&counter.init, Some(Expression::NumericLiteral(zero)) if zero.value == 0.0);
    let source = holder.init.as_ref()?;
    zero.then_some((Form::Indexed { index, array }, source))
}

fn counted(form: Form, scoping: &Scoping) -> bool {
    let uses = |symbol: SymbolId, expected: usize| {
        scoping.get_resolved_reference_ids(symbol).len() == expected
            && scoping.symbol_redeclarations(symbol).is_empty()
    };
    match form {
        Form::Helper { iterator, step } => uses(iterator, ITERATOR_USES) && uses(step, STEP_USES),
        Form::Indexed { index, array } => uses(index, INDEX_USES) && uses(array, ARRAY_USES),
    }
}

fn lead_of(first: &Statement<'_>, form: Form, scoping: &Scoping) -> Option<Lead> {
    match first {
        Statement::VariableDeclaration(declaration) if !declaration.declare => {
            let declarator = declaration.declarations.first()?;
            let item = declarator
                .init
                .as_ref()
                .is_some_and(|init| form.is_item(init, scoping));
            item.then_some(Lead::Declared)
        }
        Statement::ExpressionStatement(statement) => {
            let Expression::AssignmentExpression(assign) = &statement.expression else {
                return None;
            };
            let plain = assign.operator == AssignmentOperator::Assign
                && matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_))
                && form.is_item(&assign.right, scoping);
            plain.then_some(Lead::Assigned)
        }
        _ => None,
    }
}

fn captured(first: &Statement<'_>, lead: Lead, source: &Expression<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = first else {
        return false;
    };
    if lead != Lead::Declared || declaration.kind == VariableDeclarationKind::Var {
        return false;
    }
    let names: Vec<&str> = declaration.declarations[0]
        .id
        .get_binding_identifiers()
        .iter()
        .map(|id| id.name.as_str())
        .collect();
    mentions(source, &names)
}

fn plan_at(
    statements: &[Statement<'_>],
    at: usize,
    table: Option<&Table>,
    module: bool,
    scoping: &Scoping,
) -> Option<Plan> {
    let Statement::ForStatement(looped) = unlabelled(statements.get(at)?) else {
        return None;
    };
    looped.scope_id.get()?;
    let (form, source) = match table {
        Some(table) => helper_form(looped, table, scoping)?,
        None => indexed_form(looped, scoping)?,
    };
    if !counted(form, scoping) {
        return None;
    }
    let Statement::BlockStatement(body) = &looped.body else {
        return None;
    };
    let first = body.body.first()?;
    let lead = lead_of(first, form, scoping)?;
    if captured(first, lead, source) {
        return None;
    }
    let context = Context {
        statements,
        at,
        scoping,
        module,
    };
    let sound = match form {
        Form::Helper { .. } => returnless(&context, source),
        Form::Indexed { .. } => proven_array(&context, source),
    };
    sound.then_some(Plan { at, form, lead })
}

fn take_left<'a>(
    lead: Lead,
    body: &mut AVec<'a, Statement<'a>>,
    cx: &Cx<'a, '_>,
) -> Option<ForStatementLeft<'a>> {
    match lead {
        Lead::Declared => {
            let Statement::VariableDeclaration(declaration) = body.first_mut()? else {
                return None;
            };
            let mut declarator = declaration.declarations.remove(0);
            let kind = declaration.kind;
            if declaration.declarations.is_empty() {
                body.remove(0);
            }
            declarator.init = None;
            let declarations = AVec::from_iter_in([declarator], cx);
            Some(ForStatementLeft::VariableDeclaration(
                VariableDeclaration::boxed(SPAN, kind, declarations, false, cx),
            ))
        }
        Lead::Assigned => {
            let Statement::ExpressionStatement(statement) = body.remove(0) else {
                return None;
            };
            let Expression::AssignmentExpression(assign) = statement.unbox().expression else {
                return None;
            };
            Some(ForStatementLeft::from(assign.unbox().left))
        }
    }
}

fn take_source<'a>(
    declaration: &mut VariableDeclaration<'a>,
    form: Form,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let holder = form.source_holder();
    let at = declaration
        .declarations
        .iter()
        .position(|declarator| binding(declarator) == Some(holder))?;
    let init = declaration.declarations.get_mut(at)?.init.take()?;
    match form {
        Form::Indexed { .. } => Some(init),
        Form::Helper { .. } => {
            let Expression::CallExpression(mut call) = init else {
                return None;
            };
            let first = call.arguments.first_mut()?;
            let placeholder = Argument::from(Expression::new_null_literal(SPAN, cx));
            Expression::try_from(std::mem::replace(first, placeholder)).ok()
        }
    }
}

fn dropped(form: Form) -> [SymbolId; 2] {
    match form {
        Form::Helper { iterator, step } => [iterator, step],
        Form::Indexed { index, array } => [index, array],
    }
}

fn rewrite<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    plan: Plan,
    cx: &mut Cx<'a, '_>,
) -> Option<Statement<'a>> {
    let slot = statements.get_mut(plan.at)?;
    let Statement::ForStatement(looped) = unlabelled_mut(slot) else {
        return None;
    };
    let span = looped.span;
    let scope = looped.scope_id.get()?;
    let Some(ForStatementInit::VariableDeclaration(declaration)) = looped.init.as_mut() else {
        return None;
    };
    let source = take_source(declaration, plan.form, cx)?;
    let skip = dropped(plan.form);
    declaration
        .declarations
        .retain(|declarator| binding(declarator).is_none_or(|symbol| !skip.contains(&symbol)));
    let hoisted = (!declaration.declarations.is_empty()).then(|| {
        let declarations = std::mem::replace(&mut declaration.declarations, AVec::new_in(&*cx));
        Statement::VariableDeclaration(VariableDeclaration::boxed(
            SPAN,
            VariableDeclarationKind::Var,
            declarations,
            false,
            &*cx,
        ))
    });
    let Statement::BlockStatement(body) = &mut looped.body else {
        return None;
    };
    let left = take_left(plan.lead, &mut body.body, cx)?;
    let body = std::mem::replace(&mut looped.body, Statement::new_empty_statement(SPAN, &*cx));
    *unlabelled_mut(slot) =
        Statement::new_for_of_statement_with_scope_id(span, false, left, source, body, scope, &*cx);
    Some(hoisted.unwrap_or_else(|| Statement::new_empty_statement(SPAN, &*cx)))
}

pub fn shaped(statement: &Statement<'_>, table: Option<&Table>, scoping: &Scoping) -> bool {
    let Statement::ForStatement(looped) = unlabelled(statement) else {
        return false;
    };
    match table {
        Some(table) => helper_form(looped, table, scoping).is_some(),
        None => indexed_form(looped, scoping).is_some(),
    }
}

pub fn has_indexed_shape(statements: &[Statement<'_>], scoping: &Scoping) -> bool {
    statements.iter().any(|statement| {
        matches!(unlabelled(statement), Statement::ForStatement(looped)
            if indexed_form(looped, scoping).is_some())
    })
}

pub fn may_hold_site(statements: &[Statement<'_>]) -> bool {
    statements
        .iter()
        .any(|statement| matches!(unlabelled(statement), Statement::ForStatement(_)))
}

pub fn rewrite_statements<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    table: Option<&Table>,
    module: bool,
    cx: &mut Cx<'a, '_>,
) -> usize {
    if !may_hold_site(statements) {
        return 0;
    }
    let plans: Vec<Plan> = (0..statements.len())
        .filter_map(|at| plan_at(statements, at, table, module, cx.scoping()))
        .collect();
    let mut hoisted = Vec::new();
    let mut done = 0;
    for plan in &plans {
        let Some(statement) = rewrite(statements, *plan, cx) else {
            continue;
        };
        done += 1;
        if !matches!(statement, Statement::EmptyStatement(_)) {
            hoisted.push((plan.at, statement));
        }
    }
    for (at, statement) in hoisted.into_iter().rev() {
        statements.insert(at, statement);
    }
    if done > 0 {
        cx.binding_moved();
    }
    done
}
