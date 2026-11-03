use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;

use oxc_allocator::GetAllocator;
use oxc_ast::ast::{IdentifierReference, Program, ThisExpression};
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::ExprEdits;
use untwist_core::pass::{CodeTextSeen, Cx, Interest, Pass};
use untwist_eval::bail::{Bail, EvalResult};
use untwist_eval::budget::STEP_BUDGET;
use untwist_eval::decode::{is_decoded_string, Decoded as DecodedValue, InterpEvaluator};
use untwist_eval::object::PROTOTYPE_LINKS;
use untwist_eval::rotation::RotationMemo;
use untwist_eval::value::JsString;

use crate::bridge::{
    build_requests_named, decoded_expression, primitive_value, DecodePlan, DecodeRequest,
};
use crate::calls::normalize_calls;
use crate::notes;
use crate::proof::{
    build_escape_context, encode_names, names_machinery, scan_pending, EscapeContext, PendingSite,
    PreScan, ProtoWrite,
};
use crate::stamp::{
    is_unedited, unstamp_block_note, unstamp_blocked, unstamp_names, RefusedByDecode,
};

mod budget;
mod conflict;
mod reach;
mod root_keys;
mod values;

pub(crate) use budget::{InputHeap, SharedHeap};
use conflict::{describe_write, prototype_conflict};
use values::{decoded_strings, judge_resolved_roots, value_map_for};

const MAX_DEFER_ROUNDS: u32 = 5;

pub const PASS: &str = "inline-decoded-strings";

struct Decoded {
    substitutions: Vec<(Span, Option<DecodedValue>)>,
    errors: Vec<Span>,
    notes: Vec<(String, Option<Span>)>,
    partial: Vec<(usize, usize)>,
    all_strings: bool,
    unstamp: Vec<Vec<(SymbolId, String)>>,
    refused: Vec<Span>,
    budget_spent: bool,
}

impl Default for Decoded {
    fn default() -> Self {
        Self {
            substitutions: Vec::new(),
            errors: Vec::new(),
            notes: Vec::new(),
            partial: Vec::new(),
            all_strings: true,
            unstamp: Vec::new(),
            refused: Vec::new(),
            budget_spent: false,
        }
    }
}

enum Outcome {
    Nothing,
    Refused,
    Decoded(Decoded),
}

fn drained(evaluator: &mut InterpEvaluator) -> Vec<(String, Option<Span>)> {
    std::mem::take(&mut evaluator.notes)
        .into_iter()
        .map(|note| (note, None))
        .collect()
}

fn all_root_names_set() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        untwist_eval::builtins::all_root_names()
            .iter()
            .copied()
            .collect()
    })
}

fn is_every_root(names: &FxHashSet<&'static str>) -> bool {
    !names.is_empty() && names.len() >= all_root_names_set().len()
}

const GLOBAL_OBJECT_NAMES: [&str; 4] = ["globalThis", "window", "self", "global"];

struct RootTouchPredictor<'s> {
    all_root_names: &'s FxHashSet<&'static str>,
    found: bool,
}

impl<'a> VisitJs<'a> for RootTouchPredictor<'_> {
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if self.all_root_names.contains(name) || GLOBAL_OBJECT_NAMES.contains(&name) {
            self.found = true;
        }
    }
}

fn request_could_touch_builtins(
    request: &DecodeRequest<'_>,
    all_root_names: &FxHashSet<&'static str>,
) -> bool {
    let mut predictor = RootTouchPredictor {
        all_root_names,
        found: false,
    };
    predictor.visit_function(request.program.string_array.function, ScopeFlags::empty());
    if !predictor.found {
        for decoder in &request.program.decoders {
            predictor.visit_function(decoder.function, ScopeFlags::empty());
            if predictor.found {
                break;
            }
        }
    }
    predictor.found
}

struct UnitRun<'a, 'r> {
    request: &'r DecodeRequest<'a>,
    evaluator: InterpEvaluator,
    result: EvalResult<Vec<Option<DecodedValue>>>,
}

