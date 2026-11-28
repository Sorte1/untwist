mod key;
mod values;

use std::collections::VecDeque;
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rustc_hash::FxHashSet;

use super::{Call, Decoded, DecoderProgram, InterpEvaluator};
use crate::bail::{Bail, EvalResult};
use crate::budget::Budget;
use crate::heap::spent_run_budget;
use crate::object::ProtoRead;
use crate::rotation::MemoCounters;

use key::{machinery_digest, unit_key};
use values::KeptValues;

const KEPT_UNITS: usize = 64;

const KEEP_FROM_STEPS: u64 = 1 << 22;

pub(crate) type UnitKey = [u8; 32];

type MachineryDigest = [u8; 32];

type UnitResult = EvalResult<Vec<Option<Decoded>>>;

#[cfg(feature = "test-hooks")]
static KEEPING_OFF: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
static ARGUMENTS_UNKEYED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
static KEEP_FROM: AtomicU64 = AtomicU64::new(KEEP_FROM_STEPS);

#[cfg(feature = "test-hooks")]
pub(crate) fn set_keep_from_steps(steps: Option<u64>) {
    KEEP_FROM.store(steps.unwrap_or(KEEP_FROM_STEPS), Ordering::SeqCst);
}

fn keep_from_steps() -> u64 {
    #[cfg(feature = "test-hooks")]
    return KEEP_FROM.load(Ordering::SeqCst);
    #[cfg(not(feature = "test-hooks"))]
    return KEEP_FROM_STEPS;
}

#[cfg(feature = "test-hooks")]
pub(crate) fn set_keeping(on: bool) {
    KEEPING_OFF.store(!on, Ordering::SeqCst);
}

#[cfg(feature = "test-hooks")]
pub(crate) fn set_arguments_unkeyed(on: bool) {
    ARGUMENTS_UNKEYED.store(on, Ordering::SeqCst);
}

fn keeping() -> bool {
    #[cfg(feature = "test-hooks")]
    return !KEEPING_OFF.load(Ordering::SeqCst) && !crate::test_hooks::recording_units();
    #[cfg(not(feature = "test-hooks"))]
    return true;
}

fn arguments_keyed() -> bool {
    #[cfg(feature = "test-hooks")]
    return !ARGUMENTS_UNKEYED.load(Ordering::SeqCst);
    #[cfg(not(feature = "test-hooks"))]
    return true;
}

struct KeptUnit {
    key: UnitKey,
    outcome: Outcome,
    notes: Vec<String>,
    proto_reads: Vec<ProtoRead>,
    builtin_roots: Vec<&'static str>,
}

enum Outcome {
    Values(KeptValues),
    Failed(Box<Failure>),
}

struct Failure {
    bail: Bail,
    from: Budget,
    to: Budget,
    stored: usize,
    searches: usize,
    hits: usize,
    heap_peak: Option<u64>,
}

impl Failure {
    fn of(
        start: &Start,
        evaluator: &InterpEvaluator,
        result: &UnitResult,
        now: MemoCounters,
    ) -> Option<Self> {
        let bail = result.as_ref().err()?;
        let ledger = evaluator.heap.as_ref();
        let settled = !matches!(bail, Bail::RunBudget(_))
            && evaluator.thrown_calls.is_empty()
            && now.stored == start.memo.stored
            && start.heap == heap_state(ledger);
        settled.then(|| Self {
            bail: bail.clone(),
            from: start.budget,
            to: evaluator.budget,
            stored: now.stored,
            searches: now.searches.saturating_sub(start.memo.searches),
            hits: now.hits.saturating_sub(start.memo.hits),
            heap_peak: ledger.map(|ledger| ledger.peak()),
        })
    }

    fn replays_from(&self, start: &Start, now: MemoCounters) -> bool {
        self.from == start.budget && self.stored == now.stored && spent_run_budget().is_ok()
    }

    fn replay(&self, evaluator: &mut InterpEvaluator) -> UnitResult {
        evaluator.budget = self.to;
        if let (Some(ledger), Some(peak)) = (evaluator.heap.as_ref(), self.heap_peak) {
            ledger.raise_peak(peak);
        }
        Err(self.bail.clone())
    }
}

pub(crate) struct Replayed {
    pub(crate) result: UnitResult,
    pub(crate) searches: usize,
    pub(crate) hits: usize,
}

#[derive(Default)]
pub(crate) struct KeptUnits {
    entries: VecDeque<KeptUnit>,
    failures: VecDeque<KeptUnit>,
    hits: usize,
    failure_hits: usize,
}

impl KeptUnits {
    pub(crate) fn hits(&self) -> usize {
        self.hits
    }

    pub(crate) fn failure_hits(&self) -> usize {
        self.failure_hits
    }

