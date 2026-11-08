use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::matcher::iife;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::Cx;

use super::checksum::parsed_statement;
use super::cookie_wrap::wrap_inlined_cookie;
use super::for_init::hoist_decoded_arrays;
use super::legacy::{incremented, is_countdown, literal_arithmetic, predecrements, pushes_shifted};
use super::single_use::declare_single_use_decoder;
use super::{array_declaration, array_literal};
use crate::array_rotator::is_infinite_rotator_loop;
use crate::mutants::inlined_rotator_vars_trusted;
use crate::stamp::DynamicScopes;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Assigned,
    Loop,
    LoopWithArray,
    Checksum,
}

struct Plan {
    first: usize,
    kind: Kind,
    moved: Vec<usize>,
    array_name: String,
    target: String,
    counter: String,
    count: usize,
}

fn assigned<'k, 'a>(stmt: &'k Statement<'a>) -> Option<(&'k str, &'k Expression<'a>)> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign)
        .then_some((target.name.as_str(), &assignment.right))
}

fn whole(value: f64) -> Option<usize> {
    let fits = value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value);
    fits.then_some(value as usize)
}

fn countdown_call(stmt: &Statement<'_>, target: &str, counter: &str) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let call = match stmt.expression.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    iife(call).is_some_and(|(function, arguments)| {
        is_countdown(function, target) && incremented(arguments, counter)
    })
}

fn rotation(statements: &[Statement<'_>], array: &str) -> Option<(String, String, usize)> {
    let [first, second, call] = statements else {
        return None;
    };
    let (first, second) = (assigned(first)?, assigned(second)?);
    let ((target, _), (counter, count)) =
        match (identifier_name(first.1), identifier_name(second.1)) {
            (Some(name), _) if name == array => (first, second),
            (_, Some(name)) if name == array => (second, first),
            _ => return None,
        };
    let count = whole(literal_arithmetic(count)?)?;
    let distinct = target != counter && target != array && counter != array;
    (distinct && countdown_call(call, target, counter))
        .then(|| (target.to_string(), counter.to_string(), count))
}

fn var_declared(body: &[Statement<'_>], name: &str) -> Option<SymbolId> {
    body.iter().find_map(|stmt| {
        let Statement::VariableDeclaration(declaration) = stmt else {
            return None;
        };
        if declaration.kind != VariableDeclarationKind::Var {
            return None;
        }
        declaration
            .declarations
            .iter()
            .find_map(|declarator| match &declarator.id {
                BindingPattern::BindingIdentifier(id) if id.name == name => id.symbol_id.get(),
                _ => None,
            })
    })
}

struct Counted<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    seen: usize,
}

impl<'a> VisitJs<'a> for Counted<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.seen += usize::from(symbol == Some(self.symbol));
    }
}

pub(super) fn used_only_in<'s, 'a: 's>(
    scoping: &Scoping,
    symbol: SymbolId,
    statements: impl Iterator<Item = &'s Statement<'a>>,
) -> bool {
    let mut counted = Counted {
        scoping,
        symbol,
        seen: 0,
    };
    for stmt in statements {
        counted.visit_statement(stmt);
    }
    let sealed = inlined_rotator_vars_trusted()
        || counted.seen == scoping.get_resolved_reference_ids(symbol).len();
    sealed
        && scoping.symbol_redeclarations(symbol).is_empty()
        && scoping.symbol_scope_id(symbol) != scoping.root_scope_id()
        && DynamicScopes::of(scoping)
            .refusal(scoping, symbol)
            .is_none()
}

