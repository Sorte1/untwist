use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::matcher::is_literal_argument;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::search::{expression_at, expressions_at, statement_at};
use untwist_core::pass::Cx;
use untwist_core::semantic::{
    is_constant_binding, is_constant_binding_in, read_reference_count, write_reference_count,
    AnnexBWrites,
};

use crate::call_checks::call_checks_apply;
use crate::init_order::{scan_init_order, DeclarationSlot, InitOrder};
use crate::mutants::external_write_order_trusted;
use crate::notes;
use crate::probe::{expression_contains, holds_own_scope};
use crate::stamp::decoder_index;
use crate::units::MachineryDepth;

use super::global_bindings::global_bindings;
use super::sequenced::{chained_literal, ordered_values, strip_writes};
use super::variables::{variable_facts, Assignment, VariableFacts, Variables};
use super::{decoder_call, is_splittable_branch};

#[derive(Default, Hash)]
struct Pending {
    ternaries: Vec<(Span, Span, Span, Span, usize)>,
    externals: Vec<SymbolId>,
    bare: Vec<Span>,
}

struct Scan<'s> {
    scoping: &'s Scoping,
    units: MachineryDepth,
    pending: Pending,
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let entered = self.units.enter(it);
        if !self.units.inside() {
            if let Statement::ExpressionStatement(stmt) = it {
                if identifier_name(&stmt.expression)
                    .and_then(decoder_index)
                    .is_some()
                {
                    self.pending.bare.push(it.span());
                }
            }
        }
        walk_js::walk_statement(self, it);
        self.units.leave(entered);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.units.inside() {
            walk_js::walk_expression(self, it);
            return;
        }
        let Some((index, call)) = decoder_call(it) else {
            walk_js::walk_expression(self, it);
            return;
        };
        if let [Argument::ConditionalExpression(conditional)] = call.arguments.as_slice() {
            self.pending.ternaries.push((
                it.span(),
                conditional.test.span(),
                conditional.consequent.span(),
                conditional.alternate.span(),
                index,
            ));
            for branch in [&conditional.consequent, &conditional.alternate] {
                if !is_splittable_branch(branch) {
                    collect_symbols(branch, self.scoping, &mut self.pending.externals);
                }
            }
            walk_js::walk_expression(self, it);
            return;
        }
        for argument in &call.arguments {
            let Some(expr) = argument.as_expression() else {
                continue;
            };
            if is_literal_argument(expr) {
                continue;
            }
            collect_symbols(expr, self.scoping, &mut self.pending.externals);
        }
        walk_js::walk_expression(self, it);
    }
}

fn scan_pending(program: &Program<'_>, scoping: &Scoping) -> Pending {
    let mut scan = Scan {
        scoping,
        units: MachineryDepth::every_unit(),
        pending: Pending::default(),
    };
    scan.visit_program(program);
    scan.pending
}

fn pending_after(
    changes: usize,
    pending: Pending,
    program: &Program<'_>,
    scoping: &Scoping,
) -> Pending {
    let pending = if changes == 0 {
        pending
    } else {
        scan_pending(program, scoping)
    };
    #[cfg(debug_assertions)]
    crate::fused::check::record("pending", crate::fused::check::digest_of(&pending));
    pending
}

fn collect_symbols(expr: &Expression<'_>, scoping: &Scoping, out: &mut Vec<SymbolId>) {
    let mut probe = SymbolProbe { scoping, out };
    probe.visit_expression(expr);
}

struct SymbolProbe<'s> {
    scoping: &'s Scoping,
    out: &'s mut Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for SymbolProbe<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        {
            self.out.push(symbol);
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn note_scoped_ternaries(program: &Program<'_>, pending: &Pending, cx: &mut Cx<'_, '_>) {
    let calls: Vec<Span> = pending.ternaries.iter().map(|ternary| ternary.0).collect();
    for (span, call) in calls.iter().zip(expressions_at(program, &calls)) {
        if call.is_some_and(|call| expression_contains(call, holds_own_scope)) {
            cx.note(notes::TERNARY_HOLDS_A_SCOPE, Some(*span));
        }
    }
}