    pub(crate) fn replay(
        &mut self,
        key: &UnitKey,
        (start, now): (&Start, MemoCounters),
        evaluator: &mut InterpEvaluator,
    ) -> Option<Replayed> {
        let kept = self
            .entries
            .iter()
            .chain(self.failures.iter())
            .find(|kept| &kept.key == key)?;
        let (result, searches, hits) = match &kept.outcome {
            Outcome::Values(values) => {
                self.hits += 1;
                (Ok(values.charged(evaluator.heap.as_ref())), 0, 0)
            }
            Outcome::Failed(failure) if failure.replays_from(start, now) => {
                self.failure_hits += 1;
                (failure.replay(evaluator), failure.searches, failure.hits)
            }
            Outcome::Failed(_) => return None,
        };
        evaluator.notes.extend(kept.notes.iter().cloned());
        evaluator
            .proto_reads
            .extend(kept.proto_reads.iter().cloned());
        evaluator
            .builtin_roots
            .extend(kept.builtin_roots.iter().copied());
        Some(Replayed {
            result,
            searches,
            hits,
        })
    }

    pub(crate) fn early_key(
        &self,
        start: &Start,
        (program, calls, escaped_decoders): Unit<'_, '_>,
    ) -> Option<UnitKey> {
        if (self.entries.is_empty() && self.failures.is_empty()) || !keeping() {
            return None;
        }
        Some(unit_key(
            start,
            &machinery_digest(program),
            calls,
            escaped_decoders,
        ))
    }

    pub(crate) fn keep(
        &mut self,
        (early, start): (Option<UnitKey>, Start),
        (unit, now): (Unit<'_, '_>, MemoCounters),
        evaluator: &InterpEvaluator,
        result: &UnitResult,
    ) {
        let spent = evaluator
            .budget
            .steps_used()
            .saturating_sub(start.steps_used);
        if spent < keep_from_steps() || !keeping() {
            return;
        }
        let outcome = match settled(evaluator, result) {
            Some(values) => Outcome::Values(values),
            None => match Failure::of(&start, evaluator, result, now) {
                Some(failure) => Outcome::Failed(Box::new(failure)),
                None => return,
            },
        };
        let (program, calls, escaped_decoders) = unit;
        let key = early.unwrap_or_else(|| {
            unit_key(&start, &machinery_digest(program), calls, escaped_decoders)
        });
        self.failures.retain(|kept| kept.key != key);
        let shelf = match outcome {
            Outcome::Values(_) => &mut self.entries,
            Outcome::Failed(_) => &mut self.failures,
        };
        if shelf.len() >= KEPT_UNITS {
            shelf.pop_front();
        }
        let marks = &start.marks;
        shelf.push_back(KeptUnit {
            key,
            outcome,
            notes: tail(&evaluator.notes, marks.notes),
            proto_reads: tail(&evaluator.proto_reads, marks.proto_reads),
            builtin_roots: tail(&evaluator.builtin_roots, marks.builtin_roots),
        });
    }
}

pub(crate) type Unit<'u, 'a> = (&'u DecoderProgram<'a>, &'u [Call<'a>], &'u FxHashSet<usize>);

fn tail<T: Clone>(all: &[T], from: usize) -> Vec<T> {
    all.get(from..).unwrap_or_default().to_vec()
}

fn settled(evaluator: &InterpEvaluator, result: &UnitResult) -> Option<KeptValues> {
    let ledger = evaluator.heap.as_ref();
    if !evaluator.thrown_calls.is_empty() || ledger.is_some_and(|ledger| ledger.is_over()) {
        return None;
    }
    KeptValues::of(result.as_ref().ok()?, ledger)
}

struct Marks {
    notes: usize,
    proto_reads: usize,
    builtin_roots: usize,
}

type HeapState = Option<(bool, u64, u64)>;

fn heap_state(ledger: Option<&std::rc::Rc<crate::heap::HeapLedger>>) -> HeapState {
    ledger.map(|ledger| (ledger.is_over(), ledger.limit(), ledger.live()))
}

pub(crate) struct Start {
    steps_used: u64,
    steps_left: u64,
    heap: HeapState,
    marks: Marks,
    budget: Budget,
    memo: MemoCounters,
}

impl Start {
    pub(crate) fn of(evaluator: &InterpEvaluator, memo: MemoCounters) -> Self {
        Self {
            steps_used: evaluator.budget.steps_used(),
            steps_left: evaluator.budget.steps_left(),
            heap: heap_state(evaluator.heap.as_ref()),
            marks: Marks {
                notes: evaluator.notes.len(),
                proto_reads: evaluator.proto_reads.len(),
                builtin_roots: evaluator.builtin_roots.len(),
            },
            budget: evaluator.budget,
            memo,
        }
    }
}