fn record_decoded_name(
    decoded: &mut Decoded,
    request: &DecodeRequest<'_>,
    evaluator: &mut InterpEvaluator,
) {
    decoded.notes.extend(drained(evaluator));
    decoded.notes.push((notes::DECODED_NAME.to_string(), None));
    decoded.unstamp.push(
        request
            .guard
            .symbols
            .iter()
            .map(|(symbol, name)| (*symbol, (*name).to_string()))
            .collect(),
    );
    decoded
        .refused
        .extend(request.guard.statements.iter().copied());
}

fn record_builtin_patched(
    decoded: &mut Decoded,
    evaluator: &mut InterpEvaluator,
    span: Option<Span>,
    replaced: &str,
) {
    decoded.notes.extend(drained(evaluator));
    #[cfg(feature = "test-hooks")]
    if let Some(span) = span {
        crate::refusal_spans::record(|| format!("BUILTIN_PATCHED: {replaced}"), span);
    }
    decoded
        .notes
        .push((format!("{}: {replaced}", notes::BUILTIN_PATCHED), span));
}

fn apply_decoded_values<'a>(
    decoded: &mut Decoded,
    (request, heap): (&DecodeRequest<'a>, &mut InputHeap),
    evaluator: &mut InterpEvaluator,
    values: Vec<Option<DecodedValue>>,
) -> bool {
    let returned = values.len();
    let mut evaluated = 0;
    let mut inlined = true;
    for (call, value) in request.calls.iter().zip(values) {
        let Some(value) = value else {
            continue;
        };
        evaluated += 1;
        let is_string = is_decoded_string(&value);
        decoded.all_strings &= is_string;
        if !heap.inline(&value) {
            inlined = false;
            continue;
        }
        if !is_string {
            decoded.errors.push(call.span);
        }
        decoded
            .substitutions
            .push((call.span, primitive_value(value)));
    }
    decoded.budget_spent |= evaluated != returned || !inlined;
    decoded.notes.extend(drained(evaluator));
    if returned != request.calls.len() {
        decoded.partial.push((returned, request.calls.len()));
    }
    inlined && returned == request.calls.len() && evaluated == returned
}

fn spelled_machinery(runs: &[UnitRun<'_, '_>]) -> Vec<bool> {
    let decoded: Vec<&[Option<DecodedValue>]> = runs
        .iter()
        .filter_map(|run| run.result.as_deref().ok())
        .collect();
    runs.iter()
        .map(|run| {
            let names = encode_names(&run.request.guard.names);
            decoded
                .iter()
                .any(|values| names_machinery(values.iter().flatten(), &names))
        })
        .collect()
}

type Judged<'j, 'p> = (
    &'j FxHashMap<Span, JsString>,
    &'j mut InputHeap,
    &'j [ProtoWrite<'p>],
);

