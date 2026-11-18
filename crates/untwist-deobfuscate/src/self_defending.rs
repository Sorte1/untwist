use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::inline::is_plain_function;
use untwist_core::ast_util::literal::{is_false_literal, is_true_literal};
use untwist_core::ast_util::matcher::{anonymous_function, call_parts, iife};
use untwist_core::ast_util::naming_slot::without_unnamed;
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::{scan, Builtins, Reliance};
use untwist_core::semantic::{has_module_syntax, AnnexBWrites};

use crate::bindings::{
    has_stray_reference, reaches_a_script_global_once_removed, reference_symbol, resolved_symbol,
};
use crate::builtin_values::exposes_a_builtin;
use crate::control_flow_object::single_declarator;
use crate::directives::remove_protection_statements;
use crate::flag_home::{is_private_local, Homes, ARGUMENTS, DIRECT_EVAL};
use crate::notes;
use crate::protection_shape::{is_protection_callback, Roles};

pub const PASS: &str = "self-defending";

const MAX_NARROWED_CONTROLLERS: usize = 16;

const RESERVED_PARAMS: [&str; 2] = [ARGUMENTS, "eval"];

static CALLBACK_RELIES_ON: Reliance = Reliance {
    roots: &["Function", "String", "RegExp", "Symbol"],
    members: &[],
};

fn binding_name<'k>(pattern: &'k BindingPattern<'_>) -> Option<&'k str> {
    let BindingPattern::BindingIdentifier(id) = pattern else {
        return None;
    };
    Some(id.name.as_str())
}

fn plain_function<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    anonymous_function(expr).filter(|function| is_plain_function(function))
}

fn plain_params<'k>(params: &'k FormalParameters<'_>) -> Option<Vec<&'k str>> {
    if params.rest.is_some() {
        return None;
    }
    let names: Vec<&str> = params
        .items
        .iter()
        .map(|item| {
            item.initializer
                .is_none()
                .then(|| binding_name(&item.pattern))
                .flatten()
        })
        .collect::<Option<_>>()?;
    let distinct = names
        .iter()
        .enumerate()
        .all(|(at, name)| !names[..at].contains(name));
    let reserved = names.iter().any(|name| RESERVED_PARAMS.contains(name));
    (distinct && !reserved).then_some(names)
}

fn sole_declarator<'k, 'a>(
    declaration: &'k VariableDeclaration<'a>,
) -> Option<&'k VariableDeclarator<'a>> {
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    Some(declarator)
}

fn assigned_value<'k, 'a>(
    reset: &'k ExpressionStatement<'a>,
    name: &str,
) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assignment) = &reset.expression else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (target.name.as_str() == name).then_some(&assignment.right)
}

fn is_bare_var_declaration(statement: &Statement<'_>, shape_names: &[&str]) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.kind == VariableDeclarationKind::Var
        && declaration.declarations.iter().all(|declarator| {
            declarator.init.is_none()
                && binding_name(&declarator.id).is_some_and(|name| !shape_names.contains(&name))
        })
}

fn live_statements<'k, 'a>(
    statements: &'k [Statement<'a>],
    shape_names: &[&str],
) -> Vec<&'k Statement<'a>> {
    statements
        .iter()
        .filter(|statement| !is_bare_var_declaration(statement, shape_names))
        .collect()
}

fn applies_the_guarded_function(init: &Expression<'_>, context: &str, fn_name: &str) -> bool {
    let Some((callee, arguments)) = call_parts(init) else {
        return false;
    };
    let Some((object, name)) = member_parts(callee) else {
        return false;
    };
    name == "apply"
        && identifier_name(object) == Some(fn_name)
        && arguments.len() == 2
        && arguments[0].as_expression().and_then(identifier_name) == Some(context)
        && arguments[1].as_expression().and_then(identifier_name) == Some("arguments")
}

fn bare_name<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    if declarator.init.is_some() {
        return None;
    }
    binding_name(&declarator.id)
}

fn guarded_if<'k, 'a>(body: &'k [Statement<'a>]) -> Option<(Option<&'k str>, &'k IfStatement<'a>)> {
    match body {
        [Statement::IfStatement(statement)] => Some((None, statement)),
        [declared, Statement::IfStatement(statement)] => {
            Some((Some(bare_name(declared)?), statement))
        }
        _ => None,
    }
}

