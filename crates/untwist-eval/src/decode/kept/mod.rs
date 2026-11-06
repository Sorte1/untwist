mod key;
mod values;

use std::collections::VecDeque;
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rustc_hash::FxHashSet;

use super::{Call, Decoded, DecoderProgram, InterpEvaluator};
use crate::bail::EvalResult;
use crate::object::ProtoRead;

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
    values: KeptValues,
    notes: Vec<String>,
    proto_reads: Vec<ProtoRead>,
    builtin_roots: Vec<&'static str>,
}

#[derive(Default)]
pub(crate) struct KeptUnits {
    entries: VecDeque<KeptUnit>,
    hits: usize,
}

impl KeptUnits {
    pub(crate) fn hits(&self) -> usize {
        self.hits
    }

    pub(crate) fn replay(
        &mut self,
        key: &UnitKey,
        evaluator: &mut InterpEvaluator,
    ) -> Option<UnitResult> {
        let kept = self.entries.iter().find(|kept| &kept.key == key)?;
        self.hits += 1;
        evaluator.notes.extend(kept.notes.iter().cloned());
        evaluator
            .proto_reads
            .extend(kept.proto_reads.iter().cloned());
        evaluator
            .builtin_roots
            .extend(kept.builtin_roots.iter().copied());
        Some(Ok(kept.values.charged(evaluator.heap.as_ref())))
    }

    pub(crate) fn early_key(
        &self,
        start: &Start,
        (program, calls, escaped_decoders): Unit<'_, '_>,
    ) -> Option<UnitKey> {
        if self.entries.is_empty() || !keeping() {
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
        unit: Unit<'_, '_>,
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
        let Some(values) = settled(evaluator, result) else {
            return;
        };
        let (program, calls, escaped_decoders) = unit;
        let key = early.unwrap_or_else(|| {
            unit_key(&start, &machinery_digest(program), calls, escaped_decoders)
        });
        if self.entries.len() >= KEPT_UNITS {
            self.entries.pop_front();
        }
        let marks = &start.marks;
        self.entries.push_back(KeptUnit {
            key,
            values,
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
    if ledger.is_some_and(|ledger| ledger.is_over()) {
        return None;
    }
    KeptValues::of(result.as_ref().ok()?, ledger)
}

struct Marks {
    notes: usize,
    proto_reads: usize,
    builtin_roots: usize,
}

pub(crate) struct Start {
    steps_used: u64,
    steps_left: u64,
    heap: Option<(bool, u64, u64)>,
    marks: Marks,
}

impl Start {
    pub(crate) fn of(evaluator: &InterpEvaluator) -> Self {
        Self {
            steps_used: evaluator.budget.steps_used(),
            steps_left: evaluator.budget.steps_left(),
            heap: evaluator
                .heap
                .as_ref()
                .map(|ledger| (ledger.is_over(), ledger.limit(), ledger.live())),
            marks: Marks {
                notes: evaluator.notes.len(),
                proto_reads: evaluator.proto_reads.len(),
                builtin_roots: evaluator.builtin_roots.len(),
            },
        }
    }
}