#[cfg(debug_assertions)]
fn facts_digest(facts: &VariableFacts<'_>, symbol: SymbolId) -> u64 {
    use crate::fused::check;
    let members = check::sorted(
        facts
            .members
            .iter()
            .map(|(span, list)| ((span.start, span.end), *list)),
    );
    check::digest_of(&(
        symbol.index(),
        (facts.statement, facts.declaration, facts.init),
        (facts.assignments, facts.references),
        members,
    ))
}

fn is_sole_assignment(
    view: &Program<'_>,
    statement: Span,
    symbol: SymbolId,
    scoping: &Scoping,
) -> bool {
    let Some(Statement::ExpressionStatement(stmt)) = statement_at(view, statement) else {
        return false;
    };
    let Expression::AssignmentExpression(assignment) = &stmt.expression else {
        return false;
    };
    if assignment.operator != AssignmentOperator::Assign || !is_literal_argument(&assignment.right)
    {
        return false;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return false;
    };
    target
        .reference_id
        .get()
        .and_then(|id| scoping.get_reference(id).symbol_id())
        == Some(symbol)
}

fn references_run_after_the_initialiser(
    facts: &VariableFacts<'_>,
    order: &InitOrder,
) -> Option<Span> {
    let statement = facts.statement?;
    let slot = order.declaration_slot(statement)?;
    facts
        .references
        .iter()
        .all(|reference| order.reference_is_initialised_before_use(slot, reference.span))
        .then_some(statement)
}

fn note_init_order_budget(
    order: &InitOrder,
    mark: usize,
    symbol: SymbolId,
    span: Option<Span>,
    cx: &mut Cx<'_, '_>,
) {
    if order.budget_exhausted() == mark {
        return;
    }
    let name = cx.scoping().symbol_name(symbol).to_string();
    cx.note(notes::init_order_budget(&name), span);
}

fn inline_constant_variable<'a>(
    view: &Program<'a>,
    facts: &VariableFacts<'_>,
    symbol: SymbolId,
    order: &InitOrder,
    cx: &mut Cx<'a, '_>,
    edits: &mut ExprEdits<'a>,
    removals: &mut StmtEdits<'a>,
) {
    let Some(init) = facts.init else {
        return;
    };
    let Some(init) = expression_at(view, init).filter(|expr| is_literal_argument(expr)) else {
        cx.note(notes::NO_LITERAL_INITIALISER, facts.declaration);
        return;
    };
    if facts.rebound && call_checks_apply(cx.opts) {
        cx.note(notes::BLOCK_FUNCTION_NAME, facts.statement);
        return;
    }
    let mark = order.budget_exhausted();
    if references_run_after_the_initialiser(facts, order).is_none() {
        cx.note(notes::USE_BEFORE_INITIALISATION, facts.statement);
        note_init_order_budget(order, mark, symbol, facts.statement, cx);
        return;
    }
    for reference in facts.references {
        edits.insert(reference.span, init.clone_in(cx.allocator()));
    }
    let Some(declaration) = facts.declaration else {
        return;
    };
    if !facts.members.contains_key(&declaration) {
        return;
    }
    if facts.references.len() != read_reference_count(cx.scoping(), symbol) {
        return;
    }
    removals.remove(declaration);
}

struct AssignmentSlot {
    index: usize,
    value: Span,
}

