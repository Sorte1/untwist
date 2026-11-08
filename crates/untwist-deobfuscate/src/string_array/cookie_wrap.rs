use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::Cx;

use super::array_declaration;
use super::checksum::parsed_statement;
use super::cookie_minified::countdown_declaration;
use super::inlined::used_only_in;
use super::legacy::is_countdown;
use crate::stamp::declaration_symbol;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Assigned,
    Flagged,
    Block,
}

struct Plan {
    array: usize,
    alias: usize,
    rotator: usize,
    cookie: usize,
    form: Form,
    alias_name: String,
    array_name: String,
    rotator_name: String,
    assigned: bool,
    holder: Option<String>,
    dropped: Vec<SymbolId>,
}

fn resolved(scoping: &Scoping, reference: &IdentifierReference<'_>) -> Option<SymbolId> {
    scoping
        .get_reference(reference.reference_id.get()?)
        .symbol_id()
}

fn inert(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.kind == VariableDeclarationKind::Var
                && declaration
                    .declarations
                    .iter()
                    .all(|declarator| declarator.init.is_none())
        }
        _ => false,
    }
}

fn alias_of(stmt: &Statement<'_>, scoping: &Scoping, array: SymbolId) -> Option<SymbolId> {
    match stmt {
        Statement::ExpressionStatement(stmt) => {
            let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses()
            else {
                return None;
            };
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
                return None;
            };
            let Expression::Identifier(source) = assignment.right.without_parentheses() else {
                return None;
            };
            let aliases = assignment.operator == AssignmentOperator::Assign
                && resolved(scoping, source) == Some(array);
            aliases.then(|| resolved(scoping, target)).flatten()
        }
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let Some(Expression::Identifier(source)) =
                declarator.init.as_ref().map(Expression::without_parentheses)
            else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let aliases = declaration.kind == VariableDeclarationKind::Var
                && resolved(scoping, source) == Some(array);
            aliases.then(|| id.symbol_id.get()).flatten()
        }
        _ => None,
    }
}

fn assigned_countdown<'k>(stmt: &'k Statement<'_>, alias: &str) -> Option<&'k IdentifierReference<'k>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let counts = matches!(assignment.right.without_parentheses(),
        Expression::FunctionExpression(function) if is_countdown(function, alias));
    (assignment.operator == AssignmentOperator::Assign && counts && target.name != alias)
        .then_some(&**target)
}

fn holder_test<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let (object, name) = member_parts(call.callee.without_parentheses())?;
    let Expression::AssignmentExpression(assignment) = object.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(holder) = &assignment.left else {
        return None;
    };
    let shaped = name == "updateCookie"
        && call.arguments.is_empty()
        && assignment.operator == AssignmentOperator::Assign
        && matches!(assignment.right.without_parentheses(), Expression::ObjectExpression(_));
    shaped.then_some(&**holder)
}

fn flagged<'k, 'a>(
    choice: &'k ConditionalExpression<'a>,
) -> Option<(&'k IdentifierReference<'a>, &'k IdentifierReference<'a>)> {
    let Expression::AssignmentExpression(assignment) = choice.test.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(flag) = &assignment.left else {
        return None;
    };
    let Expression::ConditionalExpression(inner) = choice.consequent.without_parentheses() else {
        return None;
    };
    let rechecked = identifier_name(&inner.test) == Some(flag.name.as_str());
    let holder = holder_test(&assignment.right)?;
    (assignment.operator == AssignmentOperator::Assign && rechecked).then_some((&**flag, holder))
}

fn cookie_form<'k, 'a>(
    stmt: &'k Statement<'a>,
) -> Option<(Form, Vec<&'k IdentifierReference<'a>>)> {
    match stmt {
        Statement::BlockStatement(_) => Some((Form::Block, Vec::new())),
        Statement::ExpressionStatement(stmt) => {
            let Expression::ConditionalExpression(choice) = stmt.expression.without_parentheses()
            else {
                return None;
            };
            if let Some(holder) = holder_test(&choice.test) {
                return Some((Form::Assigned, vec![holder]));
            }
            let (flag, holder) = flagged(choice)?;
            Some((Form::Flagged, vec![holder, flag]))
        }
        _ => None,
    }
}

struct Contextual {
    found: bool,
}

