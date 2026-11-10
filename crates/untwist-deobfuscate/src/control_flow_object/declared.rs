use std::cell::OnceCell;

use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::readonly::{
    is_readonly_object_outside, scan_symbol_uses, ObjectUses, PropRead,
};
use untwist_core::ast_util::search::expressions_at;
use untwist_core::pass::{Cx, NoteKind, RepeatedSites};
use untwist_core::semantic::is_constant_binding;

use crate::bindings::refuse_block_function_names;
use crate::init_order::{
    reads_are_initialised, refuse_read_before_declaration, scan_init_order, InitOrder,
};
use crate::notes;

use super::dead::{dead_branch_spans, dead_write_statements, predicate_branches, Branches};
use super::nested::{insert_nested, PlannedEdit, Recipe};
use super::{
    call_site_is_safe, cff_props, emit_logical_refusals, inline_site, nest_depth,
    object_declaration, CffValue, Refusal, SiteSafety, MAX_NEST, PASS,
};

pub struct DeclaredObject {
    pub statement: Span,
    pub symbol: SymbolId,
    pub object: Span,
}

struct DeclaredSearch {
    extended: bool,
    found: Vec<DeclaredObject>,
}

impl<'a> VisitJs<'a> for DeclaredSearch {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(found) = declared_object(it, self.extended) {
            self.found.push(found);
        }
        walk_js::walk_statement(self, it);
    }
}

fn declared_object(stmt: &Statement<'_>, extended: bool) -> Option<DeclaredObject> {
    let declared = object_declaration(stmt, extended)?;
    if declared.keys.is_empty() {
        return None;
    }
    Some(DeclaredObject {
        statement: stmt.span(),
        symbol: declared.symbol,
        object: declared.object,
    })
}

fn collect_candidates(program: &Program<'_>, extended: bool) -> Vec<DeclaredObject> {
    let mut search = DeclaredSearch {
        extended,
        found: Vec::new(),
    };
    search.visit_program(program);
    search.found
}

type CallArgs<'p> = (Span, &'p AVec<'p, Argument<'p>>);

type ObjectsAt<'p> = FxHashMap<Span, &'p Expression<'p>>;

fn objects_at<'p>(program: &'p Program<'p>, candidates: &[DeclaredObject]) -> ObjectsAt<'p> {
    let spans: Vec<Span> = candidates
        .iter()
        .map(|candidate| candidate.object)
        .collect();
    spans
        .iter()
        .copied()
        .zip(expressions_at(program, &spans))
        .filter_map(|(span, found)| Some((span, found?)))
        .collect()
}

struct Located<'p, 'l> {
    program: &'p Program<'p>,
    calls: &'l FxHashMap<Span, CallArgs<'p>>,
    objects: &'l ObjectsAt<'p>,
    branches: OnceCell<Branches<'p>>,
}

impl<'p> Located<'p, '_> {
    fn branches(&self, scoping: &Scoping) -> &Branches<'p> {
        self.branches
            .get_or_init(|| predicate_branches(self.program, scoping))
    }
}

struct CallsAt<'p, 'w> {
    wanted: &'w FxHashSet<Span>,
    found: FxHashMap<Span, CallArgs<'p>>,
}

impl<'p> VisitJs<'p> for CallsAt<'p, '_> {
    fn visit_expression(&mut self, it: &Expression<'p>) {
        if self.found.len() == self.wanted.len() {
            return;
        }
        if let Expression::CallExpression(call) = it {
            let callee = call.callee.span();
            if !call.optional && self.wanted.contains(&callee) && !self.found.contains_key(&callee)
            {
                let call: &'p CallExpression<'p> = self.alloc(call);
                self.found.insert(callee, (call.span, &call.arguments));
            }
        }
        walk_js::walk_expression(self, it);
    }
}

fn calls_at<'p>(
    program: &'p Program<'p>,
    candidates: &[DeclaredObject],
    scanned: &FxHashMap<SymbolId, ObjectUses>,
) -> FxHashMap<Span, CallArgs<'p>> {
    let wanted: FxHashSet<Span> = candidates
        .iter()
        .filter_map(|candidate| scanned.get(&candidate.symbol))
        .flat_map(|uses| uses.reads.iter().map(|read| read.member_span))
        .collect();
    if wanted.is_empty() {
        return FxHashMap::default();
    }
    let mut at = CallsAt {
        wanted: &wanted,
        found: FxHashMap::default(),
    };
    at.visit_program(program);
    at.found
}

struct CandidatePlan<'a, 'p> {
    statement: Span,
    edits: Vec<PlannedEdit<'a, 'p>>,
    reads: usize,
    dead_statements: Vec<Span>,
    dead_kept: Vec<Span>,
}

#[derive(Default)]
struct ReadRefusals {
    uncalled: RepeatedSites,
    logical: RepeatedSites,
}