fn assignment_slots(
    facts: &VariableFacts<'_>,
    assignments: &[Assignment],
    order: &InitOrder,
) -> Option<(DeclarationSlot, Vec<AssignmentSlot>)> {
    let anchor = order.declaration_slot(facts.statement?)?;
    let mut slots = Vec::with_capacity(assignments.len());
    for assignment in assignments {
        let slot = order.declaration_slot(assignment.statement)?;
        if !anchor.same_list(slot) {
            return None;
        }
        slots.push(AssignmentSlot {
            index: slot.index(),
            value: assignment.value,
        });
    }
    let nested = assignments.iter().any(|outer| {
        assignments.iter().any(|inner| {
            inner.expression != outer.expression
                && outer.expression.start <= inner.expression.start
                && inner.expression.end <= outer.expression.end
        })
    });
    if nested && !crate::mutants::nested_literal_writes_trusted() {
        return None;
    }
    let initialised =
        !facts.initialised_in.is_empty() && !crate::mutants::declarator_writes_ignored();
    if initialised && slots.iter().any(|slot| slot.index <= anchor.index()) {
        return None;
    }
    Some((anchor, slots))
}

fn nearest_slot(slots: &[AssignmentSlot], use_index: usize) -> Option<&AssignmentSlot> {
    slots
        .iter()
        .filter(|slot| slot.index < use_index)
        .max_by_key(|slot| slot.index)
}

fn statement_value(
    facts: &VariableFacts<'_>,
    slots: Option<&(DeclarationSlot, Vec<AssignmentSlot>)>,
    order: &InitOrder,
    reference: Span,
) -> Result<Span, (&'static str, Option<Span>)> {
    let Some((anchor, slots)) = slots else {
        return Err((notes::REFERENCE_NOT_DOMINATED, refusal_span(facts)));
    };
    let Some(site) = order.slot_of_reference(*anchor, reference) else {
        return Err((notes::REFERENCE_NOT_DOMINATED, Some(reference)));
    };
    if !external_write_order_trusted() && slots.iter().any(|slot| slot.index == site.index()) {
        return Err((notes::WRITTEN_BESIDE_THE_READ, Some(reference)));
    }
    match nearest_slot(slots, site.index()) {
        Some(nearest) => Ok(nearest.value),
        None => Err((notes::NO_PRECEDING_ASSIGNMENT, Some(reference))),
    }
}

fn inline_assigned_references<'a>(
    view: &Program<'a>,
    (facts, symbol): (&VariableFacts<'_>, SymbolId),
    (assignments, refusal): (&[Assignment], Option<&'static str>),
    order: &InitOrder,
    cx: &mut Cx<'a, '_>,
    edits: &mut ExprEdits<'a>,
) -> usize {
    let slots = refusal
        .is_none()
        .then(|| assignment_slots(facts, assignments, order))
        .flatten();
    let mut sequenced: Option<FxHashMap<Span, Span>> = None;
    let mut rewritten = 0;
    let mut refused = None;
    for reference in facts.references {
        let found = statement_value(facts, slots.as_ref(), order, reference.span).or_else(|err| {
            let values =
                sequenced.get_or_insert_with(|| ordered_values(view, cx.scoping(), symbol));
            let value = values.get(&reference.span).copied();
            if value.is_some() {
                crate::write_only::note_sequenced_value();
            }
            value.ok_or(err)
        });
        let value = match found {
            Ok(value) => value,
            Err(err) => {
                refused.get_or_insert(err);
                continue;
            }
        };
        let Some(value) = expression_at(view, value) else {
            cx.note(notes::NO_PRECEDING_ASSIGNMENT, Some(reference.span));
            continue;
        };
        let mut value = value.clone_in(cx.allocator());
        strip_writes(&mut value, cx.allocator());
        if edits.insert(reference.span, value) {
            rewritten += 1;
        }
    }
    match (refused, refusal) {
        (Some(_), Some(reason)) => cx.note(reason, refusal_span(facts)),
        (Some((reason, span)), None) => cx.note(reason, span),
        (None, _) => {}
    }
    rewritten
}

fn literal_assignments(view: &Program<'_>, facts: &VariableFacts<'_>) -> Vec<Assignment> {
    facts
        .assignments
        .iter()
        .filter_map(|assignment| {
            let value = chained_literal(expression_at(view, assignment.value)?)?;
            Some(Assignment {
                value,
                ..*assignment
            })
        })
        .collect()
}

fn can_remove_assigned(
    view: &Program<'_>,
    facts: &VariableFacts<'_>,
    assignments: &[Assignment],
    symbol: SymbolId,
    scoping: &Scoping,
) -> bool {
    assignments
        .iter()
        .all(|assignment| is_sole_assignment(view, assignment.statement, symbol, scoping))
        && facts.references.len() == read_reference_count(scoping, symbol)
}

fn refusal_span(facts: &VariableFacts<'_>) -> Option<Span> {
    facts
        .declaration
        .or_else(|| facts.references.first().map(|reference| reference.span))
}

fn assigned_refusal(
    facts: &VariableFacts<'_>,
    assignments: &[Assignment],
    symbol: SymbolId,
    cx: &Cx<'_, '_>,
) -> Option<&'static str> {
    if !shares_the_declaration_list(facts, assignments) {
        return Some(notes::ASSIGNMENT_NOT_BESIDE_THE_DECLARATION);
    }
    if assignments.len() != write_reference_count(cx.scoping(), symbol)
        || initialised_beside_an_assignment(facts, assignments)
        || (facts.conditional && call_checks_apply(cx.opts))
    {
        return Some(notes::NON_LITERAL_WRITE);
    }
    None
}

