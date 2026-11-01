use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::marker::PhantomData;

use crate::bail::{AbruptKind, Bail, EvalResult};
use crate::decode::InterpEvaluator;
use crate::interp::Interp;
use crate::object::ProtoRead;
use crate::rotation::RotationMemo;
use crate::value::{Eval, FnCode, ObjKind, Value};

pub use crate::frozen::{frozen, frozen_key, FREEZE_DIR};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Decode,
    Script,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRecord {
    pub kind: UnitKind,
    pub values: Vec<String>,
    pub notes: Vec<String>,
    pub bail: Option<String>,
    pub steps_used: u64,
    pub traces: Vec<String>,
    pub proto_reads: Vec<String>,
    pub memo_inserts: usize,
    pub heap_peak: u64,
    pub heap_final: u64,
    pub attempts: Vec<String>,
    pub site_steps: Vec<u64>,
}

thread_local! {
    static RECORDS: RefCell<Option<Vec<UnitRecord>>> = const { RefCell::new(None) };
    static WIDEST: Cell<Option<usize>> = const { Cell::new(None) };
    static ATTEMPTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static SITES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

thread_local! {
    static COMPILING: Cell<u32> = const { Cell::new(0) };
}

pub fn compiling() -> bool {
    COMPILING.try_with(|depth| depth.get() > 0).unwrap_or(false)
}

pub(crate) struct Compiling(PhantomData<*const ()>);

impl Compiling {
    pub(crate) fn enter() -> Self {
        COMPILING.with(|depth| depth.set(depth.get() + 1));
        Compiling(PhantomData)
    }
}

impl Drop for Compiling {
    fn drop(&mut self) {
        COMPILING.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub(crate) fn note_attempt(line: impl FnOnce() -> String) {
    if recording() {
        ATTEMPTS.with(|attempts| attempts.borrow_mut().push(line()));
    }
}

pub(crate) fn note_site(steps: u64) {
    if recording() {
        SITES.with(|sites| sites.borrow_mut().push(steps));
    }
}

fn attempts_len() -> usize {
    ATTEMPTS.with(|attempts| attempts.borrow().len())
}

fn sites_len() -> usize {
    SITES.with(|sites| sites.borrow().len())
}

fn take_attempts(from: usize) -> Vec<String> {
    ATTEMPTS.with(|attempts| {
        let mut attempts = attempts.borrow_mut();
        let from = from.min(attempts.len());
        attempts.split_off(from)
    })
}

fn take_sites(from: usize, base: u64) -> Vec<u64> {
    SITES.with(|sites| {
        let mut sites = sites.borrow_mut();
        let from = from.min(sites.len());
        let taken = sites.split_off(from);
        taken
            .into_iter()
            .map(|steps| steps.saturating_sub(base))
            .collect()
    })
}

pub(crate) fn note_frame_width(width: impl FnOnce() -> usize) {
    WIDEST.with(|widest| {
        if let Some(seen) = widest.get() {
            widest.set(Some(seen.max(width())));
        }
    });
}

struct RestoreWidest(Option<usize>);

impl Drop for RestoreWidest {
    fn drop(&mut self) {
        WIDEST.with(|widest| widest.set(self.0));
    }
}

pub fn widest_frame(input: impl FnOnce()) -> usize {
    let restore = RestoreWidest(WIDEST.with(|widest| widest.replace(Some(0))));
    input();
    let widest = WIDEST.with(Cell::get).unwrap_or_default();
    drop(restore);
    widest
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileCensus {
    pub calls: u64,
    pub nodes: u64,
    pub statements: u64,
    pub maybe: u64,
    pub rotators: u64,
    pub checksum_scopes: u64,
    pub checksum_terms: u64,
    pub direct_arguments: u64,
    pub compiled_arguments: u64,
    pub argument_entries: u64,
    pub direct_terms: u64,
    pub constants: u64,
    pub constant_fallbacks: u64,
    pub constant_runs: Vec<ConstantRun>,
    pub templates: u64,
    pub template_constants: u64,
    pub template_batches: u64,
    pub template_fallbacks: u64,
    pub node_spans: Vec<(u32, u32)>,
    pub unit_spans: Vec<(u32, u32)>,
    pub functions: BTreeSet<(u32, u32)>,
    pub checked_units: u64,
    pub unchecked_units: u64,
    pub depth_overruns: u64,
    pub fresh_reaches: u64,
}

thread_local! {
    static CENSUS: RefCell<Option<CompileCensus>> = const { RefCell::new(None) };
}

fn with_census(update: impl FnOnce(&mut CompileCensus)) {
    CENSUS.with(|census| {
        if let Some(census) = census.borrow_mut().as_mut() {
            update(census);
        }
    });
}

pub use crate::compile::depth::{NODE_FRAME_BYTES, UNCHECKED_BYTES, UNCHECKED_LEVELS};

thread_local! {
    static FORCED_CHECKS: Cell<Option<bool>> = const { Cell::new(None) };
}

pub(crate) fn forced_reach() -> Option<crate::compile::depth::Reach> {
    FORCED_CHECKS.with(Cell::get).map(|checked| match checked {
        true => crate::compile::depth::Reach::CHECKED,
        false => crate::compile::depth::Reach::UNBOUNDED,
    })
}

struct RestoreForcedChecks(Option<bool>);

impl Drop for RestoreForcedChecks {
    fn drop(&mut self) {
        FORCED_CHECKS.with(|forced| forced.set(self.0));
    }
}

pub fn with_forced_checks<R>(checked: bool, input: impl FnOnce() -> R) -> R {
    let restore = RestoreForcedChecks(FORCED_CHECKS.with(|forced| forced.replace(Some(checked))));
    let result = input();
    drop(restore);
    result
}

pub(crate) fn note_unchecked_level(nesting: u32, ceiling: u32) {
    if nesting > ceiling {
        with_census(|census| census.depth_overruns += 1);
    }
}

pub(crate) fn note_fresh_reach() {
    with_census(|census| census.fresh_reaches += 1);
}

pub(crate) fn note_unit_run(checked: bool) {
    with_census(|census| match checked {
        true => census.checked_units += 1,
        false => census.unchecked_units += 1,
    });
}

pub(crate) fn note_compiled_call(code: FnCode<'_>) {
    with_census(|census| {
        census.calls += 1;
        census.functions.insert(code_span(code));
    });
}

fn code_span(code: FnCode<'_>) -> (u32, u32) {
    let span = match code {
        FnCode::Function(function) => function.span,
        FnCode::Arrow(arrow) => arrow.span,
    };
    (span.start, span.end)
}

pub(crate) fn note_compiled_statement(span: oxc_span::Span) {
    with_census(|census| {
        census.statements += 1;
        census.node_spans.push((span.start, span.end));
    });
}

pub fn note_compiled_node(span: oxc_span::Span) {
    with_census(|census| {
        census.nodes += 1;
        census.node_spans.push((span.start, span.end));
    });
}

pub fn note_maybe_node() {
    with_census(|census| census.maybe += 1);
}

pub(crate) fn note_folded_constant() {
    with_census(|census| census.constants += 1);
}

pub(crate) fn note_template(constants: usize) {
    with_census(|census| {
        census.templates += 1;
        census.template_constants += constants as u64;
        census.constants += constants as u64;
    });
}

pub(crate) fn note_template_run(batched: bool) {
    with_census(|census| match batched {
        true => census.template_batches += 1,
        false => census.template_fallbacks += 1,
    });
}

pub(crate) fn note_constant_fallback() {
    with_census(|census| census.constant_fallbacks += 1);
}

const CONSTANT_RUNS_KEPT: usize = 1 << 20;

pub(crate) fn note_constant_run(steps: u64, weight: u64, nesting: u32, depth: u32) {
    with_census(|census| {
        if census.constant_runs.len() < CONSTANT_RUNS_KEPT {
            census.constant_runs.push(ConstantRun {
                steps,
                weight,
                nesting,
                depth,
            });
        }
    });
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstantRun {
    pub steps: u64,
    pub weight: u64,
    pub nesting: u32,
    pub depth: u32,
}

thread_local! {
    static BAIL_NESTING: Cell<Option<Option<u32>>> = const { Cell::new(None) };
}

pub(crate) fn note_bail_nesting(nesting: u32) {
    BAIL_NESTING.with(|first| {
        if first.get() == Some(None) {
            first.set(Some(Some(nesting)));
        }
    });
}

struct RestoreBailNesting(Option<Option<u32>>);

impl Drop for RestoreBailNesting {
    fn drop(&mut self) {
        BAIL_NESTING.with(|first| first.set(self.0));
    }
}

pub fn nesting_at_first_bail(input: impl FnOnce()) -> Option<u32> {
    let restore = RestoreBailNesting(BAIL_NESTING.with(|first| first.replace(Some(None))));
    input();
    let found = BAIL_NESTING.with(Cell::get).flatten();
    drop(restore);
    found
}

#[derive(Clone, Copy)]
pub(crate) enum Entry {
    Rotator,
    ChecksumScope,
    ChecksumTerm,
    DirectArgument,
    CompiledArgument,
    CachedArgument,
    DirectTerm,
}

pub(crate) fn note_entry(entry: Entry) {
    with_census(|census| {
        let counter = match entry {
            Entry::Rotator => &mut census.rotators,
            Entry::ChecksumScope => &mut census.checksum_scopes,
            Entry::ChecksumTerm => &mut census.checksum_terms,
            Entry::DirectArgument => &mut census.direct_arguments,
            Entry::CompiledArgument => &mut census.compiled_arguments,
            Entry::CachedArgument => &mut census.argument_entries,
            Entry::DirectTerm => &mut census.direct_terms,
        };
        *counter += 1;
    });
}

pub fn note_compiled_unit(span: oxc_span::Span) {
    with_census(|census| census.unit_spans.push((span.start, span.end)));
}

struct RestoreCensus(Option<CompileCensus>);

impl Drop for RestoreCensus {
    fn drop(&mut self) {
        let outer = self.0.take();
        CENSUS.with(|census| *census.borrow_mut() = outer);
    }
}

pub fn compile_census(input: impl FnOnce()) -> CompileCensus {
    let restore =
        RestoreCensus(CENSUS.with(|census| census.borrow_mut().replace(CompileCensus::default())));
    input();
    let taken = CENSUS
        .with(|census| census.borrow_mut().take())
        .unwrap_or_default();
    drop(restore);
    taken
}

struct Restore {
    records: Option<Vec<UnitRecord>>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        RECORDS.with(|records| *records.borrow_mut() = self.records.take());
        ATTEMPTS.with(|attempts| attempts.borrow_mut().clear());
        SITES.with(|sites| sites.borrow_mut().clear());
    }
}

pub fn take_attempts_outside_units() -> Vec<String> {
    take_attempts(0)
}

pub fn record_units(input: impl FnOnce()) -> Vec<UnitRecord> {
    let records = RECORDS.with(|records| records.borrow_mut().replace(Vec::new()));
    let restore = Restore { records };
    input();
    let recorded = RECORDS.with(|records| records.borrow_mut().take());
    drop(restore);
    recorded.unwrap_or_default()
}

thread_local! {
    static CHARGED: Cell<bool> = const { Cell::new(false) };
}

struct RestoreCharged(bool);

impl Drop for RestoreCharged {
    fn drop(&mut self) {
        CHARGED.with(|charged| charged.set(self.0));
    }
}

pub fn record_units_charged(input: impl FnOnce()) -> Vec<UnitRecord> {
    let restore = RestoreCharged(CHARGED.with(|charged| charged.replace(true)));
    let records = record_units(input);
    drop(restore);
    records
}

pub(crate) fn charge_recorded_script(interp: &mut Interp<'_>) {
    if CHARGED.with(Cell::get) && recording() {
        let budget = crate::budget::heap_budget(interp.source_text.len());
        interp.set_heap(Some(crate::heap::HeapLedger::new(budget)));
    }
}

fn recording() -> bool {
    RECORDS.with(|records| records.borrow().is_some())
}

fn push(record: UnitRecord) {
    RECORDS.with(|records| {
        if let Some(records) = records.borrow_mut().as_mut() {
            records.push(record);
        }
    });
}

pub fn render_value(value: &Value<'_>) -> String {
    match value {
        Value::Undefined => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(flag) => format!("bool:{flag}"),
        Value::Number(n) => format!("number:{:016x}", n.to_bits()),
        Value::Str(text) => format!("string:{}", text.to_lossy_string()),
        Value::Object(obj) => match &obj.borrow().kind {
            ObjKind::Array(items) => format!("array:{}", items.len()),
            ObjKind::Arguments(items) => format!("arguments:{}", items.len()),
            ObjKind::MappedArguments(mapped) => format!("mapped:{}", mapped.items.len()),
            ObjKind::Function(_) | ObjKind::Bound(_) => "function".to_string(),
            ObjKind::Native(_, _) => "native".to_string(),
            ObjKind::Error(kind) => format!("error:{}", kind.name()),
            ObjKind::RegExp(_) => "regexp".to_string(),
            ObjKind::Plain(_) => "object".to_string(),
        },
    }
}

fn rendered_reads(reads: &[ProtoRead]) -> Vec<String> {
    reads.iter().map(|read| format!("{read:?}")).collect()
}

fn rendered_roots(roots: &[&'static str]) -> Vec<String> {
    roots.iter().map(|root| (*root).to_string()).collect()
}

pub(crate) struct UnitMark {
    recording: bool,
    steps: u64,
    notes: usize,
    proto_reads: usize,
    roots: usize,
    stored: usize,
    recipe_terms: usize,
    attempts: usize,
    sites: usize,
}

impl UnitMark {
    pub(crate) fn decode(evaluator: &InterpEvaluator, memo: &RotationMemo) -> Self {
        Self {
            recording: recording(),
            steps: evaluator.budget.steps_used(),
            notes: evaluator.notes.len(),
            proto_reads: evaluator.proto_reads.len(),
            roots: evaluator.builtin_roots.len(),
            stored: memo.stored(),
            recipe_terms: crate::decode::memo_recipe_stats_for_tests().recipe_terms,
            attempts: attempts_len(),
            sites: sites_len(),
        }
    }

    pub(crate) fn finish(
        self,
        evaluator: &InterpEvaluator,
        memo: &RotationMemo,
        result: &EvalResult<Vec<Option<Value<'_>>>>,
    ) {
        if !self.recording {
            return;
        }
        let (values, bail) = match result {
            Ok(values) => (values.iter().map(rendered_slot).collect(), None),
            Err(bail) => (Vec::new(), Some(bail.to_string())),
        };
        let recipe_terms = crate::decode::memo_recipe_stats_for_tests().recipe_terms;
        let heap = evaluator.heap.as_ref();
        push(UnitRecord {
            kind: UnitKind::Decode,
            values,
            notes: evaluator.notes[self.notes.min(evaluator.notes.len())..].to_vec(),
            bail,
            steps_used: evaluator.budget.steps_used().saturating_sub(self.steps),
            traces: rendered_roots(tail(&evaluator.builtin_roots, self.roots)),
            proto_reads: rendered_reads(tail(&evaluator.proto_reads, self.proto_reads)),
            memo_inserts: memo.stored().saturating_sub(self.stored)
                + recipe_terms.saturating_sub(self.recipe_terms),
            heap_peak: heap.map_or(0, |ledger| ledger.peak()),
            heap_final: heap.map_or(0, |ledger| ledger.live()),
            attempts: take_attempts(self.attempts),
            site_steps: take_sites(self.sites, self.steps),
        });
    }
}

fn tail<T>(items: &[T], from: usize) -> &[T] {
    &items[from.min(items.len())..]
}

fn rendered_slot(value: &Option<Value<'_>>) -> String {
    match value {
        Some(value) => render_value(value),
        None => "unevaluated".to_string(),
    }
}

fn rendered_result(result: &Eval<'_, Value<'_>>) -> (Vec<String>, Option<String>) {
    match result {
        Ok(value) => (vec![render_value(value)], None),
        Err(abrupt) => match abrupt.kind() {
            AbruptKind::Bail(bail) => (Vec::new(), Some(Bail::to_string(bail))),
            AbruptKind::Throw(thrown) => (vec![format!("throw {}", render_value(thrown))], None),
        },
    }
}

pub(crate) fn record_script(
    interp: &Interp<'_>,
    result: &Eval<'_, Value<'_>>,
    proto_reads: &[ProtoRead],
    roots: &[&'static str],
) {
    if !recording() {
        return;
    }
    let (values, bail) = rendered_result(result);
    let heap = interp.heap.as_ref();
    push(UnitRecord {
        kind: UnitKind::Script,
        values,
        notes: interp.notes.clone(),
        bail,
        steps_used: interp.budget.steps_used(),
        traces: rendered_roots(roots),
        proto_reads: rendered_reads(proto_reads),
        memo_inserts: 0,
        heap_peak: heap.map_or(0, |ledger| ledger.peak()),
        heap_final: heap.map_or(0, |ledger| ledger.live()),
        attempts: Vec::new(),
        site_steps: Vec::new(),
    });
}

pub fn on_budget_host<R>(body: impl FnOnce(&mut dyn crate::host::Host<'static>) -> R) -> R {
    body(&mut crate::fold::BudgetHost::default())
}