impl<'a> VisitJs<'a> for Contextual {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.found = true;
    }

    fn visit_await_expression(&mut self, _it: &AwaitExpression<'a>) {
        self.found = true;
    }

    fn visit_yield_expression(&mut self, _it: &YieldExpression<'a>) {
        self.found = true;
    }

    fn visit_return_statement(&mut self, _it: &ReturnStatement<'a>) {
        self.found = true;
    }

    fn visit_break_statement(&mut self, _it: &BreakStatement<'a>) {
        self.found = true;
    }

    fn visit_continue_statement(&mut self, _it: &ContinueStatement<'a>) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

fn movable(stmt: &Statement<'_>) -> bool {
    let mut contextual = Contextual { found: false };
    contextual.visit_statement(stmt);
    !contextual.found
}

struct BlockVars<'s> {
    found: &'s mut Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for BlockVars<'_> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            for declarator in &it.declarations {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                    self.found.extend(id.symbol_id.get());
                }
            }
        }
        walk_js::walk_variable_declaration(self, it);
    }
}

fn block_vars(stmt: &Statement<'_>) -> Vec<SymbolId> {
    let mut found = Vec::new();
    BlockVars { found: &mut found }.visit_statement(stmt);
    found
}

struct Rotator {
    at: usize,
    name: String,
    symbol: SymbolId,
    assigned: bool,
}

fn rotator_of(
    body: &[Statement<'_>],
    (array, alias): (usize, usize),
    alias_name: &str,
    scoping: &Scoping,
) -> Option<Rotator> {
    let next = body.get(alias + 1)?;
    if let Some(target) = assigned_countdown(next, alias_name) {
        return Some(Rotator {
            at: alias + 1,
            name: target.name.to_string(),
            symbol: resolved(scoping, target)?,
            assigned: true,
        });
    }
    let declared = |at: usize| {
        let Statement::FunctionDeclaration(function) = &body[at] else {
            return None;
        };
        countdown_declaration(&body[at], alias_name)?;
        let id = function.id.as_ref()?;
        Some(Rotator {
            at,
            name: id.name.to_string(),
            symbol: id.symbol_id.get()?,
            assigned: false,
        })
    };
    declared(alias + 1).or_else(|| (array + 1..alias).find_map(declared))
}

fn plan_at(body: &[Statement<'_>], array: usize, scoping: &Scoping) -> Option<Plan> {
    let (array_name, length) = array_declaration(&body[array])?;
    let array_symbol = declaration_symbol(&body[array])?;
    if length == 0 || scoping.symbol_scope_id(array_symbol) == scoping.root_scope_id() {
        return None;
    }
    let alias = array + 1 + body.get(array + 1..)?.iter().take_while(|stmt| inert(stmt)).count();
    let alias_symbol = alias_of(body.get(alias)?, scoping, array_symbol)?;
    let alias_name = scoping.symbol_name(alias_symbol);
    let rotator = rotator_of(body, (array, alias), alias_name, scoping)?;
    let cookie = if rotator.at == alias + 1 { alias + 2 } else { alias + 1 };
    let (form, holders) = cookie_form(body.get(cookie)?)?;
    let holder_symbols: Vec<SymbolId> = holders
        .iter()
        .map(|holder| resolved(scoping, holder))
        .collect::<Option<_>>()?;
    let rotator_uses: Vec<usize> = if rotator.assigned {
        vec![rotator.at, cookie]
    } else {
        vec![cookie]
    };
    let in_statements = |symbol: SymbolId, at: &[usize]| {
        used_only_in(scoping, symbol, at.iter().map(|index| &body[*index]))
    };
    let mut owned = holder_symbols.clone();
    if form == Form::Block {
        owned.extend(block_vars(&body[cookie]));
    }
    let sealed = in_statements(alias_symbol, &[alias, rotator.at])
        && in_statements(rotator.symbol, &rotator_uses)
        && owned.iter().all(|symbol| in_statements(*symbol, &[cookie]))
        && holder_symbols
            .iter()
            .skip(1)
            .all(|flag| scoping.get_resolved_reference_ids(*flag).len() == 2)
        && movable(&body[cookie]);
    let original = [array, alias, rotator.at, cookie]
        .iter()
        .all(|index| body[*index].span().start < SYNTHETIC_BASE);
    let mut dropped = vec![alias_symbol];
    dropped.extend(holder_symbols.iter().copied());
    if rotator.assigned {
        dropped.push(rotator.symbol);
    }
    (sealed && original).then(|| Plan {
        array,
        alias,
        rotator: rotator.at,
        cookie,
        form,
        alias_name: alias_name.to_string(),
        array_name: array_name.to_string(),
        rotator_name: rotator.name,
        assigned: rotator.assigned,
        holder: holders.first().map(|holder| holder.name.to_string()),
        dropped,
    })
}

fn plan(body: &[Statement<'_>], scoping: &Scoping) -> Option<Plan> {
    (0..body.len()).find_map(|index| plan_at(body, index, scoping))
}

fn declared_rotator<'a>(stmt: Statement<'a>, plan: &Plan, cx: &Cx<'a, '_>) -> Option<Statement<'a>> {
    if !plan.assigned {
        return Some(stmt);
    }
    let Statement::ExpressionStatement(mut stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses_mut()
    else {
        return None;
    };
    let Expression::FunctionExpression(mut function) = assignment.right.take_in(cx) else {
        return None;
    };
    let Some(Statement::FunctionDeclaration(mut named)) =
        parsed_statement(&format!("function {}() {{}}", plan.rotator_name), cx)
    else {
        return None;
    };
    function.r#type = FunctionType::FunctionDeclaration;
    function.id = named.id.take();
    Some(Statement::FunctionDeclaration(function))
}

fn simplified<'a>(stmt: &mut Statement<'a>, cx: &Cx<'a, '_>) -> Option<()> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::ConditionalExpression(choice) = stmt.expression.without_parentheses_mut() else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = choice.test.without_parentheses_mut() else {
        return None;
    };
    let test = assignment.right.take_in(cx);
    let Expression::ConditionalExpression(inner) = choice.consequent.without_parentheses_mut()
    else {
        return None;
    };
    let read = inner.consequent.take_in(cx);
    choice.test = test;
    choice.consequent = read;
    Some(())
}