fn applied_result<'k, 'a>(
    live: &[&'k Statement<'a>],
    declared: &[&str],
) -> Option<(
    &'k str,
    &'k Expression<'a>,
    &'k ExpressionStatement<'a>,
    &'k ReturnStatement<'a>,
)> {
    match live {
        [Statement::VariableDeclaration(declaration), Statement::ExpressionStatement(reset), Statement::ReturnStatement(ret)] =>
        {
            let declarator = sole_declarator(declaration)?;
            let result = binding_name(&declarator.id)?;
            Some((result, declarator.init.as_ref()?, reset, ret))
        }
        [declared_here, Statement::ExpressionStatement(applied), Statement::ExpressionStatement(reset), Statement::ReturnStatement(ret)] =>
        {
            let result = bare_name(declared_here)?;
            Some((result, assigned_value(applied, result)?, reset, ret))
        }
        [Statement::ExpressionStatement(applied), Statement::ExpressionStatement(reset), Statement::ReturnStatement(ret)] =>
        {
            let result = ret.argument.as_ref().and_then(identifier_name)?;
            if !declared.contains(&result) {
                return None;
            }
            Some((result, assigned_value(applied, result)?, reset, ret))
        }
        _ => None,
    }
}

fn returns_guarded_apply(function: &Function<'_>, context: &str, fn_name: &str) -> bool {
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let Some((hoisted, statement)) = guarded_if(&body.statements) else {
        return false;
    };
    if identifier_name(&statement.test) != Some(fn_name) || statement.alternate.is_some() {
        return false;
    }
    let Statement::BlockStatement(block) = &statement.consequent else {
        return false;
    };
    if hoisted.is_some_and(|name| [context, fn_name, "arguments"].contains(&name)) {
        return false;
    }
    let mut declared: Vec<&str> = block.body.iter().filter_map(bare_name).collect();
    declared.extend(hoisted);
    let live = live_statements(&block.body, &[context, fn_name, "arguments"]);
    let Some((result, init, reset, ret)) = applied_result(&live, &declared) else {
        return false;
    };
    applies_the_guarded_function(init, context, fn_name)
        && assigned_value(reset, fn_name)
            .is_some_and(|value| matches!(value, Expression::NullLiteral(_)))
        && ret.argument.as_ref().and_then(identifier_name) == Some(result)
        && ![context, fn_name, "arguments"].contains(&result)
}

fn is_empty_function(function: &Function<'_>) -> bool {
    has_no_params(function)
        && function
            .body
            .as_ref()
            .is_some_and(|body| body.statements.is_empty())
}

fn has_no_params(function: &Function<'_>) -> bool {
    function.params.items.is_empty() && function.params.rest.is_none()
}

fn chooses_the_guarded_function(init: &Expression<'_>, flag: &str, names: (&str, &str)) -> bool {
    let Expression::ConditionalExpression(choice) = init else {
        return false;
    };
    if identifier_name(&choice.test) != Some(flag) {
        return false;
    }
    let (Some(taken), Some(skipped)) = (
        plain_function(&choice.consequent),
        plain_function(&choice.alternate),
    ) else {
        return false;
    };
    has_no_params(taken)
        && returns_guarded_apply(taken, names.0, names.1)
        && is_empty_function(skipped)
}

fn inner_matches(inner: &Function<'_>, flag: &str) -> bool {
    returner_matches(&inner.params, inner.body.as_deref(), flag)
}

fn returner_matches(
    params: &FormalParameters<'_>,
    body: Option<&FunctionBody<'_>>,
    flag: &str,
) -> bool {
    let Some(names) = plain_params(params) else {
        return false;
    };
    let &[context, fn_name] = names.as_slice() else {
        return false;
    };
    if [context, fn_name].contains(&flag) {
        return false;
    }
    let Some(body) = body else {
        return false;
    };
    let live = live_statements(&body.statements, &[flag]);
    let [Statement::VariableDeclaration(declaration), Statement::ExpressionStatement(reset), Statement::ReturnStatement(ret)] =
        live.as_slice()
    else {
        return false;
    };
    let Some(declarator) = sole_declarator(declaration) else {
        return false;
    };
    let (Some(rfn), Some(init)) = (binding_name(&declarator.id), declarator.init.as_ref()) else {
        return false;
    };
    ![flag, context, fn_name, "arguments"].contains(&rfn)
        && chooses_the_guarded_function(init, flag, (context, fn_name))
        && assigned_value(reset, flag).is_some_and(is_false_literal)
        && ret.argument.as_ref().and_then(identifier_name) == Some(rfn)
}

fn controller_body<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k FunctionBody<'a>> {
    if let Some((outer, arguments)) = iife(expr) {
        if !arguments.is_empty() || !has_no_params(outer) || !is_plain_function(outer) {
            return None;
        }
        return outer.body.as_deref();
    }
    let (callee, arguments) = call_parts(expr)?;
    let Expression::ArrowFunctionExpression(arrow) = callee.without_parentheses() else {
        return None;
    };
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        return None;
    };
    let plain = arguments.is_empty()
        && arrow.params.items.is_empty()
        && arrow.params.rest.is_none()
        && !arrow.r#async;
    plain.then_some(&**body)
}

