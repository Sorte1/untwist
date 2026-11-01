use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::{call_parts, infinite_loop_body, statement_iife};
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::pass::NoteKind;

use crate::notes;
use crate::probe::contains_expression;
use crate::stamp::string_array_unit;

fn is_parse_int_call(expr: &Expression<'_>) -> bool {
    let Some((callee, _)) = call_parts(expr) else {
        return false;
    };
    identifier_name(callee) == Some("parseInt")
}

fn push_shift_target<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k str> {
    let (callee, arguments) = call_parts(expr)?;
    let (object, name) = member_parts(callee)?;
    if name != "push" || arguments.len() != 1 {
        return None;
    }
    let target = identifier_name(object)?;
    let inner = arguments.first()?.as_expression()?;
    let (inner_callee, inner_arguments) = call_parts(inner)?;
    let (inner_object, inner_name) = member_parts(inner_callee)?;
    if inner_name != "shift" || !inner_arguments.is_empty() {
        return None;
    }
    if identifier_name(inner_object)? != target {
        return None;
    }
    Some(target)
}

fn block_has_push_shift(block: &BlockStatement<'_>) -> bool {
    block
        .body
        .iter()
        .any(|stmt| contains_expression(stmt, |expr| push_shift_target(expr).is_some()))
}

fn is_rotator_loop(body: &Statement<'_>) -> bool {
    let Statement::BlockStatement(block) = body else {
        return false;
    };
    let [Statement::TryStatement(statement)] = block.body.as_slice() else {
        return false;
    };
    let Some(handler) = statement.handler.as_ref() else {
        return false;
    };
    block_has_push_shift(&statement.block)
        && block_has_push_shift(&handler.body)
        && block
            .body
            .iter()
            .any(|stmt| contains_expression(stmt, is_parse_int_call))
}

#[derive(Default)]
struct BreaksOut {
    found: bool,
}

impl<'a> VisitJs<'a> for BreaksOut {
    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        self.found |= it.label.is_none();
    }

    fn visit_for_statement(&mut self, _it: &ForStatement<'a>) {}

    fn visit_for_in_statement(&mut self, _it: &ForInStatement<'a>) {}

    fn visit_for_of_statement(&mut self, _it: &ForOfStatement<'a>) {}

    fn visit_while_statement(&mut self, _it: &WhileStatement<'a>) {}

    fn visit_do_while_statement(&mut self, _it: &DoWhileStatement<'a>) {}

    fn visit_switch_statement(&mut self, _it: &SwitchStatement<'a>) {}

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn is_guarded_try(stmt: &Statement<'_>) -> bool {
    let Statement::TryStatement(statement) = stmt else {
        return false;
    };
    let Some(handler) = statement.handler.as_ref() else {
        return false;
    };
    let mut breaks = BreaksOut::default();
    breaks.visit_block_statement(&statement.block);
    breaks.found && block_has_push_shift(&statement.block) && block_has_push_shift(&handler.body)
}

fn is_guarded_rotator_loop(body: &Statement<'_>) -> bool {
    let Statement::BlockStatement(block) = body else {
        return false;
    };
    block.body.iter().any(is_guarded_try)
}

fn has_rotator_loop(stmt: &Statement<'_>) -> bool {
    let Some((function, _)) = statement_iife(stmt) else {
        return false;
    };
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    body.statements
        .iter()
        .filter_map(infinite_loop_body)
        .any(|body| is_rotator_loop(body) || is_guarded_rotator_loop(body))
}

pub fn is_rotator_statement(stmt: &Statement<'_>, array_name: &str) -> bool {
    has_rotator_loop(stmt)
        && contains_expression(stmt, |expr| identifier_name(expr) == Some(array_name))
}

#[derive(Default)]
struct RotatedArray {
    units: FxHashSet<usize>,
}

impl<'a> VisitJs<'a> for RotatedArray {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(unit) = string_array_unit(it.name.as_str()) {
            self.units.insert(unit);
        }
    }
}

pub fn rotator_unit(stmt: &Statement<'_>) -> Option<usize> {
    if !has_rotator_loop(stmt) {
        return None;
    }
    let mut probe = RotatedArray::default();
    probe.visit_statement(stmt);
    let mut units = probe.units.into_iter();
    let unit = units.next()?;
    units.next().is_none().then_some(unit)
}

struct Search<'p> {
    array_name: String,
    found: Option<(Span, &'p Expression<'p>)>,
}

impl<'p> VisitJs<'p> for Search<'p> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        if self.found.is_some() {
            return;
        }
        if is_rotator_statement(it, &self.array_name) {
            let statement: &'p Statement<'p> = self.alloc(it);
            if let Statement::ExpressionStatement(expression) = statement {
                self.found = Some((statement.span(), &expression.expression));
                return;
            }
        }
        walk_js::walk_statement(self, it);
    }
}

pub fn find_rotator<'p>(
    program: &'p Program<'p>,
    array_name: &str,
) -> Option<(Span, &'p Expression<'p>)> {
    let mut search = Search {
        array_name: array_name.to_string(),
        found: None,
    };
    search.visit_program(program);
    search.found
}

pub fn rotator_note(program: &Program<'_>, array_name: &str) -> (String, NoteKind, Option<Span>) {
    match find_rotator(program, array_name) {
        Some((span, _)) => (
            notes::ROTATOR_FOUND.to_string(),
            NoteKind::Progress,
            Some(span),
        ),
        None => (notes::ROTATOR_NONE.to_string(), NoteKind::Progress, None),
    }
}