fn only_used_within(scoping: &Scoping, symbol: SymbolId, statements: &[Statement<'_>]) -> bool {
    used_only_in(scoping, symbol, statements.iter())
}

fn bare(stmt: &Statement<'_>) -> bool {
    matches!(stmt, Statement::VariableDeclaration(declaration)
        if declaration.kind == VariableDeclarationKind::Var
            && declaration.declarations.iter().all(|declarator| declarator.init.is_none()))
}

fn assigned_plan(body: &[Statement<'_>], index: usize, scoping: &Scoping) -> Option<Plan> {
    let (array, length) = array_declaration(&body[index])?;
    let first = index
        + 1
        + body
            .get(index + 1..)?
            .iter()
            .take_while(|stmt| bare(stmt))
            .count();
    let rotator = body.get(first..first + 3)?;
    let (target, counter, count) = rotation(rotator, array)?;
    let sealed = length > 0
        && [&target, &counter].iter().all(|name| {
            var_declared(body, name)
                .is_some_and(|symbol| only_used_within(scoping, symbol, rotator))
        });
    let original = rotator
        .iter()
        .all(|stmt| stmt.span().start < SYNTHETIC_BASE);
    (sealed && original).then(|| Plan {
        first,
        kind: Kind::Assigned,
        moved: Vec::new(),
        array_name: array.to_string(),
        target,
        counter,
        count,
    })
}

fn binding<'k>(declarator: &'k VariableDeclarator<'_>) -> Option<(&'k str, SymbolId)> {
    match &declarator.id {
        BindingPattern::BindingIdentifier(id) => Some((id.name.as_str(), id.symbol_id.get()?)),
        _ => None,
    }
}

fn loop_plan(body: &[Statement<'_>], index: usize, scoping: &Scoping) -> Option<Plan> {
    let (at, kind, array) = match (array_declaration(&body[index]), body.get(index + 1)) {
        (Some((array, length)), Some(Statement::ForStatement(_))) if length > 0 => {
            (index + 1, Kind::Loop, Some(array))
        }
        (None, _) => (index, Kind::LoopWithArray, None),
        _ => return None,
    };
    let Statement::ForStatement(node) = &body[at] else {
        return None;
    };
    let Some(ForStatementInit::VariableDeclaration(declaration)) = node.init.as_ref() else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var || node.update.is_some() {
        return None;
    }
    let declarators = declaration.declarations.as_slice();
    let (array, alias, counter) = match (kind, declarators) {
        (Kind::Loop, [alias, counter]) => (array?, alias, counter),
        (Kind::LoopWithArray, [first, alias, counter]) => {
            let (name, _) = binding(first)?;
            first
                .init
                .as_ref()
                .and_then(array_literal)
                .filter(|n| *n > 0)?;
            (name, alias, counter)
        }
        _ => return None,
    };
    let (target, target_symbol) = binding(alias)?;
    let (count_name, count_symbol) = binding(counter)?;
    let aliases = alias.init.as_ref().and_then(identifier_name) == Some(array);
    let start = whole(literal_arithmetic(counter.init.as_ref()?)?)?;
    let shaped = aliases
        && start >= 1
        && target != count_name
        && target != array
        && count_name != array
        && node
            .test
            .as_ref()
            .is_some_and(|test| predecrements(test, count_name))
        && pushes_shifted(&node.body, target);
    let rotator = &body[at..=at];
    let sealed = shaped
        && only_used_within(scoping, target_symbol, rotator)
        && only_used_within(scoping, count_symbol, rotator)
        && body[at].span().start < SYNTHETIC_BASE;
    sealed.then(|| Plan {
        first: at,
        kind,
        moved: Vec::new(),
        array_name: array.to_string(),
        target: target.to_string(),
        counter: count_name.to_string(),
        count: start - 1,
    })
}

fn alias_of<'k>(stmt: &'k Statement<'_>) -> Option<(&'k str, Option<&'k str>)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var {
        return None;
    }
    match declarator.init.as_ref()?.without_parentheses() {
        Expression::Identifier(source) => Some((id.name.as_str(), Some(source.name.as_str()))),
        Expression::FunctionExpression(_) => Some((id.name.as_str(), None)),
        _ => None,
    }
}

fn loop_alias<'k>(stmt: &'k Statement<'_>, array: &str) -> Option<Option<(&'k str, &'k str)>> {
    let Statement::ForStatement(node) = stmt else {
        return Some(None);
    };
    match node.init.as_ref() {
        None => Some(None),
        Some(ForStatementInit::VariableDeclaration(declaration))
            if declaration.kind == VariableDeclarationKind::Var =>
        {
            let mut aliases = Vec::new();
            for declarator in &declaration.declarations {
                let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                    return None;
                };
                let source = declarator.init.as_ref().and_then(identifier_name)?;
                aliases.push((id.name.as_str(), source));
            }
            let mut of_array = aliases.iter().filter(|(_, source)| *source == array);
            match (of_array.next(), of_array.next()) {
                (Some(found), None) => Some(Some(*found)),
                _ => aliases.first().map(|first| Some(*first)),
            }
        }
        _ => None,
    }
}

fn inert_between(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::FunctionDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().all(|declarator| {
                matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                    && declarator.init.as_ref().is_some_and(|init| {
                        matches!(init.without_parentheses(), Expression::Identifier(_))
                    })
            })
        }
        _ => false,
    }
}