pub fn is_call_controller(expr: &Expression<'_>) -> bool {
    let Some(body) = controller_body(expr) else {
        return false;
    };
    let live = live_statements(&body.statements, &[]);
    let [Statement::VariableDeclaration(first_call), Statement::ReturnStatement(ret)] =
        live.as_slice()
    else {
        return false;
    };
    if !matches!(
        first_call.kind,
        VariableDeclarationKind::Var | VariableDeclarationKind::Let
    ) {
        return false;
    }
    let Some(declarator) = sole_declarator(first_call) else {
        return false;
    };
    let Some(flag) = binding_name(&declarator.id) else {
        return false;
    };
    if !declarator.init.as_ref().is_some_and(is_true_literal) {
        return false;
    }
    let Some(inner) = ret.argument.as_ref().and_then(plain_function) else {
        return false;
    };
    inner_matches(inner, flag)
}

fn controller_declaration(stmt: &Statement<'_>) -> Option<SymbolId> {
    let (id, init) = single_declarator(stmt)?;
    if !is_call_controller(init?) {
        return None;
    }
    id.symbol_id.get()
}

fn is_immediate_controller_call(
    stmt: &Statement<'_>,
    controllers: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Some((outer, _)) = call_parts(&stmt.expression) else {
        return false;
    };
    let Some((inner, _)) = call_parts(outer) else {
        return false;
    };
    reference_symbol(inner, scoping).is_some_and(|symbol| controllers.contains(&symbol))
}

fn controller_variable(
    stmt: &Statement<'_>,
    controllers: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> Option<SymbolId> {
    let (id, init) = single_declarator(stmt)?;
    let (callee, _) = call_parts(init?)?;
    if is_call_controller(callee.without_parentheses()) {
        return id.symbol_id.get();
    }
    let controller = reference_symbol(callee, scoping)?;
    if !controllers.contains(&controller) {
        return None;
    }
    id.symbol_id.get()
}

fn reference_symbol_of_bare_call(stmt: &Statement<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let (callee, arguments) = call_parts(&stmt.expression)?;
    if !arguments.is_empty() {
        return None;
    }
    reference_symbol(callee, scoping)
}

fn is_bare_call(stmt: &Statement<'_>, variable: SymbolId, scoping: &Scoping) -> bool {
    reference_symbol_of_bare_call(stmt, scoping) == Some(variable)
}

fn shell_statements<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k AVec<'a, Statement<'a>>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let (function, arguments) = iife(&stmt.expression)?;
    if !arguments.is_empty() || !has_no_params(function) || !is_plain_function(function) {
        return None;
    }
    Some(&function.body.as_ref()?.statements)
}

fn set_flag<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k IdentifierReference<'a>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &stmt.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign && is_true_literal(&assignment.right))
        .then_some(&**target)
}