fn without_bare<'a>(it: &mut AVec<'a, Statement<'a>>, dropped: &[SymbolId]) {
    let gone = |declarator: &VariableDeclarator<'_>| {
        declarator.init.is_none()
            && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                if id.symbol_id.get().is_some_and(|symbol| dropped.contains(&symbol)))
    };
    for stmt in it.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = stmt {
            if declaration.kind == VariableDeclarationKind::Var {
                declaration.declarations.retain(|declarator| !gone(declarator));
            }
        }
    }
    it.retain(|stmt| {
        !matches!(stmt, Statement::VariableDeclaration(declaration)
            if declaration.declarations.is_empty())
    });
}

fn wrapper_body<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut AVec<'a, Statement<'a>>> {
    let Statement::ExpressionStatement(outer) = statement else {
        return None;
    };
    let Expression::UnaryExpression(unary) = &mut outer.expression else {
        return None;
    };
    let Expression::CallExpression(call) = unary.argument.without_parentheses_mut() else {
        return None;
    };
    let Expression::FunctionExpression(function) = call.callee.without_parentheses_mut() else {
        return None;
    };
    Some(&mut function.body.as_mut()?.statements)
}

fn rewrite<'a>(it: &mut AVec<'a, Statement<'a>>, plan: &Plan, cx: &Cx<'a, '_>) -> bool {
    let text = format!("!function ({}) {{ 0; }}({});", plan.alias_name, plan.array_name);
    let holder = match (&plan.holder, plan.form) {
        (Some(holder), Form::Assigned | Form::Flagged) => {
            parsed_statement(&format!("var {holder};"), cx).map(Some)
        }
        _ => Some(None),
    };
    let (Some(mut wrapped), Some(holder)) = (parsed_statement(&text, cx), holder) else {
        return false;
    };
    let Some(body) = wrapper_body(&mut wrapped) else {
        return false;
    };
    let mut cookie = it[plan.cookie].take_in(cx);
    if plan.form == Form::Flagged && simplified(&mut cookie, cx).is_none() {
        it[plan.cookie] = cookie;
        return false;
    }
    let rotator = it[plan.rotator].take_in(cx);
    let Some(rotator) = declared_rotator(rotator, plan, cx) else {
        return false;
    };
    body.clear();
    body.push(rotator);
    body.extend(holder);
    body.push(cookie);
    let mut removed = [plan.alias, plan.rotator, plan.cookie];
    removed.sort_unstable();
    for index in removed.iter().rev() {
        it.remove(*index);
    }
    it.insert(plan.array + 1, wrapped);
    without_bare(it, &plan.dropped);
    true
}

pub(super) fn wrap_inlined_cookie<'a>(it: &mut AVec<'a, Statement<'a>>, cx: &Cx<'a, '_>) -> bool {
    plan(it, cx.scoping()).is_some_and(|plan| rewrite(it, &plan, cx))
}
