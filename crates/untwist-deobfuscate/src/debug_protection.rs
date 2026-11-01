use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::{call_parts, iife, var_function_or_declaration};
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::pass::{Cx, Interest, Pass};

use crate::bindings::{has_stray_reference, reference_symbol};
use crate::control_flow_object::single_declarator;
use crate::directives::remove_protection_statements;
use crate::notes;
use crate::probe::contains_expression;

pub const PASS: &str = "debug-protection";

fn is_constructed_debugger(expr: &Expression<'_>) -> bool {
    let Some((callee, arguments)) = call_parts(expr) else {
        return false;
    };
    let Some((_, name)) = member_parts(callee) else {
        return false;
    };
    name == "constructor"
        && arguments.len() == 1
        && matches!(
            arguments[0].as_expression(),
            Some(Expression::StringLiteral(literal)) if literal.value.as_str() == "debugger"
        )
}

fn is_debugger_marker(stmt: &Statement<'_>) -> bool {
    if matches!(stmt, Statement::DebuggerStatement(_)) {
        return true;
    }
    match stmt {
        Statement::BlockStatement(block) if block.body.iter().any(is_debugger_marker) => {
            return true
        }
        Statement::IfStatement(branch) => {
            if is_debugger_marker(&branch.consequent) {
                return true;
            }
            if branch.alternate.as_ref().is_some_and(is_debugger_marker) {
                return true;
            }
        }
        _ => {}
    }
    contains_expression(stmt, is_constructed_debugger)
}

fn is_protection_guard(body: &[Statement<'_>], inner: &str, outer: &Function<'_>) -> Option<()> {
    let [Statement::IfStatement(branch)] = body else {
        return None;
    };
    let BindingPattern::BindingIdentifier(parameter) = &outer.params.items.first()?.pattern else {
        return None;
    };
    if identifier_name(&branch.test) != Some(parameter.name.as_str()) {
        return None;
    }
    let Statement::BlockStatement(consequent) = &branch.consequent else {
        return None;
    };
    let [Statement::ReturnStatement(returned)] = consequent.body.as_slice() else {
        return None;
    };
    if returned.argument.as_ref().and_then(identifier_name) != Some(inner) {
        return None;
    }
    let Some(Statement::BlockStatement(alternate)) = branch.alternate.as_ref() else {
        return None;
    };
    let [Statement::ExpressionStatement(call)] = alternate.body.as_slice() else {
        return None;
    };
    let (callee, arguments) = call_parts(&call.expression)?;
    (identifier_name(callee) == Some(inner) && arguments.len() == 1).then_some(())
}

fn is_protection_body<'k, 'a>(function: &'k Function<'a>) -> Option<&'k str> {
    let body = function.body.as_ref()?;
    let [inner, Statement::TryStatement(guard)] = body.statements.as_slice() else {
        return None;
    };
    let declaration = var_function_or_declaration(inner)?;
    let inner_body = declaration.function.body.as_ref()?;
    let [Statement::IfStatement(branch), Statement::ExpressionStatement(recurse)] =
        inner_body.statements.as_slice()
    else {
        return None;
    };
    let alternate = branch.alternate.as_ref()?;
    if !is_debugger_marker(alternate) {
        return None;
    }
    let (callee, arguments) = call_parts(&recurse.expression)?;
    if identifier_name(callee) != Some(declaration.name) || arguments.len() != 1 {
        return None;
    }
    guard.handler.as_ref()?;
    is_protection_guard(&guard.block.body, declaration.name, function)?;
    Some(declaration.name)
}

fn declared_symbol(stmt: &Statement<'_>) -> Option<SymbolId> {
    match stmt {
        Statement::FunctionDeclaration(function) => function.id.as_ref()?.symbol_id.get(),
        Statement::VariableDeclaration(_) => single_declarator(stmt)?.0.symbol_id.get(),
        _ => None,
    }
}

fn debug_protection_declaration(stmt: &Statement<'_>) -> Option<SymbolId> {
    let declaration = var_function_or_declaration(stmt)?;
    if declaration.function.params.items.len() != 1 {
        return None;
    }
    is_protection_body(declaration.function)?;
    declared_symbol(stmt)
}

fn is_interval_call(
    expr: &Expression<'_>,
    protections: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    let Some((callee, arguments)) = call_parts(expr) else {
        return false;
    };
    let Some((_, property)) = member_parts(callee) else {
        return false;
    };
    property == "setInterval"
        && arguments.len() == 2
        && arguments[0]
            .as_expression()
            .and_then(|argument| reference_symbol(argument, scoping))
            .is_some_and(|symbol| protections.contains(&symbol))
        && matches!(
            arguments[1].as_expression(),
            Some(Expression::NumericLiteral(_))
        )
}

fn is_interval_iife(
    stmt: &Statement<'_>,
    protections: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    let Statement::ExpressionStatement(statement) = stmt else {
        return false;
    };
    let Some((function, arguments)) = iife(&statement.expression) else {
        return false;
    };
    if !arguments.is_empty() {
        return false;
    }
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    body.statements.iter().any(|inner| {
        contains_expression(inner, |expr| is_interval_call(expr, protections, scoping))
    })
}

#[derive(Default)]
struct Declarations {
    spans: Vec<Span>,
    symbols: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Declarations {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if let Some(symbol) = debug_protection_declaration(stmt) {
                self.spans.push(stmt.span());
                self.symbols.insert(symbol);
            }
        }
        walk_js::walk_statements(self, it);
    }
}

struct Intervals<'k> {
    protections: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    spans: Vec<Span>,
}

impl<'a> VisitJs<'a> for Intervals<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if is_interval_iife(stmt, self.protections, self.scoping) {
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
    let mut declarations = Declarations::default();
    declarations.visit_program(program);
    if declarations.symbols.is_empty() {
        return Plan::Nothing;
    }
    let mut intervals = Intervals {
        protections: &declarations.symbols,
        scoping,
        spans: Vec::new(),
    };
    intervals.visit_program(program);
    let mut spans = declarations.spans;
    spans.extend(intervals.spans);
    if has_stray_reference(program, &declarations.symbols, &spans, scoping) {
        return Plan::Refused;
    }
    Plan::Remove(spans)
}

#[derive(Default)]
pub struct DebugProtection;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DebugProtection)
}

impl<'a> Pass<'a> for DebugProtection {
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
                cx.note(notes::DEBUG_PROTECTION_REFERENCE_KEPT, None);
                return;
            }
            Plan::Remove(spans) => spans,
        };
        let (wanted, applied) = match remove_protection_statements(program, &spans, cx) {
            Ok(counts) => counts,
            Err(span) => {
                cx.note(notes::DEBUG_PROTECTION_NO_USABLE_SPAN, Some(span));
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