fn returner_flag<'f>(expr: &Expression<'_>, flags: &[&'f str]) -> Option<&'f str> {
    let (params, body) = match without_unnamed(expr.without_parentheses()).without_parentheses() {
        Expression::FunctionExpression(function)
            if function.id.is_none() && is_plain_function(function) =>
        {
            (&function.params, function.body.as_deref())
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => (&arrow.params, Some(&**body)),
            _ => return None,
        },
        _ => return None,
    };
    flags
        .iter()
        .copied()
        .find(|flag| returner_matches(params, body, flag))
}

fn returner(expr: &Expression<'_>, flags: &[&str]) -> bool {
    returner_flag(expr, flags).is_some()
}

fn flag_of_statement<'f>(stmt: &Statement<'_>, flags: &[&'f str]) -> Option<&'f str> {
    if let Some((_, Some(init))) = single_declarator(stmt) {
        return returner_flag(init, flags)
            .or_else(|| call_parts(init).and_then(|(callee, _)| returner_flag(callee, flags)));
    }
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let (outer, _) = call_parts(&stmt.expression)?;
    let (inner, _) = call_parts(outer.without_parentheses())?;
    returner_flag(inner, flags)
}

struct FlagUse {
    reference: Option<ReferenceId>,
    set_at: Span,
    returner_at: Span,
}

fn flag_controller(stmt: &Statement<'_>, flags: &[&str]) -> Option<SymbolId> {
    let (id, init) = single_declarator(stmt)?;
    returner(init?, flags).then(|| id.symbol_id.get()).flatten()
}

#[derive(Default)]
struct Controllers {
    found: FxHashSet<SymbolId>,
    inline: bool,
    returner_calls: Vec<Span>,
    flags: Vec<FlagUse>,
    homes: Homes,
}

fn is_returner_variable(stmt: &Statement<'_>, flags: &[&str]) -> bool {
    if flags.is_empty() {
        return false;
    }
    let Some((_, Some(init))) = single_declarator(stmt) else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(init) else {
        return false;
    };
    let made =
        returner(callee.without_parentheses(), flags) || crate::mutants::inline_returners_trusted();
    arguments.len() == 2 && made
}

fn inline_controller_call(stmt: &Statement<'_>) -> bool {
    let Some((_, Some(init))) = single_declarator(stmt) else {
        return false;
    };
    call_parts(init).is_some_and(|(callee, _)| is_call_controller(callee.without_parentheses()))
}

impl<'a> VisitJs<'a> for Controllers {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        let set: Vec<(&IdentifierReference<'a>, Span)> = it
            .iter()
            .filter_map(|stmt| set_flag(stmt).map(|target| (target, stmt.span())))
            .collect();
        let flags: Vec<&str> = set.iter().map(|(target, _)| target.name.as_str()).collect();
        for stmt in it {
            if let Some(flag) = flag_of_statement(stmt, &flags) {
                self.flags
                    .extend(set.iter().filter(|(target, _)| target.name == flag).map(
                        |(target, at)| FlagUse {
                            reference: target.reference_id.get(),
                            set_at: *at,
                            returner_at: stmt.span(),
                        },
                    ));
            }
            if let Some(symbol) = controller_declaration(stmt) {
                self.found.insert(symbol);
            }
            if let Some(symbol) = flag_controller(stmt, &flags) {
                self.found.insert(symbol);
            }
            self.inline |= inline_controller_call(stmt);
            if is_inline_returner_call(stmt, &flags) {
                self.inline = true;
                self.returner_calls.push(stmt.span());
            }
            if is_returner_variable(stmt, &flags) {
                self.inline = true;
                self.returner_calls.push(stmt.span());
            }
        }
        walk_js::walk_statements(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let scope = it.scope_id.get();
        self.homes.enter(scope, it.params.span, false);
        walk_js::walk_function(self, it, flags);
        self.homes.leave(scope);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let scope = it.scope_id.get();
        self.homes.enter(scope, it.params.span, true);
        walk_js::walk_arrow_function_expression(self, it);
        self.homes.leave(scope);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.homes.reaches_every_frame();
        walk_js::walk_with_statement(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = it.callee.without_parentheses() {
            if callee.name == DIRECT_EVAL {
                self.homes.reaches_every_frame();
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == ARGUMENTS {
            self.homes.reads_arguments();
        }
    }
}

#[derive(Default)]
struct Frame {
    declared: FxHashSet<SymbolId>,
    reachable: FxHashSet<SymbolId>,
}

fn is_inert_value(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
    )
}

fn is_inert_statement(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().all(|declarator| {
                matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                    && declarator.init.as_ref().is_none_or(is_inert_value)
            })
        }
        _ => false,
    }
}

fn called_body<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k AVec<'a, Statement<'a>>> {
    let (_, init) = single_declarator(stmt)?;
    let (callee, arguments) = call_parts(init?.without_parentheses())?;
    if !arguments.is_empty() {
        return None;
    }
    match callee.without_parentheses() {
        Expression::FunctionExpression(function)
            if has_no_params(function) && is_plain_function(function) =>
        {
            Some(&function.body.as_ref()?.statements)
        }
        Expression::ArrowFunctionExpression(arrow)
            if !arrow.r#async && arrow.params.items.is_empty() && arrow.params.rest.is_none() =>
        {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(&body.statements),
                _ => None,
            }
        }
        _ => None,
    }
}

fn hoisted_body<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k AVec<'a, Statement<'a>>> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    let simple = function.params.rest.is_none()
        && function.params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        });
    if !simple || !is_plain_function(function) {
        return None;
    }
    Some(&function.body.as_ref()?.statements)
}

struct Removals<'k> {
    controllers: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    annex_b: &'k AnnexBWrites,
    spans: Vec<Span>,
    removed: FxHashSet<Span>,
    variables: FxHashSet<SymbolId>,
    returner_calls: &'k [Span],
    proof: Option<&'k FxHashSet<SymbolId>>,
    unproven: FxHashSet<SymbolId>,
    kept: bool,
    frames: Vec<Frame>,
    module: bool,
    placed_anywhere: bool,
    with_depth: usize,
}

fn registration<'k, 'a>(stmt: &'k Statement<'a>) -> Option<(Option<SymbolId>, &'k Expression<'a>)> {
    if let Some((id, init)) = single_declarator(stmt) {
        return Some((id.symbol_id.get(), init?));
    }
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let (outer, _) = call_parts(&stmt.expression)?;
    Some((None, outer.without_parentheses()))
}