fn declared_names<'k>(statements: &'k [Statement<'_>]) -> Vec<(&'k str, SymbolId)> {
    let mut names = Vec::new();
    for stmt in statements {
        let declarations = match stmt {
            Statement::VariableDeclaration(declaration) => Some(&**declaration),
            Statement::ForStatement(node) => match node.init.as_ref() {
                Some(ForStatementInit::VariableDeclaration(declaration)) => Some(&**declaration),
                _ => None,
            },
            _ => None,
        };
        for declarator in declarations.into_iter().flat_map(|d| d.declarations.iter()) {
            if let Some(found) = binding(declarator) {
                names.push(found);
            }
        }
    }
    names
}

fn owned(stmt: &Statement<'_>, moved: &[usize], body: &[Statement<'_>], scoping: &Scoping) -> bool {
    declared_names(std::slice::from_ref(stmt))
        .iter()
        .all(|(_, symbol)| used_only_in(scoping, *symbol, moved.iter().map(|i| &body[*i])))
}

fn checksum_plan(body: &[Statement<'_>], index: usize, scoping: &Scoping) -> Option<Plan> {
    let (array, length) = array_declaration(&body[index])?;
    let movers = body
        .get(index + 1..)?
        .iter()
        .take_while(|stmt| alias_of(stmt).is_some() || inert_between(stmt))
        .count();
    let at = index + 1 + movers;
    let rotator = body.get(at)?;
    if length == 0 || !is_infinite_rotator_loop(rotator) {
        return None;
    }
    let looped = loop_alias(rotator, array)?;
    let region = &body[index + 1..=at];
    let mut moved: Vec<usize> = (index + 1..=at)
        .filter(|i| {
            *i == at || alias_of(&body[*i]).is_some() || crate::mutants::inert_statements_moved()
        })
        .collect();
    loop {
        let kept: Vec<usize> = moved
            .iter()
            .copied()
            .filter(|i| owned(&body[*i], &moved, body, scoping))
            .collect();
        if kept.len() == moved.len() {
            break;
        }
        moved = kept;
    }
    if moved.last() != Some(&at) {
        return None;
    }
    let parameter = match looped {
        Some((name, source)) if source == array => name.to_string(),
        Some(_) => return None,
        None => moved
            .iter()
            .filter_map(|i| alias_of(&body[*i]))
            .find_map(|(name, source)| (source == Some(array)).then_some(name))
            .unwrap_or(array)
            .to_string(),
    };
    let original = region.iter().all(|stmt| stmt.span().start < SYNTHETIC_BASE);
    original.then(|| Plan {
        first: index + 1,
        kind: Kind::Checksum,
        moved,
        array_name: array.to_string(),
        target: parameter,
        counter: String::new(),
        count: 0,
    })
}

fn rewrite_checksum<'a>(it: &mut AVec<'a, Statement<'a>>, plan: &Plan, cx: &Cx<'a, '_>) -> bool {
    let text = format!("!function ({}) {{ 0; }}({});", plan.target, plan.array_name);
    let Some(mut wrapped) = parsed_statement(&text, cx) else {
        return false;
    };
    let Some(body) = function_body(&mut wrapped) else {
        return false;
    };
    body.clear();
    let Some(&at) = plan.moved.last() else {
        return false;
    };
    for &index in &plan.moved {
        let mut stmt = it[index].take_in(cx);
        if index == at {
            if let Statement::ForStatement(node) = &mut stmt {
                drop_array_alias(node, plan);
            }
        } else if alias_of(&stmt).is_some_and(|(name, source)| {
            name == plan.target && source == Some(plan.array_name.as_str())
        }) {
            continue;
        }
        body.push(stmt);
    }
    it[at] = wrapped;
    for &index in plan.moved.iter().rev().skip(1) {
        it.remove(index);
    }
    true
}

fn drop_array_alias(node: &mut ForStatement<'_>, plan: &Plan) {
    let Some(ForStatementInit::VariableDeclaration(declaration)) = node.init.as_mut() else {
        node.init = None;
        return;
    };
    declaration.declarations.retain(|declarator| {
        let is_alias = matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.name == plan.target.as_str())
            && declarator.init.as_ref().and_then(identifier_name) == Some(plan.array_name.as_str());
        !is_alias
    });
    if declaration.declarations.is_empty() {
        node.init = None;
    }
}

