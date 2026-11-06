use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

use rustc_hash::FxHasher;
use std::num::NonZeroU64;
use std::time::Duration;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_semantic::{Scoping, Stats};

use crate::clock::Instant;
use crate::print::{print, PrintStyle};
use crate::semantic::{refresh_scoping, Round, Semantics, Shape};

use super::settled;
use super::{Note, NoteKind, PassGroup};

pub const ITERATION_CEILING: u32 = 10_000;
pub const CYCLE_CHECK_START: u32 = 8;
const DEFERRABLE_DECODE_PASS: &str = "inline-decoded-strings";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopReason {
    #[default]
    Converged,
    IterationCap,
    Cycle,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            StopReason::Converged => "converged",
            StopReason::IterationCap => "iteration_cap",
            StopReason::Cycle => "cycle",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IterationTiming {
    traverse_ns: u64,
    rebuild_ns_plus_one: Option<NonZeroU64>,
    fingerprint_ns_plus_one: Option<NonZeroU64>,
}

impl IterationTiming {
    pub fn traversed(traverse: Duration) -> Self {
        Self {
            traverse_ns: saturating_nanos(traverse),
            ..Self::default()
        }
    }

    pub fn traverse(&self) -> Duration {
        Duration::from_nanos(self.traverse_ns)
    }

    pub fn rebuild(&self) -> Option<Duration> {
        self.rebuild_ns_plus_one.map(decode_present)
    }

    pub fn fingerprint(&self) -> Option<Duration> {
        self.fingerprint_ns_plus_one.map(decode_present)
    }

    pub fn set_rebuild(&mut self, rebuild: Duration) {
        self.rebuild_ns_plus_one = Some(encode_present(rebuild));
    }

    pub fn set_fingerprint(&mut self, fingerprint: Duration) {
        self.fingerprint_ns_plus_one = Some(encode_present(fingerprint));
    }
}

fn saturating_nanos(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX - 1)
}

fn encode_present(elapsed: Duration) -> NonZeroU64 {
    NonZeroU64::MIN.saturating_add(saturating_nanos(elapsed))
}

fn decode_present(stored: NonZeroU64) -> Duration {
    Duration::from_nanos(stored.get() - 1)
}

#[derive(Debug, Default)]
pub struct RunReport {
    pub iterations: u32,
    pub total_changes: u32,
    pub prepare_changes: u32,
    pub per_pass: Vec<(&'static str, u32)>,
    pub per_pass_elapsed: Vec<(&'static str, Duration)>,
    pub fixpoint_per_pass: Vec<(&'static str, u32)>,
    pub fixpoint_per_pass_elapsed: Vec<(&'static str, Duration)>,
    pub iteration_elapsed: Vec<IterationTiming>,
    pub stop: StopReason,
    pub notes: Vec<Note>,
    pub heap_charged: u64,
}

impl RunReport {
    pub fn converged(&self) -> bool {
        self.stop == StopReason::Converged
    }
}

fn retry_deferred_pass<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    scoping: Scoping,
    group: &mut PassGroup<'a>,
    tally: &mut Tally,
) -> (Scoping, u32) {
    let Some(index) = group.find_pass_index(DEFERRABLE_DECODE_PASS) else {
        return (scoping, 0);
    };
    let (redone, redo_changes) =
        group.run_single_pass_enter_program(alloc, program, scoping, index);
    tally.ledger.absorb(group.take_notes(), tally.iterations);
    tally.last = None;
    if redo_changes > 0 {
        group.forget_marks();
        settled::edited();
        tally.total_changes += redo_changes;
        let latest = group.per_pass_changes();
        tally.round |= round_of(&latest);
        accumulate_per_pass(&mut tally.per_pass, latest);
    }
    (redone, redo_changes)
}

struct IterationState<'x> {
    tally: &'x mut Tally,
    fingerprints: &'x mut HashSet<u64>,
    cap: u32,
}

fn run_one_iteration<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    scoping: Scoping,
    stats: Stats,
    group: &mut PassGroup<'a>,
    state: &mut IterationState<'_>,
) -> (Scoping, Stats, Option<StopReason>) {
    let is_last_allowed = state.tally.iterations + 1 >= state.cap;
    group.set_defer_disabled(is_last_allowed);
    let before = Shape::of(&scoping);
    let (traversed, timing) = timed_traverse(alloc, program, scoping, group);
    let (traversed, mut timing) = confirm_settled(alloc, program, (traversed, timing), group);
    let mut changes = state.tally.absorb(group);
    let deferred_only = changes == 0 && group.deferred_pending() && !is_last_allowed;
    let mut scoping = traversed;

    if deferred_only {
        let (redone, redo_changes) =
            retry_deferred_pass(alloc, program, scoping, group, state.tally);
        scoping = redone;
        if redo_changes > 0 {
            changes = redo_changes;
        }
    }

    if changes == 0 {
        state.tally.timings.push(timing);
        if group.round_requested() && !is_last_allowed {
            group.reset_passes();
            return (scoping, stats, None);
        }
        return (scoping, stats, Some(StopReason::Converged));
    }

    let round = Round {
        before,
        passes: state.tally.round,
    };
    let (rebuilt, elapsed) = timed_rebuild(program, scoping, stats, round);
    hold_flags(group, &rebuilt);
    group.reset_passes();
    timing.set_rebuild(elapsed);

    let stop = check_stop(
        state.tally.iterations,
        state.cap,
        program,
        state.fingerprints,
        &mut timing,
    );
    state.tally.timings.push(timing);
    (rebuilt.scoping, rebuilt.stats, stop)
}

pub fn run_to_fixpoint<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    group: &mut PassGroup<'a>,
) -> (Semantics, RunReport) {
    let (sem, report, _) = fixpoint(alloc, program, sem, group, false);
    (sem, report)
}