fn is_inert_context(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => {
            id.name == "undefined" && resolved_symbol(id, scoping).is_none()
        }
        Expression::ThisExpression(_)
        | Expression::NullLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

fn is_callback_expression(expr: &Expression<'_>) -> bool {
    matches!(
        without_unnamed(expr.without_parentheses()).without_parentheses(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

fn emitted_arguments(arguments: &[Argument<'_>], scoping: &Scoping) -> bool {
    let [context, callback] = arguments else {
        return false;
    };
    context
        .as_expression()
        .is_some_and(|context| is_inert_context(context, scoping))
        && callback.as_expression().is_some_and(is_callback_expression)
}

fn emitted_call(stmt: &Statement<'_>, scoping: &Scoping) -> bool {
    if let Statement::ExpressionStatement(expression) = stmt {
        let passes_nothing =
            call_parts(&expression.expression).is_some_and(|(_, arguments)| arguments.is_empty());
        if !passes_nothing {
            return false;
        }
    }
    registration(stmt)
        .and_then(|(_, call)| call_parts(call.without_parentheses()))
        .is_some_and(|(_, arguments)| emitted_arguments(arguments, scoping))
}

fn is_inline_returner_call(stmt: &Statement<'_>, flags: &[&str]) -> bool {
    if flags.is_empty() {
        return false;
    }
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Some((outer, arguments)) = call_parts(&stmt.expression) else {
        return false;
    };
    let Some((inner, inner_arguments)) = call_parts(outer.without_parentheses()) else {
        return false;
    };
    arguments.is_empty()
        && inner_arguments.len() == 2
        && (returner(inner.without_parentheses(), flags)
            || crate::mutants::inline_returners_trusted())
}

impl Removals<'_> {
    fn is_shell(&self, stmt: &Statement<'_>) -> bool {
        let Some(body) = shell_statements(stmt) else {
            return false;
        };
        body.iter().all(|inner| {
            is_immediate_controller_call(inner, self.controllers, self.scoping)
                && self.emitted_site(inner)
                && self.runs_only_protection(inner)
        })
    }

    fn runs_only_protection(&self, stmt: &Statement<'_>) -> bool {
        let Some(protections) = self.proof else {
            return true;
        };
        let Some((own, call)) = registration(stmt) else {
            return false;
        };
        let Some((callee, arguments)) = call_parts(call.without_parentheses()) else {
            return false;
        };
        let [_, callback] = arguments.as_slice() else {
            return false;
        };
        let Some(callback) = callback.as_expression() else {
            return false;
        };
        let roles = Roles {
            own,
            controller: reference_symbol(callee, self.scoping),
            protections: Some(protections),
        };
        is_protection_callback(callback, roles, self.scoping)
    }

    fn declared_once(&self, symbol: SymbolId) -> bool {
        self.scoping.symbol_redeclarations(symbol).is_empty() && !self.annex_b.may_write(symbol)
    }

    fn controller_declared_before(&self, stmt: &Statement<'_>) -> bool {
        let controller = registration(stmt)
            .and_then(|(_, call)| call_parts(call.without_parentheses()))
            .and_then(|(callee, _)| reference_symbol(callee, self.scoping));
        self.placed_anywhere
            || controller.is_none_or(|symbol| {
                self.frames.last().is_some_and(|frame| {
                    frame.declared.contains(&symbol) || frame.reachable.contains(&symbol)
                })
            })
    }

    fn emitted_site(&self, stmt: &Statement<'_>) -> bool {
        self.with_depth == 0
            && emitted_call(stmt, self.scoping)
            && self.controller_declared_before(stmt)
    }

    fn record(&mut self, stmt: &Statement<'_>) -> Option<Option<SymbolId>> {
        let declared = single_declarator(stmt)
            .and_then(|(id, _)| id.symbol_id.get())
            .filter(|symbol| self.controllers.contains(symbol));
        if let Some(controller) = declared {
            if self.with_depth > 0 {
                self.unproven.insert(controller);
                self.kept = true;
                return None;
            }
            if let Some(frame) = self.frames.last_mut() {
                frame.declared.insert(controller);
            }
            self.mark(stmt.span());
            return Some(None);
        }
        if self.is_shell(stmt) {
            self.mark(stmt.span());
            return Some(None);
        }
        let returner_call = self.returner_calls.contains(&stmt.span());
        let registers =
            is_immediate_controller_call(stmt, self.controllers, self.scoping) || returner_call;
        let variable = controller_variable(stmt, self.controllers, self.scoping).or_else(|| {
            returner_call
                .then(|| single_declarator(stmt).and_then(|(id, _)| id.symbol_id.get()))
                .flatten()
        });
        if !(registers || variable.is_some()) {
            return None;
        }
        let sound_variable = variable.is_none_or(|symbol| self.declared_once(symbol));
        if !sound_variable || !self.emitted_site(stmt) || !self.runs_only_protection(stmt) {
            self.keep(stmt);
            return None;
        }
        self.mark(stmt.span());
        self.variables.extend(variable);
        Some(variable)
    }

    fn mark(&mut self, span: Span) {
        self.spans.push(span);
        self.removed.insert(span);
    }

    fn keep(&mut self, stmt: &Statement<'_>) {
        let controller = registration(stmt)
            .and_then(|(_, call)| call_parts(call.without_parentheses()))
            .and_then(|(callee, _)| reference_symbol(callee, self.scoping));
        self.unproven.extend(controller);
        self.kept = true;
    }

    fn sets_a_local(&self, stmt: &Statement<'_>) -> bool {
        let root = self.scoping.root_scope_id();
        set_flag(stmt)
            .and_then(|target| resolved_symbol(target, self.scoping))
            .is_some_and(|symbol| self.scoping.symbol_scope_id(symbol) != root)
    }

    fn declared_first(&self, statements: &[Statement<'_>]) -> FxHashSet<SymbolId> {
        let mut found = FxHashSet::default();
        if self.with_depth > 0 || (self.module && self.frames.is_empty()) {
            return found;
        }
        for stmt in statements {
            let controller = single_declarator(stmt)
                .and_then(|(id, _)| id.symbol_id.get())
                .filter(|symbol| self.controllers.contains(symbol));
            match controller {
                Some(symbol) => {
                    found.insert(symbol);
                }
                None if is_inert_statement(stmt) || self.sets_a_local(stmt) => {}
                None => break,
            }
        }
        found
    }

    fn visit_list<'a>(
        &mut self,
        it: &AVec<'a, Statement<'a>>,
        reachable: FxHashSet<SymbolId>,
        leading: FxHashSet<SymbolId>,
    ) {
        let first = self.declared_first(it);
        self.visit_list_after(it, first, (reachable, leading));
    }

    fn visit_list_after<'a>(
        &mut self,
        it: &AVec<'a, Statement<'a>>,
        mut inherited: FxHashSet<SymbolId>,
        (reachable, leading): (FxHashSet<SymbolId>, FxHashSet<SymbolId>),
    ) {
        inherited.extend(reachable.iter().copied());
        let mut opening = reachable.clone();
        opening.extend(leading);
        self.frames.push(Frame {
            declared: FxHashSet::default(),
            reachable: opening,
        });
        let mut armed = FxHashSet::default();
        let mut opening_run = true;
        for (at, stmt) in it.iter().enumerate() {
            if opening_run && at > 0 && !self.removed.contains(&it[at - 1].span()) {
                opening_run = false;
                if let Some(frame) = self.frames.last_mut() {
                    frame.reachable.clone_from(&reachable);
                }
            }
            match self.record(stmt) {
                Some(Some(variable)) if !self.placed_anywhere => {
                    self.remove_later_calls(&it[at + 1..], variable);
                }
                Some(_) => {}
                None => self.visit_unrecorded(stmt, &inherited, &armed),
            }
            armed = self.armed_after(stmt, armed);
        }
        self.frames.pop();
    }

    fn visit_unrecorded<'a>(
        &mut self,
        stmt: &Statement<'a>,
        inherited: &FxHashSet<SymbolId>,
        armed: &FxHashSet<SymbolId>,
    ) {
        if let Some(body) = hoisted_body(stmt) {
            self.visit_list(body, inherited.clone(), FxHashSet::default());
            return;
        }
        match called_body(stmt) {
            Some(body) if !armed.is_empty() => {
                self.visit_list(body, FxHashSet::default(), armed.clone());
            }
            _ => walk_js::walk_statement(self, stmt),
        }
    }

    fn armed_after(
        &self,
        stmt: &Statement<'_>,
        mut armed: FxHashSet<SymbolId>,
    ) -> FxHashSet<SymbolId> {
        if self.with_depth > 0 {
            return FxHashSet::default();
        }
        let declared = single_declarator(stmt)
            .and_then(|(id, _)| id.symbol_id.get())
            .filter(|symbol| self.controllers.contains(symbol));
        match declared {
            Some(symbol) => {
                armed.insert(symbol);
                armed
            }
            None if is_inert_statement(stmt) => armed,
            None => FxHashSet::default(),
        }
    }

    fn remove_later_calls(&mut self, later: &[Statement<'_>], variable: SymbolId) {
        let calls: Vec<Span> = later
            .iter()
            .filter(|stmt| is_bare_call(stmt, variable, self.scoping))
            .map(GetSpan::span)
            .collect();
        for span in calls {
            self.mark(span);
        }
    }
}

impl<'a> VisitJs<'a> for Removals<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        self.visit_list(it, FxHashSet::default(), FxHashSet::default());
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.with_depth += 1;
        walk_js::walk_with_statement(self, it);
        self.with_depth -= 1;
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        if let Some(test) = &it.test {
            self.visit_expression(test);
        }
        let none = FxHashSet::default;
        self.visit_list_after(&it.consequent, none(), (none(), none()));
    }
}

enum Plan {
    Nothing,
    Refused,
    Remove(Vec<Span>, FxHashSet<SymbolId>),
}

struct BareCalls<'k> {
    variables: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    spans: Vec<Span>,
    with_depth: usize,
}