fn initialised_beside_an_assignment(facts: &VariableFacts<'_>, assignments: &[Assignment]) -> bool {
    if crate::mutants::declarator_writes_ignored() {
        return false;
    }
    if facts.rebound {
        return true;
    }
    match facts.initialised_in {
        [] => false,
        [statement] => {
            facts.statement != Some(*statement)
                || assignments
                    .iter()
                    .any(|assignment| assignment.statement == *statement)
        }
        _ => true,
    }
}

fn shares_the_declaration_list(facts: &VariableFacts<'_>, assignments: &[Assignment]) -> bool {
    let Some(list) = facts.declaration.and_then(|span| facts.members.get(&span)) else {
        return false;
    };
    assignments
        .iter()
        .all(|assignment| facts.members.get(&assignment.statement) == Some(list))
}

fn inline_assigned_variable<'a>(
    view: &Program<'a>,
    facts: &VariableFacts<'_>,
    symbol: SymbolId,
    order: &InitOrder,
    cx: &mut Cx<'a, '_>,
    edits: &mut ExprEdits<'a>,
    removals: &mut StmtEdits<'a>,
) {
    let assignments = literal_assignments(view, facts);
    if assignments.is_empty() {
        return;
    }
    let refusal = assigned_refusal(facts, &assignments, symbol, cx);
    let rewritten = inline_assigned_references(
        view,
        (facts, symbol),
        (&assignments, refusal),
        order,
        cx,
        edits,
    );
    if refusal.is_some()
        || rewritten != facts.references.len()
        || !can_remove_assigned(view, facts, &assignments, symbol, cx.scoping())
    {
        return;
    }
    for assignment in &assignments {
        removals.remove(assignment.statement);
    }
    if let Some(declaration) = facts.declaration {
        removals.remove(declaration);
    }
}

fn external_symbols(pending: &Pending) -> Vec<SymbolId> {
    let mut seen = FxHashSet::default();
    pending
        .externals
        .iter()
        .copied()
        .filter(|symbol| seen.insert(*symbol))
        .collect()
}