pub fn run_to_fixpoint_noting_last<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    group: &mut PassGroup<'a>,
) -> (Semantics, RunReport, Option<Vec<Note>>) {
    let (sem, report, last) = fixpoint(alloc, program, sem, group, true);
    let settled = last.filter(|_| report.converged()).map(|notes| {
        let mut ledger = NoteLedger::default();
        ledger.absorb(notes, 1);
        ledger.retain_current(1)
    });
    (sem, report, settled)
}

pub fn replayed_report(group: &PassGroup<'_>, notes: Vec<Note>) -> RunReport {
    RunReport {
        iterations: 1,
        total_changes: 0,
        prepare_changes: 0,
        per_pass: group.per_pass_changes(),
        per_pass_elapsed: group.per_pass_elapsed(),
        fixpoint_per_pass: Vec::new(),
        fixpoint_per_pass_elapsed: Vec::new(),
        iteration_elapsed: vec![IterationTiming::traversed(Duration::ZERO)],
        stop: StopReason::Converged,
        notes,
        heap_charged: 0,
    }
}

fn fixpoint<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    group: &mut PassGroup<'a>,
    keep_last: bool,
) -> (Semantics, RunReport, Option<Vec<Note>>) {
    hold_flags(group, &sem);
    let Semantics {
        mut scoping,
        mut stats,
        ..
    } = sem;
    group.begin_marks();
    let cap = group.opts().max_iterations.unwrap_or(ITERATION_CEILING);
    let mut tally = Tally {
        keep_last,
        ..Tally::default()
    };
    let mut fingerprints: HashSet<u64> = HashSet::new();
    let mut state = IterationState {
        tally: &mut tally,
        fingerprints: &mut fingerprints,
        cap,
    };

    let stop = loop {
        let (next_scoping, next_stats, stop) =
            run_one_iteration(alloc, program, scoping, stats, group, &mut state);
        scoping = next_scoping;
        stats = next_stats;
        if let Some(stop) = stop {
            break stop;
        }
    };

    let last = tally.last.take();
    let report = finish_report(tally, stop, cap);
    (held_semantics(group, scoping, stats), report, last)
}

fn hold_flags(group: &PassGroup<'_>, sem: &Semantics) {
    group.opts().scoping_patched.set(sem.patched);
    group.opts().scoping_canonical.set(sem.canonical);
}