fn function_body<'k, 'a>(
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

fn plan(body: &[Statement<'_>], scoping: &Scoping) -> Option<Plan> {
    (0..body.len()).find_map(|index| {
        assigned_plan(body, index, scoping)
            .or_else(|| loop_plan(body, index, scoping))
            .or_else(|| checksum_plan(body, index, scoping))
    })
}

fn loop_rotator<'a>(plan: &Plan, cx: &Cx<'a, '_>) -> Option<Statement<'a>> {
    let (x, c) = (&plan.target, &plan.counter);
    let text = format!(
        "!function ({x}, {c}) {{ (function ({c}) {{ for (; --{c};) {{ {x}.push({x}.shift()); }} }})(++{c}); }}({}, {});",
        plan.array_name, plan.count
    );
    parsed_statement(&text, cx)
}

fn rewrite_loop<'a>(it: &mut AVec<'a, Statement<'a>>, plan: &Plan, cx: &Cx<'a, '_>) -> bool {
    let Some(rotator) = loop_rotator(plan, cx) else {
        return false;
    };
    if plan.kind == Kind::Loop {
        it[plan.first] = rotator;
        return true;
    }
    let Statement::ForStatement(node) = &mut it[plan.first] else {
        return false;
    };
    let Some(ForStatementInit::VariableDeclaration(declaration)) = node.init.as_mut() else {
        return false;
    };
    let array = declaration.declarations.remove(0);
    let array = Statement::new_variable_declaration(
        array.span(),
        VariableDeclarationKind::Var,
        AVec::from_iter_in([array], cx),
        false,
        cx,
    );
    it[plan.first] = array;
    it.insert(plan.first + 1, rotator);
    true
}

fn wrapper<'a>(plan: &Plan, cx: &Cx<'a, '_>) -> Option<Statement<'a>> {
    let text = format!(
        "!function ({}, {}) {{ 0; }}({}, {});",
        plan.target, plan.counter, plan.array_name, plan.count
    );
    parsed_statement(&text, cx)
}

fn placeholder<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
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
    match function.body.as_mut()?.statements.first_mut()? {
        Statement::ExpressionStatement(placeholder) => Some(&mut placeholder.expression),
        _ => None,
    }
}

fn countdown_of<'k, 'a>(rotator: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    match rotator {
        Statement::ExpressionStatement(rotator) => Some(&mut rotator.expression),
        _ => None,
    }
}

fn rewrite<'a>(it: &mut AVec<'a, Statement<'a>>, plan: &Plan, cx: &Cx<'a, '_>) -> bool {
    let Some(mut wrapped) = wrapper(plan, cx) else {
        return false;
    };
    let (Some(slot), Some(countdown)) = (
        placeholder(&mut wrapped),
        countdown_of(&mut it[plan.first + 2]),
    ) else {
        return false;
    };
    *slot = countdown.take_in(cx);
    it[plan.first] = wrapped;
    it.remove(plan.first + 2);
    it.remove(plan.first + 1);
    it.retain(|stmt| !declares_only(stmt, &plan.target) && !declares_only(stmt, &plan.counter));
    true
}

fn declares_only(stmt: &Statement<'_>, name: &str) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    matches!(declaration.declarations.as_slice(), [declarator]
        if declarator.init.is_none()
            && matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.name == name))
}

struct Inline<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    changed: usize,
}

impl<'a> VisitMut<'a> for Inline<'_, 'a, '_> {
    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        if self.changed == 0 {
            self.changed += declare_single_use_decoder(&mut it.statements, self.cx);
        }
        walk_mut::walk_function_body(self, it);
    }

    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        if self.changed == 0 {
            if let Some(plan) = plan(it, self.cx.scoping()) {
                let rewritten = match plan.kind {
                    Kind::Assigned => rewrite(it, &plan, self.cx),
                    Kind::Loop | Kind::LoopWithArray => rewrite_loop(it, &plan, self.cx),
                    Kind::Checksum => rewrite_checksum(it, &plan, self.cx),
                };
                self.changed += usize::from(rewritten);
            } else if wrap_inlined_cookie(it, self.cx) {
                self.changed += 1;
            } else {
                self.changed += hoist_decoded_arrays(it, self.cx);
            }
        }
        walk_mut::walk_statements(self, it);
    }
}

pub(super) fn inline_assigned_rotators<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    let mut inline = Inline { cx, changed: 0 };
    inline.visit_program(program);
    inline.changed
}