fn plan_externals<'a>(
    program: &Program<'a>,
    symbols: &[SymbolId],
    order: &InitOrder,
    cx: &mut Cx<'a, '_>,
    edits: &mut ExprEdits<'a>,
    removals: &mut StmtEdits<'a>,
) {
    let variables = variable_facts(program, cx.scoping(), symbols);
    let globals = global_bindings(program, cx, symbols);
    let writes = if symbols.is_empty() || crate::mutants::annex_b_writes_trusted() {
        AnnexBWrites::default()
    } else {
        AnnexBWrites::of(program, cx.scoping())
    };
    let mut kept = StmtEdits::new();
    for (index, symbol) in symbols.iter().copied().enumerate() {
        let Some(facts) = variables.facts(index) else {
            continue;
        };
        #[cfg(debug_assertions)]
        crate::fused::check::record("variable-facts", facts_digest(&facts, symbol));
        if let Some(span) = globals.reached(symbol) {
            cx.note(notes::GLOBAL_BINDING_REACHED, Some(span));
            continue;
        }
        if writes.may_write(symbol) {
            cx.note(notes::BLOCK_FUNCTION_NAME, facts.declaration);
            continue;
        }
        let removals = if globals.observed(symbol) {
            &mut kept
        } else {
            &mut *removals
        };
        if is_constant_binding(cx.scoping(), symbol) {
            inline_constant_variable(program, &facts, symbol, order, cx, edits, removals);
        } else {
            inline_assigned_variable(program, &facts, symbol, order, cx, edits, removals);
        }
    }
    let sources = constant_sources(program, &variables, symbols, cx.scoping(), &writes);
    if !sources.is_empty() {
        plan_externals(program, &sources, order, cx, edits, removals);
    }
}

fn constant_sources(
    program: &Program<'_>,
    variables: &Variables,
    symbols: &[SymbolId],
    scoping: &Scoping,
    writes: &AnnexBWrites,
) -> Vec<SymbolId> {
    let mut found = Vec::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if is_constant_binding(scoping, *symbol) {
            continue;
        }
        let Some(facts) = variables.facts(index) else {
            continue;
        };
        for assignment in facts.assignments {
            let Some(value) = expression_at(program, assignment.value) else {
                continue;
            };
            if !is_literal_argument(value) && literal_shape(value) {
                collect_symbols(value, scoping, &mut found);
            }
        }
    }
    let mut seen = FxHashSet::default();
    found
        .into_iter()
        .filter(|symbol| {
            is_constant_binding_in(scoping, writes, *symbol) && !symbols.contains(symbol)
        })
        .filter(|symbol| seen.insert(*symbol))
        .collect()
}

fn literal_shape(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::Identifier(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) && literal_shape(&unary.argument)
        }
        Expression::BinaryExpression(binary) => {
            literal_shape(&binary.left) && literal_shape(&binary.right)
        }
        _ => false,
    }
}

fn inline_external_variables<'a>(
    program: &mut Program<'a>,
    pending: &Pending,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let symbols = external_symbols(pending);
    if symbols.is_empty() {
        return 0;
    }
    let order = scan_init_order(program, cx);
    let mut edits = ExprEdits::new();
    let mut removals = StmtEdits::new();
    plan_externals(program, &symbols, &order, cx, &mut edits, &mut removals);
    let wanted = edits.len() + removals.len();
    let applied = edits.apply(program, cx.allocator(), cx.scoping_mut())
        + removals.apply(program, cx.allocator(), cx.scoping_mut());
    if applied != wanted {
        cx.note(
            notes::planned_and_applied_edits(crate::decoded::PASS, wanted, applied),
            None,
        );
    }
    applied
}

fn remove_bare_references<'a>(
    program: &mut Program<'a>,
    pending: &Pending,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let mut removals = StmtEdits::new();
    for span in pending.bare.iter().copied() {
        removals.remove(span);
    }
    removals.apply(program, cx.allocator(), cx.scoping_mut())
}

pub fn normalize_calls<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) -> usize {
    let first = scan_pending(program, cx.scoping());
    let pending = pending_after(0, first, program, cx.scoping());
    note_scoped_ternaries(program, &pending, cx);
    let pending = pending_after(0, pending, program, cx.scoping());
    let externals = inline_external_variables(program, &pending, cx);
    let pending = pending_after(externals, pending, program, cx.scoping());
    let bare = remove_bare_references(program, &pending, cx);
    externals + bare
}