fn held_semantics(group: &PassGroup<'_>, scoping: Scoping, stats: Stats) -> Semantics {
    Semantics {
        scoping,
        stats,
        patched: group.opts().scoping_patched.get(),
        canonical: group.opts().scoping_canonical.get(),
    }
}

fn timed_traverse<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    scoping: Scoping,
    group: &mut PassGroup<'a>,
) -> (Scoping, IterationTiming) {
    group.begin_iteration();
    let clock = Instant::now();
    let traversed = group.traverse(alloc, program, scoping);
    let timing = IterationTiming::traversed(clock.elapsed());
    if group.total_changes() > 0 {
        settled::edited();
    }
    (traversed, timing)
}

fn confirm_settled<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (traversed, steered): (Scoping, IterationTiming),
    group: &mut PassGroup<'a>,
) -> (Scoping, IterationTiming) {
    if group.total_changes() > 0 || !group.was_steered() {
        return (traversed, steered);
    }
    drop(group.take_notes());
    group.confirm_next();
    let (confirmed, timing) = timed_traverse(alloc, program, traversed, group);
    if group.total_changes() > 0 {
        group.fall_back();
    }
    let both = IterationTiming::traversed(steered.traverse() + timing.traverse());
    (confirmed, both)
}

fn timed_rebuild(
    program: &Program<'_>,
    traversed: Scoping,
    stats: Stats,
    round: Round,
) -> (Semantics, Duration) {
    let clock = Instant::now();
    let refreshed = refresh_scoping(program, traversed, stats, round);
    (refreshed, clock.elapsed())
}

fn check_stop(
    iterations: u32,
    cap: u32,
    program: &Program<'_>,
    fingerprints: &mut HashSet<u64>,
    timing: &mut IterationTiming,
) -> Option<StopReason> {
    if iterations >= cap {
        return Some(StopReason::IterationCap);
    }
    if iterations < CYCLE_CHECK_START {
        return None;
    }
    let clock = Instant::now();
    let fingerprint = fingerprint_program(program);
    timing.set_fingerprint(clock.elapsed());
    (!fingerprints.insert(fingerprint)).then_some(StopReason::Cycle)
}