impl<'a> VisitJs<'a> for BareCalls<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            let called = self.with_depth == 0
                && reference_symbol_of_bare_call(stmt, self.scoping)
                    .is_some_and(|symbol| self.variables.contains(&symbol));
            if called {
                self.spans.push(stmt.span());
            }
        }
        walk_js::walk_statements(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.with_depth += 1;
        walk_js::walk_with_statement(self, it);
        self.with_depth -= 1;
    }
}

struct Inputs<'k> {
    scoping: &'k Scoping,
    unsafe_lane: bool,
    annex_b: &'k AnnexBWrites,
    returner_calls: &'k [Span],
    proof: Option<&'k FxHashSet<SymbolId>>,
}

fn removals<'k>(
    program: &Program<'_>,
    inputs: &Inputs<'k>,
    controllers: &'k FxHashSet<SymbolId>,
) -> Removals<'k> {
    let mut removals = Removals {
        controllers,
        scoping: inputs.scoping,
        annex_b: inputs.annex_b,
        spans: Vec::new(),
        removed: FxHashSet::default(),
        variables: FxHashSet::default(),
        returner_calls: inputs.returner_calls,
        proof: inputs.proof,
        unproven: FxHashSet::default(),
        kept: false,
        frames: Vec::new(),
        module: program.source_type.is_module(),
        placed_anywhere: inputs.unsafe_lane,
        with_depth: 0,
    };
    removals.visit_program(program);
    removals
}