fn judge_unit(
    decoded: &mut Decoded,
    run: UnitRun<'_, '_>,
    spelled: bool,
    resolved_roots: &FxHashSet<&'static str>,
    resolved_span: Option<Span>,
    (value_map, heap, late): Judged<'_, '_>,
) -> bool {
    let UnitRun {
        request,
        mut evaluator,
        result,
    } = run;
    let values = match result {
        Ok(values) => values,
        Err(Bail::Budget) => {
            decoded.notes.extend(drained(&mut evaluator));
            decoded.budget_spent = true;
            return false;
        }
        Err(bail) => {
            decoded.notes.extend(drained(&mut evaluator));
            decoded.notes.push((notes::decode_bailed(bail), None));
            return false;
        }
    };
    if spelled {
        record_decoded_name(decoded, request, &mut evaluator);
        return false;
    }
    let relies_on_prototypes =
        !evaluator.proto_reads.is_empty() && resolved_roots.contains(PROTOTYPE_LINKS);
    let replaced = if relies_on_prototypes {
        Some("an object's prototype link".to_string())
    } else if let Some(name) = evaluator
        .builtin_roots
        .iter()
        .find(|name| resolved_roots.contains(*name))
    {
        Some((*name).to_string())
    } else {
        prototype_conflict(
            &evaluator.proto_reads,
            &request.guard.proto_writes,
            value_map,
        )
        .or_else(|| prototype_conflict(&evaluator.proto_reads, late, value_map))
        .map(|write| {
            #[cfg(feature = "test-hooks")]
            crate::refusal_spans::record(
                || {
                    let proof = &request.guard.proto_writes;
                    let kept = proof.iter().any(|own| std::ptr::eq(own, write));
                    let source = match (kept, write.key) {
                        (true, _) => "the patch proof",
                        (false, crate::proof::PatchKey::Whole) => {
                            "the agnostic check, standing for code from text or a saturated check"
                        }
                        (false, _) => "the agnostic check",
                    };
                    let what = describe_write(write, value_map);
                    format!("BUILTIN_PATCHED write: {what}, from {source}")
                },
                write.span,
            );
            describe_write(write, value_map)
        })
    };
    if let Some(replaced) = replaced {
        record_builtin_patched(decoded, &mut evaluator, resolved_span, &replaced);
        return false;
    }
    apply_decoded_values(decoded, (request, heap), &mut evaluator, values)
}

#[derive(Default)]
struct ScanCache {
    context: Option<EscapeContext>,
    prescan: Option<PreScan>,
    own_spans: Option<FxHashSet<Span>>,
}

fn own_call_spans(plan: &DecodePlan<'_>) -> FxHashSet<Span> {
    plan.requests
        .iter()
        .flat_map(|request| request.calls.iter().map(|call| call.span))
        .collect()
}

fn prescan_for<'c>(
    cache: &'c mut ScanCache,
    program: &Program<'_>,
    scoping: &Scoping,
    plan: &DecodePlan<'_>,
) -> (&'c EscapeContext, &'c PreScan) {
    let ScanCache {
        context,
        prescan,
        own_spans,
    } = cache;
    let context = &*context.get_or_insert_with(|| build_escape_context(program, scoping));
    let spans = own_spans.get_or_insert_with(|| own_call_spans(plan));
    let prescan = &*prescan.get_or_insert_with(|| {
        scan_pending(program, scoping, all_root_names_set(), context, spans)
    });
    (context, prescan)
}

fn should_defer(
    plan: &DecodePlan<'_>,
    program: &Program<'_>,
    scoping: &Scoping,
    defer_disabled: bool,
    defer_budget: &Cell<u32>,
    cache: &mut ScanCache,
) -> bool {
    let has_calls = plan
        .requests
        .iter()
        .any(|request| !request.calls.is_empty());
    let could_touch = has_calls
        && plan
            .requests
            .iter()
            .any(|request| request_could_touch_builtins(request, all_root_names_set()));
    if !has_calls || defer_disabled || defer_budget.get() == 0 || !could_touch {
        return false;
    }
    defer_budget.set(defer_budget.get() - 1);
    let (_, scan) = prescan_for(cache, program, scoping, plan);
    let has_forcing_pending = scan.pending.iter().any(|site| match site {
        PendingSite::Sink { forcing, .. } | PendingSite::GlobalKey { forcing, .. } => *forcing,
        PendingSite::ProxyKey {
            strict_names,
            forcing,
            ..
        } => *forcing && !strict_names.is_empty(),
    });
    has_forcing_pending && !is_every_root(&scan.names)
}

const CODE_RUNNERS: [&str; 5] = ["eval", "Function", "setTimeout", "setInterval", "<unknown>"];

fn reached_code_runner(runs: &[UnitRun<'_, '_>]) -> bool {
    runs.iter().any(|run| {
        matches!(&run.result, Err(Bail::HostileGlobal(name)) if CODE_RUNNERS.contains(&name.0.as_str()))
    })
}