#[derive(Default)]
struct Tally {
    iterations: u32,
    total_changes: u32,
    per_pass: Vec<(&'static str, u32)>,
    per_pass_elapsed: Vec<(&'static str, Duration)>,
    timings: Vec<IterationTiming>,
    ledger: NoteLedger,
    keep_last: bool,
    last: Option<Vec<Note>>,
    round: u64,
}

fn round_of(latest: &[(&'static str, u32)]) -> u64 {
    latest
        .iter()
        .filter(|(_, changes)| *changes > 0)
        .fold(0, |round, (name, _)| round | pass_bit(name))
}

fn pass_bit(name: &str) -> u64 {
    let mut hasher = FxHasher::default();
    name.hash(&mut hasher);
    1 << (hasher.finish() % 64)
}

impl Tally {
    fn absorb(&mut self, group: &mut PassGroup<'_>) -> u32 {
        self.iterations += 1;
        let notes = group.take_notes();
        if self.keep_last && group.total_changes() == 0 {
            self.last = Some(notes.clone());
        }
        self.ledger.absorb(notes, self.iterations);
        let changes = group.total_changes();
        self.total_changes += changes;
        let latest = group.per_pass_changes();
        self.round = round_of(&latest);
        accumulate_per_pass(&mut self.per_pass, latest);
        accumulate_elapsed(&mut self.per_pass_elapsed, group.per_pass_elapsed());
        changes
    }
}

fn accumulate_elapsed(
    acc: &mut Vec<(&'static str, Duration)>,
    latest: Vec<(&'static str, Duration)>,
) {
    if acc.is_empty() {
        *acc = latest;
        return;
    }
    for (slot, (_, elapsed)) in acc.iter_mut().zip(latest.iter()) {
        slot.1 += *elapsed;
    }
}

#[derive(Default)]
pub struct NoteLedger {
    order: Vec<Note>,
    last: HashMap<Note, u32>,
}

impl NoteLedger {
    pub fn absorb(&mut self, notes: Vec<Note>, iteration: u32) {
        for note in notes {
            if self.last.insert(note.clone(), iteration).is_none() {
                self.order.push(note);
            }
        }
    }

    pub fn survives(&self, note: &Note, settled: u32) -> bool {
        super::note_survives_retraction(note) || self.last.get(note) == Some(&settled)
    }

    pub fn retain_current(self, settled: u32) -> Vec<Note> {
        let kept: Vec<Note> = self
            .order
            .iter()
            .filter(|note| self.survives(note, settled))
            .cloned()
            .collect();
        kept
    }
}

fn finish_report(tally: Tally, stop: StopReason, cap: u32) -> RunReport {
    let Tally {
        iterations,
        total_changes,
        per_pass,
        per_pass_elapsed,
        timings,
        ledger,
        ..
    } = tally;
    let mut notes = ledger.retain_current(iterations);
    notes.extend(driver_note(stop, iterations, cap));
    RunReport {
        iterations,
        total_changes,
        prepare_changes: 0,
        per_pass,
        per_pass_elapsed,
        fixpoint_per_pass: Vec::new(),
        fixpoint_per_pass_elapsed: Vec::new(),
        iteration_elapsed: timings,
        stop,
        notes,
        heap_charged: 0,
    }
}

fn fingerprint_program(program: &Program<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    print(program, PrintStyle::Compact).hash(&mut hasher);
    hasher.finish()
}

fn driver_note(stop: StopReason, iterations: u32, cap: u32) -> Option<Note> {
    let message = match stop {
        StopReason::Converged => return None,
        StopReason::IterationCap => format!("iteration cap {cap} reached with changes pending"),
        StopReason::Cycle => format!(
            "passes oscillate: iteration {iterations} reproduced an earlier program; stopping"
        ),
    };
    Some(Note {
        pass: "driver",
        kind: NoteKind::Progress,
        message,
        span: None,
    })
}

pub fn run_once<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    group: &mut PassGroup<'a>,
) -> (Semantics, RunReport) {
    hold_flags(group, &sem);
    let Semantics {
        mut scoping,
        mut stats,
        ..
    } = sem;
    group.begin_marks();

    let before = Shape::of(&scoping);
    let (traversed, mut timing) = timed_traverse(alloc, program, scoping, group);
    let total_changes = group.total_changes();
    let per_pass = group.per_pass_changes();
    let per_pass_elapsed = group.per_pass_elapsed();

    if total_changes > 0 {
        let round = Round {
            before,
            passes: round_of(&per_pass),
        };
        let (rebuilt, elapsed) = timed_rebuild(program, traversed, stats, round);
        hold_flags(group, &rebuilt);
        scoping = rebuilt.scoping;
        stats = rebuilt.stats;
        timing.set_rebuild(elapsed);
    } else {
        scoping = traversed;
    }

    let report = RunReport {
        iterations: 1,
        total_changes,
        prepare_changes: 0,
        per_pass,
        per_pass_elapsed,
        fixpoint_per_pass: Vec::new(),
        fixpoint_per_pass_elapsed: Vec::new(),
        iteration_elapsed: vec![timing],
        stop: StopReason::Converged,
        notes: group.take_notes(),
        heap_charged: 0,
    };
    (held_semantics(group, scoping, stats), report)
}

pub fn scoping_after_traversal(
    program: &Program<'_>,
    traversed: Scoping,
    stats: Stats,
    before: Shape,
    group: &PassGroup<'_>,
) -> Semantics {
    if group.total_changes() == 0 {
        return held_semantics(group, traversed, stats);
    }
    let round = Round { before, passes: 0 };
    refresh_scoping(program, traversed, stats, round)
}

fn accumulate_per_pass(acc: &mut Vec<(&'static str, u32)>, latest: Vec<(&'static str, u32)>) {
    if acc.is_empty() {
        *acc = latest;
        return;
    }
    for (slot, (_, count)) in acc.iter_mut().zip(latest.iter()) {
        slot.1 += count;
    }
}
