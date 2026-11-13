use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;

use super::guard_key::{guard_key, GuardKey};
use super::{settle, Edit, Scan};
use crate::guard_shape::{holder_read_target, may_be_a_guard, read_target};
use crate::parse::{has_error, parse_program};
use crate::print::{compact_function_text, function_text};

const LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];

pub(super) type Guard = (u32, u32, String);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GuardTally {
    pub compacted: u32,
    pub kept: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct GuardCounts {
    compacted: HashMap<GuardKey, u32>,
    kept: HashMap<GuardKey, u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuardRecord {
    counts: Option<Box<GuardCounts>>,
}

fn total(counts: &HashMap<GuardKey, u32>) -> u32 {
    counts
        .values()
        .fold(0, |sum, count| sum.saturating_add(*count))
}

fn unseen(counts: &HashMap<GuardKey, u32>, seen: Option<&HashMap<GuardKey, u32>>) -> u32 {
    counts.iter().fold(0, |sum, (key, count)| {
        let earlier = seen.and_then(|seen| seen.get(key)).copied().unwrap_or(0);
        sum.saturating_add(count.saturating_sub(earlier))
    })
}

fn add(into: &mut HashMap<GuardKey, u32>, key: GuardKey, count: u32) {
    let slot = into.entry(key).or_insert(0);
    *slot = slot.saturating_add(count);
}

impl GuardRecord {
    fn counts_mut(&mut self) -> &mut GuardCounts {
        self.counts.get_or_insert_with(Box::default)
    }

    pub(super) fn note_compacted(&mut self, key: GuardKey) {
        add(&mut self.counts_mut().compacted, key, 1);
    }

    pub(super) fn note_kept(&mut self, key: GuardKey) {
        add(&mut self.counts_mut().kept, key, 1);
    }

    pub fn compacted(&self, text: &str) -> bool {
        let key = guard_key(text);
        self.counts
            .as_ref()
            .is_some_and(|counts| counts.compacted.contains_key(&key))
    }

    pub fn tally(&self) -> GuardTally {
        let Some(counts) = &self.counts else {
            return GuardTally::default();
        };
        GuardTally {
            compacted: total(&counts.compacted),
            kept: total(&counts.kept),
        }
    }

    pub fn tally_beyond(&self, earlier: &GuardRecord) -> GuardTally {
        let Some(counts) = &self.counts else {
            return GuardTally::default();
        };
        let seen = earlier.counts.as_deref();
        GuardTally {
            compacted: unseen(&counts.compacted, seen.map(|seen| &seen.compacted)),
            kept: unseen(&counts.kept, seen.map(|seen| &seen.kept)),
        }
    }

    pub fn absorb(&mut self, other: GuardRecord) {
        let Some(other) = other.counts else {
            return;
        };
        let counts = self.counts_mut();
        for (key, count) in other.compacted {
            add(&mut counts.compacted, key, count);
        }
        for (key, count) in other.kept {
            add(&mut counts.kept, key, count);
        }
    }
}

pub fn guard_text(target: &Function<'_>) -> Option<String> {
    let printed = function_text(target, false);
    holds_a_line_terminator(&printed).then(|| printed.replace('\n', ""))
}

pub(super) fn holds_a_line_terminator(text: &str) -> bool {
    text.contains(LINE_TERMINATORS)
}

fn reprints_the_same(target: &Function<'_>, compact: &str) -> bool {
    let alloc = Allocator::default();
    let wrapped = format!("({compact})");
    let parsed = parse_program(&alloc, &wrapped);
    if has_error(&parsed.diagnostics) {
        return false;
    }
    let [Statement::ExpressionStatement(only)] = parsed.program.body.as_slice() else {
        return false;
    };
    let Expression::FunctionExpression(reparsed) = only.expression.without_parentheses() else {
        return false;
    };
    function_text(reparsed, true) == function_text(target, true)
}

pub(super) fn fixed_point_text(
    target: &Function<'_>,
    compact: &str,
    cascade: bool,
) -> Option<String> {
    let settled = settle(format!("({compact})"), cascade);
    let inner = settled.strip_prefix('(')?.strip_suffix(')')?;
    let sound = !holds_a_line_terminator(inner) && reprints_the_same(target, inner);
    sound.then(|| inner.to_string())
}

pub(super) fn may_hold_a_guard(init: Option<&Expression<'_>>) -> bool {
    matches!(
        init,
        Some(Expression::FunctionExpression(_) | Expression::CallExpression(_))
    )
}

fn assigned_guard<'k, 'a>(statement: &'k ExpressionStatement<'a>) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assign) = &statement.expression else {
        return None;
    };
    let plain = assign.operator == AssignmentOperator::Assign
        && matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_));
    (plain && may_hold_a_guard(Some(&assign.right))).then_some(&assign.right)
}