fn run_decode_units<'a, 'r>(
    plan: &'r DecodePlan<'a>,
    fuel: u64,
    memo: &mut RotationMemo,
    defer_all: bool,
    heap: &mut InputHeap,
) -> (Vec<UnitRun<'a, 'r>>, bool) {
    let mut runs = Vec::new();
    let mut deferred = false;
    for request in &plan.requests {
        if request.calls.is_empty() {
            continue;
        }
        if defer_all {
            deferred = true;
            continue;
        }
        let mut evaluator = heap.evaluator(fuel);
        let result = if heap.is_spent() {
            Err(Bail::Budget)
        } else {
            let decoded = evaluator.decode_partial_with_memo(
                &request.program,
                &request.calls,
                memo,
                &request.guard.escaped_decoders,
            );
            untwist_eval::stats::record(&evaluator.budget);
            decoded
        };
        heap.settle(&result);
        runs.push(UnitRun {
            request,
            evaluator,
            result,
        });
    }
    (runs, deferred)
}

fn plan_notes(plan: &DecodePlan<'_>, decoded: &mut Decoded) {
    for (name, statement) in &plan.unattributed {
        decoded
            .notes
            .push((notes::unattributed_decoder(name), Some(*statement)));
    }
    for refusal in &plan.refusals {
        decoded.notes.push((refusal.message(), Some(refusal.span)));
    }
}

struct DecodeLimits<'d> {
    fuel: u64,
    defer_budget: &'d Cell<u32>,
    defer_disabled: bool,
    nested_root: bool,
}

fn reads_prototypes(runs: &[UnitRun<'_, '_>]) -> bool {
    runs.iter().any(|run| !run.evaluator.proto_reads.is_empty())
}

fn decode_program(
    (program, unedited): (&Program<'_>, bool),
    scoping: &Scoping,
    limits: DecodeLimits<'_>,
    memo: &mut RotationMemo,
    heap: &mut InputHeap,
    published: &CodeTextSeen,
) -> (Outcome, bool) {
    let plan = build_requests_named(program, scoping, unedited);
    if plan.refused {
        return (Outcome::Refused, false);
    }
    let mut decoded = Decoded::default();
    plan_notes(&plan, &mut decoded);

    let mut cache = ScanCache::default();
    let defer_all = should_defer(
        &plan,
        program,
        scoping,
        limits.defer_disabled,
        limits.defer_budget,
        &mut cache,
    );
    let (runs, deferred) = run_decode_units(&plan, limits.fuel, memo, defer_all, heap);
    if reached_code_runner(&runs) {
        published.mark();
    }
    if runs.is_empty() && decoded.notes.is_empty() {
        return (Outcome::Nothing, deferred);
    }

    let code_text = judge_runs(
        &mut decoded,
        runs,
        &plan,
        (program, scoping, limits.nested_root),
        (&mut cache, heap),
    );
    if code_text {
        published.mark();
    }
    (Outcome::Decoded(decoded), deferred)
}

fn refused_before_judging(
    run: &UnitRun<'_, '_>,
    reached: Option<&reach::Reached>,
    decoding: &[usize],
) -> Option<(&'static str, Span)> {
    let unheld = run
        .request
        .reliance
        .as_ref()
        .and_then(|reliance| reliance.unheld(decoding))
        .map(|span| (notes::CODE_BEFORE_ROTATOR, span));
    unheld.or_else(|| {
        reached
            .filter(|(units, _, _)| units.contains(&run.request.unit))
            .map(|(_, span, _)| (notes::STRING_ARRAY_GLOBAL_REACH, *span))
    })
}

type GuardedReach<'p> = (Option<reach::Reached>, Option<Vec<ProtoWrite<'p>>>);

