use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::literal::{is_false_literal, is_true_literal};
use untwist_core::ast_util::matcher::{anonymous_function, call_parts, iife};
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::pass::{Cx, Interest, Pass};

use crate::bindings::{has_stray_reference, reference_symbol};
use crate::control_flow_object::single_declarator;
use crate::directives::remove_protection_statements;
use crate::notes;
use crate::protection_shape::{is_protection_callback, Roles};

pub const PASS: &str = "self-defending";

fn binding_name<'k>(pattern: &'k BindingPattern<'_>) -> Option<&'k str> {
    let BindingPattern::BindingIdentifier(id) = pattern else {
        return None;
    };
    Some(id.name.as_str())
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
            let declarator = declaration.declarations.first()?;
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
        && ![context, fn_name].contains(&result)
}

fn is_empty_function(function: &Function<'_>) -> bool {
    function.params.items.is_empty()
        && function
            .body
            .as_ref()
            .is_some_and(|body| body.statements.is_empty())
}

fn chooses_the_guarded_function(init: &Expression<'_>, flag: &str, names: (&str, &str)) -> bool {
    let Expression::ConditionalExpression(choice) = init else {
        return false;
    };
    if identifier_name(&choice.test) != Some(flag) {
        return false;
    }
    let (Some(taken), Some(skipped)) = (
        anonymous_function(&choice.consequent),
        anonymous_function(&choice.alternate),
    ) else {
        return false;
    };
    taken.params.items.is_empty()
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
    let [context, guarded] = params.items.as_slice() else {
        return false;
    };
    let (Some(context), Some(fn_name)) = (
        binding_name(&context.pattern),
        binding_name(&guarded.pattern),
    ) else {
        return false;
    };
    let Some(body) = body else {
        return false;
    };
    let live = live_statements(&body.statements, &[flag]);
    let [Statement::VariableDeclaration(declaration), Statement::ExpressionStatement(reset), Statement::ReturnStatement(ret)] =
        live.as_slice()
    else {
        return false;
    };
    let Some(declarator) = declaration.declarations.first() else {
        return false;
    };
    let (Some(rfn), Some(init)) = (binding_name(&declarator.id), declarator.init.as_ref()) else {
        return false;
    };
    chooses_the_guarded_function(init, flag, (context, fn_name))
        && assigned_value(reset, flag).is_some_and(is_false_literal)
        && ret.argument.as_ref().and_then(identifier_name) == Some(rfn)
}

fn controller_body<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k FunctionBody<'a>> {
    if let Some((outer, arguments)) = iife(expr) {
        if !arguments.is_empty() || !outer.params.items.is_empty() {
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
    let Some(declarator) = first_call.declarations.first() else {
        return false;
    };
    let Some(flag) = binding_name(&declarator.id) else {
        return false;
    };
    if !declarator.init.as_ref().is_some_and(is_true_literal) {
        return false;
    }
    let Some(inner) = ret.argument.as_ref().and_then(anonymous_function) else {
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

fn is_bare_call(stmt: &Statement<'_>, variables: &FxHashSet<SymbolId>, scoping: &Scoping) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(&stmt.expression) else {
        return false;
    };
    arguments.is_empty()
        && reference_symbol(callee, scoping).is_some_and(|symbol| variables.contains(&symbol))
}

fn shell_statements<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k AVec<'a, Statement<'a>>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let (function, arguments) = iife(&stmt.expression)?;
    if !arguments.is_empty() || !function.params.items.is_empty() {
        return None;
    }
    Some(&function.body.as_ref()?.statements)
}

fn set_flag<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
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
        .then_some(target.name.as_str())
}

fn returner(expr: &Expression<'_>, flags: &[&str]) -> bool {
    let (params, body) = match expr.without_parentheses() {
        Expression::FunctionExpression(function) if function.id.is_none() => {
            (&function.params, function.body.as_deref())
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => (&arrow.params, Some(&**body)),
            _ => return false,
        },
        _ => return false,
    };
    flags
        .iter()
        .any(|flag| returner_matches(params, body, flag))
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
    returner_variables: FxHashSet<SymbolId>,
}

fn returner_variable(stmt: &Statement<'_>, flags: &[&str]) -> Option<SymbolId> {
    if flags.is_empty() {
        return None;
    }
    let (id, init) = single_declarator(stmt)?;
    let (callee, arguments) = call_parts(init?)?;
    let made =
        returner(callee.without_parentheses(), flags) || crate::mutants::inline_returners_trusted();
    (arguments.len() == 2 && made)
        .then(|| id.symbol_id.get())
        .flatten()
}

fn inline_controller_call(stmt: &Statement<'_>) -> bool {
    let Some((_, Some(init))) = single_declarator(stmt) else {
        return false;
    };
    call_parts(init).is_some_and(|(callee, _)| is_call_controller(callee.without_parentheses()))
}

