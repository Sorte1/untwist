use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::global::{global_is_written, is_global_reference};
use untwist_core::ast_util::matcher::{call_parts, statement_iife, var_function_or_declaration};
use untwist_core::parse::parse_program;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::bindings::{
    has_stray_reference, reaches_a_script_global_once_removed, reference_symbol,
};
use crate::control_flow_object::single_declarator;
use crate::directives::remove_protection_statements;
use crate::notes;
use crate::protection_shape::{is_interval_function, is_protection_function};

pub const PASS: &str = "debug-protection";

const TIMER: &str = "setInterval";
const GLOBAL_OBJECTS: [&str; 3] = ["window", "globalThis", "self"];

fn declared_symbol(stmt: &Statement<'_>) -> Option<SymbolId> {
    match stmt {
        Statement::FunctionDeclaration(function) => function.id.as_ref()?.symbol_id.get(),
        Statement::VariableDeclaration(_) => single_declarator(stmt)?.0.symbol_id.get(),
        _ => None,
    }
}

fn has_protection_outline(function: &Function<'_>) -> bool {
    function
        .body
        .as_ref()
        .is_some_and(|body| matches!(body.statements.as_slice(), [_, Statement::TryStatement(_)]))
}

fn debug_protection_declaration(stmt: &Statement<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let declaration = var_function_or_declaration(stmt)?;
    if declaration.function.params.items.len() != 1 || !has_protection_outline(declaration.function)
    {
        return None;
    }
    if !is_protection_function(declaration.function, scoping) {
        return None;
    }
    declared_symbol(stmt)
}

fn is_intact_global(expr: &Expression<'_>, names: &[&str], scoping: &Scoping) -> bool {
    let Expression::Identifier(id) = expr else {
        return false;
    };
    let name = id.name.as_str();
    names.contains(&name) && is_global_reference(scoping, id) && !global_is_written(scoping, name)
}

fn is_global_timer(callee: &Expression<'_>, scoping: &Scoping) -> bool {
    match callee {
        Expression::StaticMemberExpression(member) => {
            !member.optional
                && member.property.name == TIMER
                && is_intact_global(&member.object, &GLOBAL_OBJECTS, scoping)
        }
        other => is_intact_global(other, &[TIMER], scoping),
    }
}

struct Intervals<'k> {
    protections: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    spans: Vec<Span>,
}

impl Intervals<'_> {
    fn is_interval_statement(&self, stmt: &Statement<'_>) -> bool {
        let Statement::ExpressionStatement(stmt) = stmt else {
            return false;
        };
        let Some((callee, arguments)) = call_parts(&stmt.expression) else {
            return false;
        };
        let [protection, Argument::NumericLiteral(_)] = arguments.as_slice() else {
            return false;
        };
        protection
            .as_expression()
            .and_then(|argument| reference_symbol(argument, self.scoping))
            .is_some_and(|symbol| self.protections.contains(&symbol))
            && is_global_timer(callee, self.scoping)
    }

    fn is_interval_shell(&self, stmt: &Statement<'_>) -> bool {
        let Some((function, arguments)) = statement_iife(stmt) else {
            return false;
        };
        if !arguments.is_empty() || !function.params.items.is_empty() {
            return false;
        }
        if is_interval_function(function, self.protections, self.scoping) {
            return true;
        }
        function.body.as_ref().is_some_and(|body| {
            !function.r#async
                && !function.generator
                && !body.statements.is_empty()
                && body
                    .statements
                    .iter()
                    .all(|inner| self.is_interval_statement(inner))
        })
    }
}

impl<'a> VisitJs<'a> for Intervals<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if self.is_interval_shell(stmt) || self.is_interval_statement(stmt) {
                self.spans.push(stmt.span());
            } else {
                walk_js::walk_statement(self, stmt);
            }
        }
    }
}

struct Declarations<'k> {
    scoping: &'k Scoping,
    spans: Vec<Span>,
    symbols: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Declarations<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for stmt in it {
            if let Some(symbol) = debug_protection_declaration(stmt, self.scoping) {
                self.spans.push(stmt.span());
                self.symbols.insert(symbol);
            }
        }
        walk_js::walk_statements(self, it);
    }
}

fn declarations<'k>(program: &Program<'_>, scoping: &'k Scoping) -> Declarations<'k> {
    let mut declarations = Declarations {
        scoping,
        spans: Vec::new(),
        symbols: FxHashSet::default(),
    };
    declarations.visit_program(program);
    declarations
}

pub(crate) fn protection_symbols(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    declarations(program, scoping).symbols
}

pub(crate) fn keeps_a_live_protection(output: &str) -> bool {
    let allocator = Allocator::default();
    let parsed = parse_program(&allocator, output);
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let declarations = declarations(&parsed.program, &scoping);
    !declarations.symbols.is_empty()
        && has_stray_reference(
            &parsed.program,
            &declarations.symbols,
            &declarations.spans,
            &scoping,
        )
}

enum Plan {
    Nothing,
    Refused,
    KeptWithController,
    Remove(Vec<Span>, FxHashSet<SymbolId>, bool),
}

fn plan(program: &Program<'_>, scoping: &Scoping, unsafe_lane: bool) -> Plan {
    let declarations = declarations(program, scoping);
    if declarations.symbols.is_empty() {
        return Plan::Nothing;
    }
    let mut intervals = Intervals {
        protections: &declarations.symbols,
        scoping,
        spans: Vec::new(),
    };
    intervals.visit_program(program);
    let provisional = !unsafe_lane && !intervals.spans.is_empty();
    if provisional && crate::self_defending::partial_removals_forbidden() {
        return Plan::KeptWithController;
    }
    let mut spans = declarations.spans;
    spans.extend(intervals.spans);
    if has_stray_reference(program, &declarations.symbols, &spans, scoping) {
        return Plan::Refused;
    }
    Plan::Remove(spans, declarations.symbols, provisional)
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
        let (spans, provisional) = match plan(program, cx.scoping(), cx.opts.unsafe_transforms) {
            Plan::Nothing => return,
            Plan::KeptWithController => {
                cx.note(notes::DEBUG_PROTECTION_KEPT_WITH_CONTROLLER, None);
                return;
            }
            Plan::Refused => {
                cx.note(notes::DEBUG_PROTECTION_REFERENCE_KEPT, None);
                return;
            }
            Plan::Remove(spans, symbols, provisional) => {
                if reaches_a_script_global_once_removed(&symbols, &spans, program, cx) {
                    cx.note(notes::DEBUG_PROTECTION_GLOBAL_REACH, None);
                    return;
                }
                (spans, provisional)
            }
        };
        if crate::directives::removes_what_an_eval_reads(cx.scoping(), &spans) {
            cx.note(notes::DEBUG_PROTECTION_REFERENCE_KEPT, None);
            return;
        }
        if crate::directives::removes_hoist_blocker(cx.scoping(), &spans) {
            cx.note(notes::PROTECTION_HOIST_BLOCKER_KEPT, None);
            return;
        }
        let (wanted, applied) = match remove_protection_statements(program, &spans, cx) {
            Ok(counts) => counts,
            Err(span) => {
                cx.note(notes::DEBUG_PROTECTION_NO_USABLE_SPAN, Some(span));
                return;
            }
        };
        cx.changed_by(applied);
        if provisional && applied > 0 {
            crate::self_defending::note_provisional_removal();
        }
        if applied != wanted {
            cx.note(
                notes::planned_and_applied_removals(PASS, wanted, applied),
                None,
            );
        }
    }
}