fn guard_and_reach<'p>(
    runs: &[UnitRun<'p, '_>],
    plan: &DecodePlan<'p>,
    (program, scoping, nested_root): (&'p Program<'p>, &Scoping, bool),
    value_map: &mut FxHashMap<Span, JsString>,
) -> GuardedReach<'p> {
    let watched = (!nested_root)
        .then(|| reach::watched(program, scoping, runs))
        .flatten();
    if !reads_prototypes(runs) {
        let reached =
            watched.and_then(|watched| watched.globally_reached((program, scoping), plan, runs));
        return (reached, None);
    }
    let strings = decoded_strings(runs);
    let found = crate::calls::agnostic::guard_writes(
        program,
        scoping,
        (Some(&strings), &plan.machinery_spans()),
        watched.as_ref().map(reach::Watched::watch),
    );
    #[cfg(feature = "test-hooks")]
    crate::refusal_spans::extend(&found.trail);
    for (span, text) in found.extra {
        value_map.insert(span, JsString::from(text.as_str()));
    }
    let reached = watched
        .zip(found.reach)
        .map(|(watched, span)| watched.reached_at(span, found.bootstrap));
    (reached, Some(found.writes))
}

fn judge_runs<'p>(
    decoded: &mut Decoded,
    runs: Vec<UnitRun<'p, '_>>,
    plan: &DecodePlan<'p>,
    (program, scoping, nested_root): (&'p Program<'p>, &Scoping, bool),
    (cache, heap): (&mut ScanCache, &mut InputHeap),
) -> bool {
    let (any_builtin_roots_touched, mut value_map) =
        value_map_for(&runs, plan, program, scoping, cache);
    let (reached, late_writes) =
        guard_and_reach(&runs, plan, (program, scoping, nested_root), &mut value_map);
    let late = late_writes.as_deref();
    let (resolved_roots, resolved_span, code_text) = if any_builtin_roots_touched {
        judge_resolved_roots(cache, (program, scoping), (plan, late), &runs, &value_map)
    } else {
        (FxHashSet::default(), None, false)
    };
    let spelled = spelled_machinery(&runs);
    let (relying, independent): (Vec<_>, Vec<_>) = runs
        .into_iter()
        .zip(spelled)
        .partition(|(run, _)| run.request.reliance.is_some());
    let mut decoding = Vec::new();
    for (run, spelled) in independent.into_iter().chain(relying) {
        let unit = run.request.unit;
        let reached = reached.as_ref().filter(|_| !spelled);
        if let Some((note, span)) = refused_before_judging(&run, reached, &decoding) {
            #[cfg(feature = "test-hooks")]
            if note == notes::STRING_ARRAY_GLOBAL_REACH {
                crate::refusal_spans::record(|| "GLOBAL_REACH".to_string(), span);
            }
            decoded.notes.push((note.to_string(), Some(span)));
            if let Some((_, _, Some(reason))) =
                reached.filter(|_| note == notes::STRING_ARRAY_GLOBAL_REACH)
            {
                decoded.notes.push((
                    format!("{}: {reason}", notes::WEBPACK_BOOTSTRAP_UNPROVEN),
                    Some(span),
                ));
            }
            continue;
        }
        let judged = (&value_map, &mut *heap, late.unwrap_or_default());
        if judge_unit(
            decoded,
            run,
            spelled,
            &resolved_roots,
            resolved_span,
            judged,
        ) {
            decoding.push(unit);
        }
    }
    code_text
}

pub struct InlineDecodedStrings {
    memo: Rc<RefCell<RotationMemo>>,
    defer_budget: Cell<u32>,
    refused_by_decode: RefusedByDecode,
    heap: SharedHeap,
}

impl Default for InlineDecodedStrings {
    fn default() -> Self {
        Self {
            memo: Rc::default(),
            defer_budget: Cell::new(MAX_DEFER_ROUNDS),
            refused_by_decode: RefusedByDecode::default(),
            heap: SharedHeap::default(),
        }
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineDecodedStrings::default())
}