fn removed_declaration(spans: &[Span], declared: Span) -> bool {
    spans
        .iter()
        .any(|span| span.start <= declared.start && declared.end <= span.end)
}

fn is_settable_flag(scoping: &Scoping, symbol: SymbolId, flag: &FlagUse) -> bool {
    let flags = scoping.symbol_flags(symbol);
    if flags.is_const_variable() {
        return false;
    }
    flags.contains(SymbolFlags::FunctionScopedVariable)
        || scoping.symbol_span(symbol).end <= flag.set_at.start.min(flag.returner_at.start)
}

fn flag_read_elsewhere(
    program: &Program<'_>,
    (flags, homes): (&[FlagUse], &Homes),
    spans: &[Span],
    scoping: &Scoping,
) -> bool {
    let removed: Vec<&FlagUse> = flags
        .iter()
        .filter(|flag| removed_declaration(spans, flag.returner_at))
        .collect();
    if removed.is_empty() {
        return false;
    }
    let mut symbols = FxHashSet::default();
    for flag in &removed {
        let symbol = flag
            .reference
            .and_then(|reference| scoping.get_reference(reference).symbol_id());
        let Some(symbol) = symbol else {
            return true;
        };
        if !is_settable_flag(scoping, symbol, flag) || !is_private_local(homes, scoping, symbol) {
            return true;
        }
        symbols.insert(symbol);
    }
    let mut ignored = spans.to_vec();
    ignored.extend(removed.iter().map(|flag| flag.set_at));
    has_stray_reference(program, &symbols, &ignored, scoping)
}

fn plan(program: &Program<'_>, scoping: &Scoping, unsafe_lane: bool) -> Plan {
    let mut controllers = Controllers::default();
    controllers.visit_program(program);
    if controllers.found.is_empty() && !controllers.inline {
        return Plan::Nothing;
    }
    let protections =
        (!unsafe_lane).then(|| crate::debug_protection::protection_symbols(program, scoping));
    let annex_b = AnnexBWrites::of(program, scoping);
    let inputs = Inputs {
        scoping,
        unsafe_lane,
        annex_b: &annex_b,
        returner_calls: controllers.returner_calls.as_slice(),
        proof: protections.as_ref(),
    };
    let declared_once: FxHashSet<SymbolId> = controllers
        .found
        .iter()
        .copied()
        .filter(|&symbol| {
            scoping.symbol_redeclarations(symbol).is_empty() && !annex_b.may_write(symbol)
        })
        .collect();
    let incomplete = declared_once.len() != controllers.found.len();
    let whole = attempt(program, &inputs, &controllers, &declared_once, incomplete);
    if !unsafe_lane
        || !matches!(whole, Plan::Refused)
        || !(2..=MAX_NARROWED_CONTROLLERS).contains(&declared_once.len())
    {
        return whole;
    }
    narrowed(program, &inputs, &controllers, &declared_once)
}