fn substitute_function<'a, 'p>(
    calls: &FxHashMap<Span, CallArgs<'p>>,
    function: &'p Function<'p>,
    read: &PropRead,
    cx: &mut Cx<'a, '_>,
    refusals: &mut ReadRefusals,
) -> Option<PlannedEdit<'a, 'p>> {
    let Some(&(call_span, arguments)) = calls.get(&read.member_span) else {
        refusals.uncalled.record(read.member_span, "");
        return None;
    };
    match call_site_is_safe(function, arguments, cx) {
        SiteSafety::Safe => {}
        SiteSafety::Refused(Refusal::LogicalArgument) => {
            refusals.logical.record(read.member_span, &read.name);
            return None;
        }
        SiteSafety::Refused(Refusal::Shape) => return None,
    }
    Some(PlannedEdit {
        span: call_span,
        expr: inline_site(function, arguments, &*cx)?,
        recipe: Some(Recipe {
            function,
            arguments,
        }),
    })
}

fn substitute_read<'a, 'p>(
    calls: &FxHashMap<Span, CallArgs<'p>>,
    value: &CffValue<'p, 'p>,
    read: &PropRead,
    cx: &mut Cx<'a, '_>,
    refusals: &mut ReadRefusals,
) -> Option<PlannedEdit<'a, 'p>> {
    match value {
        CffValue::Text(literal) => Some(PlannedEdit {
            span: read.member_span,
            expr: Expression::StringLiteral(oxc_allocator::Box::new_in(
                (*literal).clone_in(cx.allocator()),
                &*cx,
            )),
            recipe: None,
        }),
        CffValue::Function(function) => substitute_function(calls, function, read, cx, refusals),
    }
}

fn plan_candidate<'a, 'p>(
    located: &Located<'p, '_>,
    candidate: &DeclaredObject,
    uses: &ObjectUses,
    cx: &mut Cx<'a, '_>,
) -> Option<CandidatePlan<'a, 'p>> {
    let calls = located.calls;
    let props = located
        .objects
        .get(&candidate.object)
        .and_then(|object| cff_props(object, cx.opts.extended));
    let Some(props) = props else {
        cx.note(notes::CONTROL_FLOW_SHAPE_LOST, Some(candidate.object));
        return None;
    };
    let mut by_key = FxHashMap::default();
    for (key, value) in &props {
        by_key.entry(key.as_ref()).or_insert(value);
    }
    let nested = uses.reads.iter().filter_map(|read| {
        let value = by_key.get(read.name.as_str())?;
        matches!(value, CffValue::Function(_))
            .then(|| calls.get(&read.member_span).map(|(span, _)| *span))
            .flatten()
    });
    if nested_past_the_limit(nested) {
        cx.note(notes::CONTROL_FLOW_NEST_TOO_DEEP, Some(candidate.statement));
        return None;
    }
    let mut edits = Vec::with_capacity(uses.reads.len());
    let mut refusals = ReadRefusals::default();
    for read in &uses.reads {
        let Some(value) = by_key.get(read.name.as_str()).copied() else {
            cx.note(
                notes::control_flow_missing_prop(&read.name),
                Some(read.member_span),
            );
            continue;
        };
        if let Some(edit) = substitute_read(calls, value, read, cx, &mut refusals) {
            edits.push(edit);
        }
    }
    refusals.uncalled.emit(
        NoteKind::Refusal,
        |_| notes::CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY.to_string(),
        |total, _| notes::control_flow_uncalled_function_properties(total),
        cx,
    );
    emit_logical_refusals(refusals.logical, cx);
    Some(CandidatePlan {
        statement: candidate.statement,
        edits,
        reads: uses.reads.len(),
        dead_statements: Vec::new(),
        dead_kept: Vec::new(),
    })
}

struct Substitutable<'u> {
    uses: &'u ObjectUses,
    dead: Vec<Span>,
}

fn substitutable_uses<'u>(
    located: &Located<'_, '_>,
    candidate: &DeclaredObject,
    uses: &'u ObjectUses,
    cx: &mut Cx<'_, '_>,
) -> Option<Substitutable<'u>> {
    let constant = is_constant_binding(cx.scoping(), candidate.symbol);
    if !constant && !cx.opts.extended {
        return None;
    }
    if constant && uses.writes == 0 && uses.opaque == 0 {
        return Some(Substitutable {
            uses,
            dead: Vec::new(),
        });
    }
    if !cx.opts.extended || is_redeclared(cx.scoping(), candidate.symbol) {
        return None;
    }
    let props = located
        .objects
        .get(&candidate.object)
        .and_then(|object| cff_props(object, cx.opts.extended))?;
    let branches = located.branches(cx.scoping());
    let dead = dead_branch_spans(branches, candidate.symbol, &props, &*cx);
    if dead.is_empty() {
        return None;
    }
    if !is_readonly_object_outside(uses, &dead) {
        cx.note(notes::CONTROL_FLOW_LIVE_MUTATION, Some(candidate.statement));
        return None;
    }
    Some(Substitutable { uses, dead })
}