pub fn boxed_with<'a>(
    memo: Rc<RefCell<RotationMemo>>,
    refused_by_decode: RefusedByDecode,
) -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineDecodedStrings {
        memo,
        defer_budget: Cell::new(MAX_DEFER_ROUNDS),
        refused_by_decode,
        heap: SharedHeap::default(),
    })
}

pub(crate) fn boxed_sharing<'a>(
    memo: Rc<RefCell<RotationMemo>>,
    refused_by_decode: RefusedByDecode,
    heap: SharedHeap,
) -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineDecodedStrings {
        memo,
        defer_budget: Cell::new(MAX_DEFER_ROUNDS),
        refused_by_decode,
        heap,
    })
}

impl InlineDecodedStrings {
    fn report(decoded: &Decoded, cx: &mut Cx<'_, '_>) {
        for (note, span) in &decoded.notes {
            cx.note(note.clone(), *span);
        }
        if decoded.budget_spent {
            cx.note(notes::DECODE_BUDGET_SPENT, None);
        }
        for (got, want) in &decoded.partial {
            cx.note(notes::decode_partial(*got, *want), None);
        }
        for span in &decoded.errors {
            cx.progress(notes::decode_error(), Some(*span));
        }
        if !decoded.all_strings {
            cx.note(notes::NON_STRING_RESULTS, None);
        }
    }

    fn apply_substitutions<'p>(decoded: &Decoded, program: &mut Program<'p>, cx: &mut Cx<'p, '_>) {
        let wanted = decoded.substitutions.len();
        let mut edits = ExprEdits::new();
        let mut planned = 0usize;
        for (span, value) in &decoded.substitutions {
            let built = value
                .as_ref()
                .and_then(|value| decoded_expression(value, *span, &*cx));
            let Some(expression) = built else {
                cx.note(notes::NON_PRIMITIVE_RESULT, Some(*span));
                continue;
            };
            if edits.insert(*span, expression) {
                planned += 1;
            }
        }
        let applied = edits.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
        if planned != wanted || applied != planned {
            cx.progress(notes::decoded_call_sites(planned, applied, wanted), None);
        }
    }

    fn apply_unstamp<'p>(&self, decoded: &Decoded, program: &mut Program<'p>, cx: &mut Cx<'p, '_>) {
        self.refused_by_decode
            .borrow_mut()
            .extend(decoded.refused.iter().copied());
        let blocked = unstamp_blocked(&decoded.unstamp, cx.scoping());
        for (symbols, is_blocked) in decoded.unstamp.iter().zip(blocked) {
            if is_blocked {
                cx.note(unstamp_block_note(symbols, cx.scoping()), None);
            } else {
                unstamp_names(program, cx, symbols);
            }
        }
    }
}

impl<'a> Pass<'a> for InlineDecodedStrings {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let normalized = normalize_calls(program, cx);
        cx.changed_by(normalized);
        let unedited = is_unedited(cx);

        let limits = DecodeLimits {
            fuel: cx.opts.fuel.unwrap_or(STEP_BUDGET),
            defer_budget: &self.defer_budget,
            defer_disabled: cx.defer_disabled(),
            nested_root: cx.opts.nested_root,
        };
        let mut heap = self.heap.borrow_mut();
        heap.start(
            program.source_text.len(),
            cx.opts.heap_budget,
            cx.opts.room.as_ref(),
        );
        let (outcome, deferred) = {
            let view: &Program<'_> = program;
            decode_program(
                (view, unedited),
                cx.scoping(),
                limits,
                &mut self.memo.borrow_mut(),
                &mut heap,
                &cx.opts.code_text,
            )
        };
        drop(heap);
        if deferred {
            cx.defer();
        }
        let decoded = match outcome {
            Outcome::Nothing => return,
            Outcome::Refused => {
                cx.note(notes::MACHINERY_INCOMPLETE, None);
                return;
            }
            Outcome::Decoded(decoded) => decoded,
        };

        Self::report(&decoded, cx);
        self.apply_unstamp(&decoded, program, cx);
        Self::apply_substitutions(&decoded, program, cx);
    }
}