fn narrowed(
    program: &Program<'_>,
    inputs: &Inputs<'_>,
    controllers: &Controllers,
    declared_once: &FxHashSet<SymbolId>,
) -> Plan {
    let mut candidates: Vec<SymbolId> = declared_once.iter().copied().collect();
    candidates.sort_unstable();
    let removable: FxHashSet<SymbolId> = candidates
        .into_iter()
        .filter(|&symbol| {
            let single = std::iter::once(symbol).collect();
            matches!(
                attempt(program, inputs, controllers, &single, true),
                Plan::Remove(..)
            )
        })
        .collect();
    if removable.is_empty() {
        return Plan::Refused;
    }
    attempt(program, inputs, controllers, &removable, true)
}

fn attempt(
    program: &Program<'_>,
    inputs: &Inputs<'_>,
    controllers: &Controllers,
    declared_once: &FxHashSet<SymbolId>,
    incomplete: bool,
) -> Plan {
    let scoping = inputs.scoping;
    let first = removals(program, inputs, declared_once);
    let kept = first.kept || incomplete;
    let proven: FxHashSet<SymbolId> = declared_once.difference(&first.unproven).copied().collect();
    let Removals {
        mut spans,
        variables,
        ..
    } = if first.unproven.is_empty() {
        first
    } else {
        removals(program, inputs, &proven)
    };
    if inputs.unsafe_lane {
        let mut bare = BareCalls {
            variables: &variables,
            scoping,
            spans: Vec::new(),
            with_depth: 0,
        };
        bare.visit_program(program);
        spans.extend(bare.spans);
    }
    let mut tracked = proven;
    tracked.extend(variables);
    if has_stray_reference(program, &tracked, &spans, scoping)
        || flag_read_elsewhere(
            program,
            (&controllers.flags, &controllers.homes),
            &spans,
            scoping,
        )
    {
        return Plan::Refused;
    }
    match (spans.is_empty(), kept) {
        (false, _) => Plan::Remove(spans, tracked),
        (true, true) => Plan::Refused,
        (true, false) => Plan::Nothing,
    }
}

fn callback_built_ins_written(program: &Program<'_>, scoping: &Scoping) -> bool {
    let module = has_module_syntax(program);
    scan(program, scoping, module, false, &CALLBACK_RELIES_ON).verdict == Builtins::Written
        || exposes_a_builtin(program, scoping, CALLBACK_RELIES_ON.roots)
}

#[derive(Default)]
pub struct SelfDefending;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(SelfDefending)
}

impl<'a> Pass<'a> for SelfDefending {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let spans = match plan(program, cx.scoping(), cx.opts.unsafe_transforms) {
            Plan::Nothing => return,
            Plan::Refused => {
                cx.note(notes::SELF_DEFENDING_REFERENCE_KEPT, None);
                return;
            }
            Plan::Remove(spans, tracked) => {
                if !cx.opts.unsafe_transforms && callback_built_ins_written(program, cx.scoping()) {
                    cx.note(notes::SELF_DEFENDING_BUILTIN_PATCHED, None);
                    return;
                }
                if reaches_a_script_global_once_removed(&tracked, &spans, program, cx) {
                    cx.note(notes::SELF_DEFENDING_REFERENCE_KEPT, None);
                    return;
                }
                spans
            }
        };
        if crate::directives::removes_what_an_eval_reads(cx.scoping(), &spans) {
            cx.note(notes::SELF_DEFENDING_REFERENCE_KEPT, None);
            return;
        }
        if crate::directives::removes_hoist_blocker(cx.scoping(), &spans) {
            cx.note(notes::PROTECTION_HOIST_BLOCKER_KEPT, None);
            return;
        }
        let (wanted, applied) = match remove_protection_statements(program, &spans, cx) {
            Ok(counts) => counts,
            Err(span) => {
                cx.note(notes::SELF_DEFENDING_NO_USABLE_SPAN, Some(span));
                return;
            }
        };
        cx.changed_by(applied);
        if applied != wanted {
            cx.note(
                notes::planned_and_applied_removals(PASS, wanted, applied),
                None,
            );
        }
    }
}
