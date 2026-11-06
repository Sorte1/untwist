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

fn returns_guarded_apply(function: &Function<'_>, context: &str, fn_name: &str) -> bool {
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let [Statement::IfStatement(statement)] = body.statements.as_slice() else {
        return false;
    };
    if identifier_name(&statement.test) != Some(fn_name) || statement.alternate.is_some() {
        return false;
    }
    let Statement::BlockStatement(block) = &statement.consequent else {
        return false;
    };
    let live = live_statements(&block.body, &[context, fn_name, "arguments"]);
    let [Statement::VariableDeclaration(declaration), Statement::ExpressionStatement(reset), Statement::ReturnStatement(ret)] =
        live.as_slice()
    else {
        return false;
    };
    let Some(declarator) = declaration.declarations.first() else {
        return false;
    };
    let (Some(result), Some(init)) = (binding_name(&declarator.id), declarator.init.as_ref())
    else {
        return false;
    };
    applies_the_guarded_function(init, context, fn_name)
        && assigned_value(reset, fn_name)
            .is_some_and(|value| matches!(value, Expression::NullLiteral(_)))
        && ret.argument.as_ref().and_then(identifier_name) == Some(result)
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
    let [context, guarded] = inner.params.items.as_slice() else {
        return false;
    };
    let (Some(context), Some(fn_name)) = (
        binding_name(&context.pattern),
        binding_name(&guarded.pattern),
    ) else {
        return false;
    };
    let Some(body) = inner.body.as_ref() else {
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

pub fn is_call_controller(expr: &Expression<'_>) -> bool {
    let Some((outer, arguments)) = iife(expr) else {
        return false;
    };
    if !arguments.is_empty() || !outer.params.items.is_empty() {
        return false;
    }
    let Some(body) = outer.body.as_ref() else {
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

#[derive(Default)]
struct Controllers {
    found: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Controllers {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if let Some(symbol) = controller_declaration(stmt) {
                self.found.insert(symbol);
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
}

impl Removals<'_> {
    fn is_shell(&self, stmt: &Statement<'_>) -> bool {
        let Some(body) = shell_statements(stmt) else {
            return false;
        };
        body.iter()
            .all(|inner| is_immediate_controller_call(inner, self.controllers, self.scoping))
    }

    fn record(&mut self, stmt: &Statement<'_>) -> bool {
        if self.is_shell(stmt) || controller_declaration(stmt).is_some() {
            self.spans.push(stmt.span());
            return true;
        }
        if is_immediate_controller_call(stmt, self.controllers, self.scoping) {
            self.spans.push(stmt.span());
            return true;
        }
        if let Some(variable) = controller_variable(stmt, self.controllers, self.scoping) {
            self.spans.push(stmt.span());
            self.variables.insert(variable);
            return true;
        }
        false
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

fn plan(program: &Program<'_>, scoping: &Scoping) -> Plan {
    let mut controllers = Controllers::default();
    controllers.visit_program(program);
    if controllers.found.is_empty() {
        return Plan::Nothing;
    }
    let mut removals = Removals {
        controllers: &controllers.found,
        scoping,
        spans: Vec::new(),
        variables: FxHashSet::default(),
    };
    removals.visit_program(program);
    let Removals {
        mut spans,
        variables,
        ..
    } = removals;
    let mut bare = BareCalls {
        variables: &variables,
        scoping,
        spans: Vec::new(),
    };
    bare.visit_program(program);
    spans.extend(bare.spans);
    let mut tracked = controllers.found;
    tracked.extend(variables);
    if has_stray_reference(program, &tracked, &spans, scoping) {
        return Plan::Refused;
    }
    Plan::Remove(spans)
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
        let spans = match plan(program, cx.scoping()) {
            Plan::Nothing => return,
            Plan::Refused => {
                cx.note(notes::SELF_DEFENDING_REFERENCE_KEPT, None);
                return;
            }
            Plan::Remove(spans) => spans,
        };
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