pub(super) fn statement_guard<'k, 'a>(
    statement: &'k ExpressionStatement<'a>,
) -> Option<&'k Expression<'a>> {
    assigned_guard(statement).or_else(|| called_guard(&statement.expression))
}

fn called_guard<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expr.without_parentheses() {
        call @ Expression::CallExpression(inner)
            if matches!(
                inner.callee.without_parentheses(),
                Expression::FunctionExpression(_)
            ) =>
        {
            Some(call)
        }
        Expression::UnaryExpression(not) if not.operator == UnaryOperator::LogicalNot => {
            match not.argument.without_parentheses() {
                call @ Expression::CallExpression(_) => called_guard(call),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(super) fn guard_candidate<'k, 'a>(init: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    match init {
        Expression::FunctionExpression(function) => Some(function),
        Expression::CallExpression(call) => called_function(&call.callee),
        _ => None,
    }
}

fn called_function<'k, 'a>(callee: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    match callee.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function),
        Expression::CallExpression(inner) => match inner.callee.without_parentheses() {
            Expression::FunctionExpression(function) => Some(function),
            _ => None,
        },
        Expression::SequenceExpression(sequence) => match sequence.expressions.last() {
            Some(Expression::FunctionExpression(function)) => Some(function),
            _ => None,
        },
        _ => None,
    }
}

struct Slots<'s, 'c, const GUARDS: bool> {
    scan: &'s mut Scan<'c, GUARDS>,
}

impl<const GUARDS: bool> Slots<'_, '_, GUARDS> {
    fn holds_a_candidate(&self, span: Span) -> bool {
        let candidates = &self.scan.candidates;
        let at = candidates.partition_point(|start| *start < span.start);
        candidates.get(at).is_some_and(|start| *start < span.end)
    }
}

impl<'a, const GUARDS: bool> VisitJs<'a> for Slots<'_, '_, GUARDS> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.holds_a_candidate(it.span()) {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.holds_a_candidate(it.span()) {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if may_hold_a_guard(it.init.as_ref()) {
            if let Some(init) = &it.init {
                self.scan.settle_guard(it.span.start, init);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Some(init) = statement_guard(it) {
            self.scan.settle_guard(it.span.start, init);
        }
        walk_js::walk_expression_statement(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        if let Some(target) = holder_read_target(it) {
            self.scan.settle_holder(target);
        }
        walk_js::walk_object_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(target) = handed_out(it) {
            self.scan.settle_holder(target);
        }
        walk_js::walk_function(self, it, flags);
    }
}

pub(super) fn handed_out<'k, 'a>(function: &'k Function<'a>) -> Option<&'k Function<'a>> {
    if !may_be_a_guard(function) {
        return None;
    }
    read_target(function)
}

impl<const GUARDS: bool> Scan<'_, GUARDS> {
    pub(super) fn settle_slots(&mut self, program: &Program<'_>) {
        self.candidates.sort_unstable();
        Slots { scan: self }.visit_program(program);
    }

    #[inline(never)]
    pub(super) fn note_candidate(&mut self, function: &Function<'_>) {
        if read_target(function).is_some() {
            self.candidates.push(function.span.start);
        }
    }

    fn settle_guard(&mut self, start: u32, init: &Expression<'_>) {
        if start < self.searched_until {
            return;
        }
        let Some(function) = guard_candidate(init) else {
            return;
        };
        if !may_be_a_guard(function) {
            return;
        }
        if let Some(target) = read_target(function) {
            self.searched_until = target.span.end;
            self.compacts(target);
        }
    }

    fn settle_holder(&mut self, target: &Function<'_>) {
        if target.span.start < self.searched_until {
            return;
        }
        self.searched_until = target.span.end;
        self.compacts(target);
    }

    fn compacts(&mut self, target: &Function<'_>) {
        let (start, end) = (target.span.start, target.span.end);
        let Some(printed) = self.code.get(start as usize..end as usize) else {
            return;
        };
        if !holds_a_line_terminator(printed) {
            return;
        }
        let compact = compact_function_text(target);
        let key = guard_key(&compact);
        match fixed_point_text(target, &compact, self.cascade) {
            Some(text) => {
                self.guards.push((start, end, text));
                self.record.note_compacted(key);
            }
            None => self.record.note_kept(key),
        }
    }
}

impl Edit {
    fn bounds(&self) -> (u32, u32) {
        match *self {
            Edit::Insert(at) => (at, at),
            Edit::Collapse(from, to) => (from, to),
        }
    }

    fn shift(&mut self, delta: i64) -> bool {
        let moved = |offset: &mut u32| match u32::try_from(i64::from(*offset) + delta) {
            Ok(value) => {
                *offset = value;
                true
            }
            Err(_) => false,
        };
        match self {
            Edit::Insert(at) => moved(at),
            Edit::Collapse(from, to) => moved(from) && moved(to),
        }
    }
}

fn growth((start, end, text): &Guard) -> i64 {
    i64::try_from(text.len()).unwrap_or(i64::MAX) - i64::from(end.saturating_sub(*start))
}

struct Spliced {
    bounds: Vec<(u32, u32)>,
    growth_before: Vec<i64>,
}

impl Spliced {
    fn delta_before(&self, from: u32, to: u32) -> Option<i64> {
        let count = self.bounds.partition_point(|(start, _)| *start < to);
        let overlaps = count
            .checked_sub(1)
            .and_then(|last| self.bounds.get(last))
            .is_some_and(|(_, end)| from < *end);
        if overlaps {
            return None;
        }
        self.growth_before.get(count).copied()
    }

    fn reposition(&self, edit: &mut Edit, first: u32, last: u32) -> bool {
        let (from, to) = edit.bounds();
        if to <= first {
            return true;
        }
        let delta = if from >= last {
            self.growth_before.last().copied()
        } else {
            self.delta_before(from, to)
        };
        delta.is_some_and(|delta| edit.shift(delta))
    }
}

pub(super) fn splice_guards(
    code: String,
    mut edits: Vec<Edit>,
    mut guards: Vec<Guard>,
) -> (String, Vec<Edit>, bool) {
    guards.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(code.len());
    let mut cursor = 0;
    let mut spliced = Spliced {
        bounds: Vec::with_capacity(guards.len()),
        growth_before: Vec::with_capacity(guards.len() + 1),
    };
    let mut grown = 0;
    spliced.growth_before.push(grown);
    for guard in guards {
        let (from, to) = (guard.0 as usize, guard.1 as usize);
        if from < cursor || code.get(from..to).is_none() {
            continue;
        }
        out.push_str(&code[cursor..from]);
        out.push_str(&guard.2);
        cursor = to;
        grown += growth(&guard);
        spliced.bounds.push((guard.0, guard.1));
        spliced.growth_before.push(grown);
    }
    out.push_str(&code[cursor..]);
    let (Some(first), Some(last)) = (spliced.bounds.first(), spliced.bounds.last()) else {
        return (out, edits, false);
    };
    let (first, last) = (first.0, last.1);
    edits.retain_mut(|edit| spliced.reposition(edit, first, last));
    (out, edits, true)
}