impl<'a> VisitJs<'a> for Controllers {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        let flags: Vec<&str> = it.iter().filter_map(set_flag).collect();
        for stmt in it {
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
            if let Some(variable) = returner_variable(stmt, &flags) {
                self.inline = true;
                self.returner_calls.push(stmt.span());
                self.returner_variables.insert(variable);
            }
        }
        walk_js::walk_statements(self, it);
    }
}

struct Removals<'k> {
    controllers: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    spans: Vec<Span>,
    variables: FxHashSet<SymbolId>,
    returner_calls: &'k [Span],
    proof: Option<&'k FxHashSet<SymbolId>>,
    unproven: FxHashSet<SymbolId>,
    kept: bool,
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

    fn record(&mut self, stmt: &Statement<'_>) -> bool {
        let declared = single_declarator(stmt)
            .and_then(|(id, _)| id.symbol_id.get())
            .is_some_and(|symbol| self.controllers.contains(&symbol));
        if self.is_shell(stmt) || declared {
            self.spans.push(stmt.span());
            return true;
        }
        let registers = is_immediate_controller_call(stmt, self.controllers, self.scoping)
            || self.returner_calls.contains(&stmt.span());
        let variable = controller_variable(stmt, self.controllers, self.scoping);
        if !(registers || variable.is_some()) {
            return false;
        }
        if !self.runs_only_protection(stmt) {
            self.keep(stmt);
            return false;
        }
        self.spans.push(stmt.span());
        self.variables.extend(variable);
        true
    }

    fn keep(&mut self, stmt: &Statement<'_>) {
        let controller = registration(stmt)
            .and_then(|(_, call)| call_parts(call.without_parentheses()))
            .and_then(|(callee, _)| reference_symbol(callee, self.scoping));
        self.unproven.extend(controller);
        self.kept = true;
    }
}

impl<'a> VisitJs<'a> for Removals<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if !self.record(stmt) {
                walk_js::walk_statement(self, stmt);
            }
        }
    }
}

struct BareCalls<'k> {
    variables: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    spans: Vec<Span>,
}

impl<'a> VisitJs<'a> for BareCalls<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if is_bare_call(stmt, self.variables, self.scoping) {
                self.spans.push(stmt.span());
            }
        }
        walk_js::walk_statements(self, it);
    }
}

enum Plan {
    Nothing,
    Refused,
    Remove(Vec<Span>),
}

fn removals<'k>(
    program: &Program<'_>,
    scoping: &'k Scoping,
    controllers: &'k FxHashSet<SymbolId>,
    returner_calls: &'k [Span],
    proof: Option<&'k FxHashSet<SymbolId>>,
) -> Removals<'k> {
    let mut removals = Removals {
        controllers,
        scoping,
        spans: Vec::new(),
        variables: FxHashSet::default(),
        returner_calls,
        proof,
        unproven: FxHashSet::default(),
        kept: false,
    };
    removals.visit_program(program);
    removals
}

fn removed_declaration(spans: &[Span], declared: Span) -> bool {
    spans
        .iter()
        .any(|span| span.start <= declared.start && declared.end <= span.end)
}

fn plan(program: &Program<'_>, scoping: &Scoping, unsafe_lane: bool) -> Plan {
    let mut controllers = Controllers::default();
    controllers.visit_program(program);
    if controllers.found.is_empty() && !controllers.inline {
        return Plan::Nothing;
    }
    let protections =
        (!unsafe_lane).then(|| crate::debug_protection::protection_symbols(program, scoping));
    let proof = protections.as_ref();
    let calls = controllers.returner_calls.as_slice();
    let first = removals(program, scoping, &controllers.found, calls, proof);
    let kept = first.kept;
    let proven: FxHashSet<SymbolId> = controllers
        .found
        .difference(&first.unproven)
        .copied()
        .collect();
    let Removals {
        mut spans,
        mut variables,
        ..
    } = if first.unproven.is_empty() {
        first
    } else {
        removals(program, scoping, &proven, calls, proof)
    };
    variables.extend(
        controllers
            .returner_variables
            .iter()
            .copied()
            .filter(|&variable| removed_declaration(&spans, scoping.symbol_span(variable))),
    );
    let mut bare = BareCalls {
        variables: &variables,
        scoping,
        spans: Vec::new(),
    };
    bare.visit_program(program);
    spans.extend(bare.spans);
    let mut tracked = proven.clone();
    tracked.extend(variables);
    if has_stray_reference(program, &tracked, &spans, scoping) {
        return Plan::Refused;
    }
    match (spans.is_empty(), kept) {
        (false, _) => Plan::Remove(spans),
        (true, true) => Plan::Refused,
        (true, false) => Plan::Nothing,
    }
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
            Plan::Remove(spans) => spans,
        };
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