fn is_redeclared(scoping: &Scoping, symbol: SymbolId) -> bool {
    !scoping.symbol_redeclarations(symbol).is_empty()
}

fn mutation_spans(uses: &ObjectUses) -> Vec<Span> {
    uses.write_spans
        .iter()
        .chain(&uses.opaque_spans)
        .copied()
        .collect()
}

fn plan_declared_object<'a, 'p>(
    located: &Located<'p, '_>,
    candidate: &DeclaredObject,
    uses: &ObjectUses,
    order: &InitOrder,
    cx: &mut Cx<'a, '_>,
) -> Option<CandidatePlan<'a, 'p>> {
    let Substitutable { uses, dead } = substitutable_uses(located, candidate, uses, cx)?;
    if let Err(refused) = reads_are_initialised(&uses.reads, candidate.statement, order) {
        refuse_read_before_declaration(
            refused,
            candidate.symbol,
            candidate.statement,
            notes::CONTROL_FLOW_USE_BEFORE_INIT,
            cx,
        );
        return None;
    }
    let mut plan = plan_candidate(located, candidate, uses, cx)?;
    if !dead.is_empty() {
        let branches = located.branches(cx.scoping());
        let writes = dead_write_statements(branches, &mutation_spans(uses), &dead);
        plan.dead_statements = writes.statements;
        plan.dead_kept = writes.kept;
    }
    Some(plan)
}

fn collect_edits<'a>(
    program: &Program<'_>,
    plans: Vec<CandidatePlan<'a, '_>>,
    edits: &mut ExprEdits<'a>,
    removals: &mut StmtEdits<'a>,
    cx: &mut Cx<'a, '_>,
) {
    let mut plans = plans;
    let planned: Vec<Vec<PlannedEdit<'a, '_>>> = plans
        .iter_mut()
        .map(|plan| std::mem::take(&mut plan.edits))
        .collect();
    let substituted = insert_nested(program, planned, edits, &*cx);
    for (plan, substituted) in plans.into_iter().zip(substituted) {
        if substituted == plan.reads {
            removals.remove(plan.statement);
            for span in plan.dead_statements {
                removals.remove(span);
            }
            for span in plan.dead_kept {
                cx.progress(notes::CONTROL_FLOW_DEAD_WRITE_KEPT, Some(span));
            }
            continue;
        }
        cx.note(
            notes::control_flow_object_partial(substituted, plan.reads),
            Some(plan.statement),
        );
    }
}

fn scanned_candidates(
    program: &Program<'_>,
    candidates: &[DeclaredObject],
    cx: &Cx<'_, '_>,
) -> FxHashMap<SymbolId, ObjectUses> {
    let wanted: FxHashSet<SymbolId> = candidates
        .iter()
        .filter(|candidate| cx.opts.extended || is_constant_binding(cx.scoping(), candidate.symbol))
        .map(|candidate| candidate.symbol)
        .collect();
    if wanted.is_empty() {
        return FxHashMap::default();
    }
    scan_symbol_uses(program, cx.scoping(), &wanted).uses
}

pub(super) fn inline_declared_objects<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) -> usize {
    let mut candidates = collect_candidates(program, cx.opts.extended);
    if candidates.is_empty() {
        return 0;
    }
    let order = scan_init_order(program, cx);
    let scanned = scanned_candidates(program, &candidates, cx);
    #[cfg(debug_assertions)]
    crate::fused::check::record_order_and_uses("declared-objects", &order, &scanned);
    candidates.retain(|candidate| scanned.contains_key(&candidate.symbol));
    refuse_block_function_names(
        &mut candidates,
        |found| (found.statement, [found.symbol]),
        cx,
    );
    let calls = calls_at(program, &candidates, &scanned);
    let objects = objects_at(program, &candidates);
    let located = Located {
        program,
        calls: &calls,
        objects: &objects,
        branches: OnceCell::new(),
    };
    let plans: Vec<CandidatePlan<'a, '_>> = candidates
        .iter()
        .filter_map(|candidate| {
            let uses = scanned.get(&candidate.symbol)?;
            plan_declared_object(&located, candidate, uses, &order, cx)
        })
        .collect();
    let mut edits = ExprEdits::new();
    let mut removals = StmtEdits::new();
    collect_edits(program, plans, &mut edits, &mut removals, cx);
    let wanted = edits.len() + removals.len();
    let applied = edits.apply(program, cx.allocator(), cx.scoping_mut())
        + removals.apply(program, cx.allocator(), cx.scoping_mut());
    if applied != wanted {
        cx.note(
            notes::planned_and_applied_edits(PASS, wanted, applied),
            None,
        );
    }
    applied
}
