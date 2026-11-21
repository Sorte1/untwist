use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::Cx;

use super::array_declaration;
use super::checksum::parsed_statement;
use super::cookie_minified::{countdown_declaration, is_cookie_block};
use super::inlined::{copied_out_of_block, used_only_in};
use super::legacy::is_countdown;
use super::moved::{moved_vars, never_rebound, read_by_closure};
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
    direct: bool,
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
            let Expression::AssignmentExpression(assignment) =
                stmt.expression.without_parentheses()
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
            let Some(Expression::Identifier(source)) = declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses)
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

fn assigned_countdown<'k>(
    stmt: &'k Statement<'_>,
    alias: &str,
) -> Option<&'k IdentifierReference<'k>> {
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
        && matches!(
            assignment.right.without_parentheses(),
            Expression::ObjectExpression(_)
        );
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
        Statement::BlockStatement(_) if is_cookie_block(stmt) => Some((Form::Block, Vec::new())),
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

struct Mentions<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= resolved(self.scoping, it) == Some(self.symbol);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= it.symbol_id.get() == Some(self.symbol);
    }
}

fn mentions(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let mut seen = Mentions {
        scoping,
        symbol,
        found: false,
    };
    seen.visit_statement(stmt);
    seen.found
}

fn mentions_expr(expr: &Expression<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let mut seen = Mentions {
        scoping,
        symbol,
        found: false,
    };
    seen.visit_expression(expr);
    seen.found
}

fn overwrites(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    match stmt {
        Statement::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return false;
            };
            let binds = matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                if id.symbol_id.get() == Some(symbol));
            binds
                && declarator
                    .init
                    .as_ref()
                    .is_some_and(|init| !mentions_expr(init, scoping, symbol))
        }
        Statement::ExpressionStatement(node) => {
            let Expression::AssignmentExpression(assignment) = &node.expression else {
                return false;
            };
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
                return false;
            };
            assignment.operator == AssignmentOperator::Assign
                && resolved(scoping, target) == Some(symbol)
                && !mentions_expr(&assignment.right, scoping, symbol)
        }
        _ => false,
    }
}

fn overwritten_after(
    body: &[Statement<'_>],
    cookie: usize,
    scoping: &Scoping,
    symbol: SymbolId,
) -> bool {
    if crate::mutants::cookie_block_vars_trusted() {
        return true;
    }
    let outside: Vec<usize> = (0..body.len())
        .filter(|index| *index != cookie && mentions(&body[*index], scoping, symbol))
        .collect();
    let Some(&first) = outside.first() else {
        return true;
    };

    first > cookie
        && overwrites(&body[first], scoping, symbol)
        && !read_by_closure(&body[cookie], scoping, symbol)
        && outside
            .iter()
            .all(|index| !matches!(body[*index], Statement::FunctionDeclaration(_)))
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
        if copied_out_of_block(function, scoping) {
            return None;
        }
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

fn declared_by_var_in(body: &[Statement<'_>], scoping: &Scoping, symbol: SymbolId) -> bool {
    let declared = scoping.symbol_span(symbol);
    let binds = |declarator: &VariableDeclarator<'_>| {
        matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
            if id.symbol_id.get() == Some(symbol) && id.span == declared)
    };
    scoping.symbol_redeclarations(symbol).is_empty()
        && body.iter().any(|stmt| {
            matches!(stmt, Statement::VariableDeclaration(declaration)
                if declaration.kind == VariableDeclarationKind::Var
                    && declaration.declarations.iter().any(binds))
        })
}

fn declared_inside(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let declared = scoping.symbol_span(symbol);
    let span = stmt.span();
    scoping.symbol_redeclarations(symbol).is_empty()
        && span.start <= declared.start
        && declared.end <= span.end
}

fn plan_at(body: &[Statement<'_>], array: usize, scoping: &Scoping) -> Option<Plan> {
    let (array_name, length) = array_declaration(&body[array])?;
    let array_symbol = declaration_symbol(&body[array])?;
    if length == 0 || scoping.symbol_scope_id(array_symbol) == scoping.root_scope_id() {
        return None;
    }
    let alias = array
        + 1
        + body
            .get(array + 1..)?
            .iter()
            .take_while(|stmt| inert(stmt))
            .count();
    let aliased = body
        .get(alias)
        .and_then(|stmt| alias_of(stmt, scoping, array_symbol));
    let direct = aliased.is_none();
    let (alias, alias_symbol) = match aliased {
        Some(symbol) => (alias, symbol),
        None if direct => (array, array_symbol),
        None => return None,
    };
    let alias_name = scoping.symbol_name(alias_symbol);
    let rotator = rotator_of(body, (array, alias), alias_name, scoping)?;
    let cookie = if rotator.at == alias + 1 {
        alias + 2
    } else {
        alias + 1
    };
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
    owned.extend(moved_vars(&body[cookie])?);
    let passed = if direct {
        never_rebound(scoping, array_symbol)
    } else {
        in_statements(alias_symbol, &[alias, rotator.at])
    };
    let plain = (direct || declared_by_var_in(body, scoping, alias_symbol))
        && holder_symbols
            .iter()
            .all(|symbol| declared_by_var_in(body, scoping, *symbol))
        && owned
            .iter()
            .skip(holder_symbols.len())
            .all(|symbol| declared_inside(&body[cookie], scoping, *symbol));
    let sealed = passed
        && plain
        && in_statements(rotator.symbol, &rotator_uses)
        && holder_symbols
            .iter()
            .all(|symbol| in_statements(*symbol, &[cookie]))
        && owned.iter().skip(holder_symbols.len()).all(|symbol| {
            in_statements(*symbol, &[cookie]) || overwritten_after(body, cookie, scoping, *symbol)
        })
        && holder_symbols
            .iter()
            .skip(1)
            .all(|flag| scoping.get_resolved_reference_ids(*flag).len() == 2);
    let original = [array, alias, rotator.at, cookie]
        .iter()
        .all(|index| body[*index].span().start < SYNTHETIC_BASE);
    let mut dropped = if direct {
        Vec::new()
    } else {
        vec![alias_symbol]
    };
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
        direct,
    })
}

fn plan(body: &[Statement<'_>], scoping: &Scoping) -> Option<Plan> {
    (0..body.len()).find_map(|index| plan_at(body, index, scoping))
}

fn declared_rotator<'a>(
    stmt: Statement<'a>,
    plan: &Plan,
    cx: &Cx<'a, '_>,
) -> Option<Statement<'a>> {
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
    let Expression::ConditionalExpression(choice) = stmt.expression.without_parentheses_mut()
    else {
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
                declaration
                    .declarations
                    .retain(|declarator| !gone(declarator));
            }
        }
    }
    it.retain(|stmt| {
        !matches!(stmt, Statement::VariableDeclaration(declaration)
            if declaration.declarations.is_empty())
    });
}

fn wrapper_body<'k, 'a>(
    statement: &'k mut Statement<'a>,
) -> Option<&'k mut AVec<'a, Statement<'a>>> {
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
    let text = format!(
        "!function ({}) {{ 0; }}({});",
        plan.alias_name, plan.array_name
    );
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
    let mut removed = vec![plan.rotator, plan.cookie];
    if !plan.direct {
        removed.push(plan.alias);
    }
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
