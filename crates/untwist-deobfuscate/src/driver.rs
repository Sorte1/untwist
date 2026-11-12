use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use untwist_core::ast_util::respan::synthetic_spans_exhausted;
use untwist_core::pass::{
    edits, replay_allowed, replayed_report, run_once, run_to_fixpoint, run_to_fixpoint_noting_last,
    IterationTiming, Note, NoteKind, PassCtor, PassGroup, PassOptions, RunReport, StopReason,
};
use untwist_core::semantic::{assumed_hazard, rebuild_scoping, scope_hazard, Semantics};
use untwist_eval::rotation::RotationMemo;

use crate::decoded::SharedHeap;
use crate::init_order::clear_kept_order;
use crate::note_set::NoteSet;
use crate::notes;
use crate::stamp::{find_stamped, has_stamped_array, names_stamped_array, RefusedByDecode};
use crate::{
    array_rotator, calls, control_flow_object, control_flow_switch, dead_assignment, dead_code,
    debug_protection, decoded, decoder, evaluate_globals, fold_chains, merge_object_assignments,
    object_props, pure_calls, removal, self_defending, string_array, unstamp, var_transformation,
    wrappers,
};

fn group_of<'a>(ctors: &[PassCtor<'a>], opts: PassOptions) -> PassGroup<'a> {
    PassGroup::from_ctors(ctors, opts)
}

pub const STRING_ARRAY_MAX_ITERATIONS: u32 = 32;

pub const DEOBFUSCATE_MAX_ROUNDS: u32 = 64;

fn max_rounds() -> u32 {
    crate::mutants::deobfuscate_round_limit(DEOBFUSCATE_MAX_ROUNDS)
}

pub const NO_STRING_ARRAY_NOTE: &str =
    "find-string-array: no string array found; deobfuscation skipped";

pub const DEOBFUSCATE_ORDER: &[&str] = &[
    "find-string-array",
    "find-decoders",
    "inline-object-props",
    "inline-decoder-wrappers",
    "inline-decoded-strings",
    "remove-decoder-machinery",
];

const FIND_DECODERS: usize = 1;

pub const OBJECT_PROPS: usize = 2;

const WRAPPERS: usize = 3;

pub const INLINE_DECODED: usize = 4;

const REMOVE_MACHINERY: usize = 5;

pub fn deobfuscate_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        string_array::boxed,
        decoder::boxed,
        object_props::boxed,
        wrappers::boxed,
        decoded::boxed,
        removal::boxed,
    ]
}

pub const CLEANUP_FIXPOINT_ORDER: &[&str] = &[
    "merge-strings",
    "dead-code",
    "control-flow-object",
    "control-flow-switch",
];

pub fn cleanup_fixpoint_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        untwist_unminify::merge_strings::boxed,
        dead_code::boxed,
        control_flow_object::boxed,
        control_flow_switch::boxed,
    ]
}

pub const PROTECTIONS_ORDER: &[&str] = &["self-defending", "debug-protection"];

pub fn protections_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![self_defending::boxed, debug_protection::boxed]
}

pub fn run_protections<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    let mut group = group_of(&protections_ctors(), opts.clone());
    run_to_fixpoint(alloc, program, sem, &mut group)
}

pub const CLEANUP_ORDER: &[&str] = &[
    "merge-object-assignments",
    "inline-object-props",
    "evaluate-globals",
    "fold-pure-calls",
    "var-transformation",
    "dead-assignment-elimination",
    "fold-literal-chains",
];

pub fn cleanup_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        merge_object_assignments::boxed,
        object_props::boxed_superset,
        evaluate_globals::boxed,
        pure_calls::boxed,
        var_transformation::boxed,
        dead_assignment::boxed,
        fold_chains::boxed,
    ]
}

fn cleanup_group<'a>(opts: &PassOptions) -> PassGroup<'a> {
    let opts = PassOptions {
        shares_init_order: opts.extended,
        ..opts.clone()
    };
    group_of(&cleanup_ctors(), opts)
}

pub fn run_cleanup<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    let mut group = cleanup_group(opts);
    clear_kept_order();
    let ran = run_to_fixpoint(alloc, program, sem, &mut group);
    clear_kept_order();
    ran
}

#[derive(Default)]
pub struct CleanupSlot(Option<(Settled, bool)>);

pub fn run_cleanup_settling<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    settling: (&PassOptions, &mut CleanupSlot),
) -> (Semantics, RunReport) {
    clear_kept_order();
    let settled = cleanup_settling(alloc, program, sem, settling);
    clear_kept_order();
    settled
}

fn cleanup_settling<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    (opts, slot): (&PassOptions, &mut CleanupSlot),
) -> (Semantics, RunReport) {
    let mut group = cleanup_group(opts);
    let marked = opts.code_text.is_marked();
    let kept = slot
        .0
        .take()
        .filter(|(kept, seen)| kept.edits == edits() && *seen == marked && replay_allowed());
    if let Some((kept, _)) = kept {
        return (sem, replayed_report(&group, kept.notes));
    }
    if !opts.extended {
        return run_to_fixpoint(alloc, program, sem, &mut group);
    }
    let (sem, report, last) = run_to_fixpoint_noting_last(alloc, program, sem, &mut group);
    if report.total_changes > 0 {
        slot.0 = last.map(|notes| {
            let settled = Settled {
                edits: edits(),
                notes,
            };
            (settled, opts.code_text.is_marked())
        });
    }
    (sem, report)
}

struct Settled {
    edits: u64,
    notes: Vec<Note>,
}

struct Aggregate {
    settled: Option<Settled>,
    rounds: u32,
    total_changes: u32,
    prepare_changes: u32,
    per_pass: Vec<u32>,
    per_pass_elapsed: Vec<Duration>,
    fixpoint_per_pass: Vec<u32>,
    fixpoint_per_pass_elapsed: Vec<Duration>,
    iteration_elapsed: Vec<IterationTiming>,
    notes: NoteSet,
    stage_stop: Option<StopReason>,
    capped: bool,
    undone: bool,
    deferred: bool,
    stamped: bool,
    skipped_noted: bool,
}

impl Aggregate {
    fn new() -> Self {
        Self {
            settled: None,
            rounds: 0,
            total_changes: 0,
            prepare_changes: 0,
            per_pass: vec![0; DEOBFUSCATE_ORDER.len()],
            per_pass_elapsed: vec![Duration::ZERO; DEOBFUSCATE_ORDER.len()],
            fixpoint_per_pass: vec![0; CLEANUP_FIXPOINT_ORDER.len()],
            fixpoint_per_pass_elapsed: vec![Duration::ZERO; CLEANUP_FIXPOINT_ORDER.len()],
            iteration_elapsed: Vec::new(),
            notes: NoteSet::default(),
            stage_stop: None,
            capped: false,
            undone: false,
            deferred: false,
            stamped: false,
            skipped_noted: false,
        }
    }

    fn push_note(&mut self, note: Note) {
        self.notes.push(note, self.rounds);
    }

    fn absorb_timings(&mut self, timings: &[IterationTiming]) {
        self.iteration_elapsed.reserve_exact(timings.len());
        self.iteration_elapsed.extend_from_slice(timings);
    }

    fn begin_round(&mut self) {
        self.stage_stop = None;
        self.deferred = false;
    }

    fn absorb(&mut self, index: usize, report: RunReport) -> u32 {
        self.total_changes += report.total_changes;
        self.absorb_timings(&report.iteration_elapsed);
        self.per_pass[index] += report.total_changes;
        for (_, elapsed) in &report.per_pass_elapsed {
            self.per_pass_elapsed[index] += *elapsed;
        }
        for note in report.notes {
            self.push_note(note);
        }
        if self.stage_stop.is_none() && report.stop != StopReason::Converged {
            self.stage_stop = Some(report.stop);
        }
        report.total_changes
    }

    fn absorb_cleanup(&mut self, report: RunReport) -> u32 {
        self.total_changes += report.total_changes;
        self.absorb_timings(&report.iteration_elapsed);
        for ((name, count), (_, elapsed)) in report.per_pass.iter().zip(&report.per_pass_elapsed) {
            let Some(index) = CLEANUP_FIXPOINT_ORDER
                .iter()
                .position(|entry| entry == name)
            else {
                continue;
            };
            if let Some(slot) = self.fixpoint_per_pass.get_mut(index) {
                *slot += count;
            }
            if let Some(slot) = self.fixpoint_per_pass_elapsed.get_mut(index) {
                *slot += *elapsed;
            }
        }
        for note in report.notes {
            self.push_note(note);
        }
        if self.stage_stop.is_none() && report.stop != StopReason::Converged {
            self.stage_stop = Some(report.stop);
        }
        report.total_changes
    }

    fn note_skipped(&mut self) {
        if self.skipped_noted || self.stamped {
            return;
        }
        self.skipped_noted = true;
        self.push_note(Note {
            pass: "deobfuscate",
            kind: NoteKind::Progress,
            message: NO_STRING_ARRAY_NOTE.to_string(),
            span: None,
        });
    }

    fn stop(&self) -> StopReason {
        if self.capped {
            return StopReason::IterationCap;
        }
        self.stage_stop.unwrap_or(StopReason::Converged)
    }

    fn finish(mut self) -> RunReport {
        let stop = self.stop();
        if self.capped {
            self.push_note(Note {
                pass: "deobfuscate",
                kind: NoteKind::Refusal,
                message: match self.undone {
                    true => notes::round_cap_undone(max_rounds()),
                    false => notes::round_cap(max_rounds()),
                },
                span: None,
            });
        }
        if self.deferred {
            self.push_note(Note {
                pass: "deobfuscate",
                kind: NoteKind::Refusal,
                message: notes::DECODE_DEFERRED_AT_STOP.to_string(),
                span: None,
            });
        }
        let settled = self.rounds.saturating_sub(1);
        let notes = std::mem::take(&mut self.notes).into_current(settled);
        self.into_report(stop, notes)
    }

    fn into_report(self, stop: StopReason, notes: Vec<Note>) -> RunReport {
        RunReport {
            iterations: self.rounds,
            total_changes: self.total_changes,
            prepare_changes: self.prepare_changes,
            per_pass: DEOBFUSCATE_ORDER
                .iter()
                .copied()
                .zip(self.per_pass)
                .collect(),
            per_pass_elapsed: DEOBFUSCATE_ORDER
                .iter()
                .copied()
                .zip(self.per_pass_elapsed)
                .collect(),
            fixpoint_per_pass: CLEANUP_FIXPOINT_ORDER
                .iter()
                .copied()
                .zip(self.fixpoint_per_pass)
                .collect(),
            fixpoint_per_pass_elapsed: CLEANUP_FIXPOINT_ORDER
                .iter()
                .copied()
                .zip(self.fixpoint_per_pass_elapsed)
                .collect(),
            iteration_elapsed: self.iteration_elapsed,
            stop,
            notes,
            heap_charged: 0,
        }
    }
}

fn note_rotators(program: &Program<'_>, names: Vec<&str>, aggregate: &mut Aggregate) {
    for (message, kind, span) in array_rotator::rotator_notes(program, &names) {
        aggregate.push_note(Note {
            pass: "find-array-rotator",
            kind,
            message,
            span,
        });
    }
}

fn stamped_names<'p>(program: &'p Program<'p>, scoping: &Scoping) -> Vec<&'p str> {
    if !names_stamped_array(scoping) {
        return Vec::new();
    }
    find_stamped(program)
        .arrays
        .iter()
        .map(|entry| entry.name)
        .collect()
}

fn note_stamped(program: &Program<'_>, scoping: &Scoping, aggregate: &mut Aggregate) -> bool {
    let names = stamped_names(program, scoping);
    let stamped = !names.is_empty();
    #[cfg(debug_assertions)]
    crate::fused::check::record(
        "stamped",
        crate::fused::check::digest_of(&(stamped, &names)),
    );
    if stamped {
        aggregate.stamped = true;
    }
    if aggregate.stamped {
        note_rotators(program, names, aggregate);
    }
    stamped
}

struct Shared {
    memo: Rc<RefCell<RotationMemo>>,
    refused: RefusedByDecode,
    heap: SharedHeap,
    observed: Rc<removal::ObservedNames>,
    defer_disabled: bool,
}

fn stage_group<'a>(
    index: usize,
    ctor: &PassCtor<'a>,
    opts: &PassOptions,
    shared: &Shared,
) -> PassGroup<'a> {
    let opts = stage_options(index, opts);
    match index {
        FIND_DECODERS => {
            PassGroup::new(vec![decoder::boxed_with(Rc::clone(&shared.refused))], opts)
        }
        INLINE_DECODED => PassGroup::new(
            vec![decoded::boxed_sharing(
                Rc::clone(&shared.memo),
                Rc::clone(&shared.refused),
                Rc::clone(&shared.heap),
            )],
            opts,
        ),
        REMOVE_MACHINERY => PassGroup::new(
            vec![removal::boxed_sharing(Rc::clone(&shared.observed))],
            opts,
        ),
        _ => PassGroup::from_ctors(std::slice::from_ref(ctor), opts),
    }
}

fn stage_options(index: usize, opts: &PassOptions) -> PassOptions {
    let shares_init_order = matches!(index, 0 | FIND_DECODERS | OBJECT_PROPS | WRAPPERS);
    if index > 0 {
        return PassOptions {
            shares_init_order,
            ..opts.clone()
        };
    }
    PassOptions {
        max_iterations: Some(STRING_ARRAY_MAX_ITERATIONS),
        shares_init_order,
        ..opts.clone()
    }
}

struct Stage<'s, 'a> {
    ctors: &'s [PassCtor<'a>],
    opts: &'s PassOptions,
    shared: &'s Shared,
}

impl<'a> Stage<'_, 'a> {
    fn run(
        &self,
        alloc: &'a Allocator,
        program: &mut Program<'a>,
        sem: Semantics,
        index: usize,
        aggregate: &mut Aggregate,
    ) -> (Semantics, u32) {
        let (next, report, deferred) = self.run_unabsorbed(alloc, program, sem, index, aggregate);
        aggregate.deferred |= deferred;
        let changes = aggregate.absorb(index, report);
        (next, changes)
    }

    fn run_unabsorbed(
        &self,
        alloc: &'a Allocator,
        program: &mut Program<'a>,
        sem: Semantics,
        index: usize,
        aggregate: &Aggregate,
    ) -> (Semantics, RunReport, bool) {
        self.shared.observed.at(aggregate.total_changes);
        let mut group = stage_group(index, &self.ctors[index], self.opts, self.shared);
        let (next, report) = run_once(alloc, program, sem, &mut group);
        if index == WRAPPERS {
            clear_kept_order();
        }
        (next, report, group.deferred_pending())
    }

    fn wrappers_then_decode(
        &self,
        alloc: &'a Allocator,
        program: &mut Program<'a>,
        sem: Semantics,
        aggregate: &mut Aggregate,
    ) -> (Semantics, u32) {
        let (sem, inlined) = self.run(alloc, program, sem, WRAPPERS, aggregate);
        let (mut sem, decoded) = self.run(alloc, program, sem, INLINE_DECODED, aggregate);
        let mut changes = inlined + decoded;
        let cascades = string_array::legacy::lifted_any() && replay_allowed();
        let mut settled = inlined == 0 || decoded > 0 || !cascades;
        while !settled && aggregate.rounds + 1 < max_rounds() {
            let (next, more) = self.next_round_wrappers(alloc, program, sem, aggregate);
            sem = next;
            let Some(inlined) = more else {
                break;
            };
            let (next, decoded) = self.run(alloc, program, sem, INLINE_DECODED, aggregate);
            sem = next;
            changes += inlined + decoded;
            settled = decoded > 0;
        }
        (sem, changes)
    }

    fn next_round_wrappers(
        &self,
        alloc: &'a Allocator,
        program: &mut Program<'a>,
        sem: Semantics,
        aggregate: &mut Aggregate,
    ) -> (Semantics, Option<u32>) {
        let flags = calls::call_flags();
        let disabled = self.opts.defer_disabled.replace(self.shared.defer_disabled);
        let (next, report, deferred) =
            self.run_unabsorbed(alloc, program, sem, WRAPPERS, aggregate);
        if report.total_changes == 0 {
            calls::restore_call_flags(flags);
            self.opts.defer_disabled.set(disabled);
            return (next, None);
        }
        aggregate.rounds += 1;
        aggregate.begin_round();
        aggregate.deferred |= deferred;
        let inlined = aggregate.absorb(WRAPPERS, report);
        (next, Some(inlined))
    }
}

#[allow(clippy::too_many_arguments)]
fn run_round<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
    finder: &mut PassGroup<'a>,
    shared: &Shared,
    aggregate: &mut Aggregate,
) -> (Semantics, u32) {
    let ctors = deobfuscate_ctors();
    let mut sem = sem;
    let mut changes = 0u32;
    aggregate.begin_round();

    let (next, report) = run_to_fixpoint(alloc, program, sem, finder);
    sem = next;
    changes += aggregate.absorb(0, report);

    let stamped = note_stamped(program, &sem.scoping, aggregate);
    if stamped || aggregate.stamped {
        let stage = Stage {
            ctors: &ctors,
            opts,
            shared,
        };
        for index in [FIND_DECODERS, OBJECT_PROPS] {
            let (next, stage_changes) = stage.run(alloc, program, sem, index, aggregate);
            sem = next;
            changes += stage_changes;
        }
        let (next, stage_changes) = stage.wrappers_then_decode(alloc, program, sem, aggregate);
        sem = next;
        changes += stage_changes;
        let (next, stage_changes) = stage.run(alloc, program, sem, REMOVE_MACHINERY, aggregate);
        sem = next;
        changes += stage_changes;
    } else {
        aggregate.note_skipped();
        if aggregate.rounds > 0 || changes > 0 {
            let mut group = stage_group(OBJECT_PROPS, &ctors[OBJECT_PROPS], opts, shared);
            let (next, report) = run_once(alloc, program, sem, &mut group);
            sem = next;
            changes += aggregate.absorb(OBJECT_PROPS, report);
        }
        clear_kept_order();
    }

    let (next, report) = cleanup_fixpoint((alloc, program, sem), opts, aggregate, changes);
    sem = next;
    changes += aggregate.absorb_cleanup(report);

    (sem, changes)
}

fn cleanup_fixpoint<'a>(
    (alloc, program, sem): (&'a Allocator, &mut Program<'a>, Semantics),
    opts: &PassOptions,
    aggregate: &mut Aggregate,
    round_changes: u32,
) -> (Semantics, RunReport) {
    let mut group = group_of(&cleanup_fixpoint_ctors(), opts.clone());
    let settled = aggregate.settled.take();
    if let Some(kept) = settled.filter(|kept| kept.edits == edits() && replay_allowed()) {
        return (sem, replayed_report(&group, kept.notes));
    }
    let (sem, report, last) = run_to_fixpoint_noting_last(alloc, program, sem, &mut group);
    if round_changes + report.total_changes > 0 {
        aggregate.settled = last.map(|notes| Settled {
            edits: edits(),
            notes,
        });
    }
    (sem, report)
}

#[derive(Default)]
pub struct PreparedSlot(Option<Settled>);

impl PreparedSlot {
    pub fn settled(notes: Option<Vec<Note>>) -> Self {
        Self(notes.map(|notes| Settled {
            edits: edits(),
            notes,
        }))
    }
}

fn replay_prepared(prepared: PreparedSlot, aggregate: &mut Aggregate) -> bool {
    let Some(kept) = prepared
        .0
        .filter(|kept| kept.edits == edits() && replay_allowed())
    else {
        return false;
    };
    aggregate.absorb_timings(&[IterationTiming::traversed(Duration::ZERO)]);
    for note in kept.notes {
        aggregate.push_note(note);
    }
    true
}

fn prepare<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (sem, prepared): (Semantics, PreparedSlot),
    opts: &PassOptions,
    aggregate: &mut Aggregate,
) -> Semantics {
    if replay_prepared(prepared, aggregate) {
        return sem;
    }
    let mut group = group_of(&untwist_unminify::prepare_ctors(), opts.clone());
    let (sem, report) = run_to_fixpoint(alloc, program, sem, &mut group);
    aggregate.prepare_changes = report.total_changes;
    aggregate.absorb_timings(&report.iteration_elapsed);
    for note in report.notes {
        aggregate.push_note(note);
    }
    if report.stop != StopReason::Converged {
        aggregate.push_note(Note {
            pass: "deobfuscate",
            kind: NoteKind::Refusal,
            message: notes::preparation_stopped_early(report.stop.as_str()),
            span: None,
        });
    }
    sem
}

fn preflight(sem: &Semantics, aggregate: &mut Aggregate) {
    let hazard = scope_hazard(&sem.scoping).or_else(|| assumed_hazard(&sem.scoping));
    if let Some(hazard) = hazard {
        aggregate.push_note(Note {
            pass: "deobfuscate",
            kind: NoteKind::Refusal,
            message: notes::scope_hazard(hazard),
            span: None,
        });
    }
    if synthetic_spans_exhausted() {
        aggregate.push_note(Note {
            pass: "deobfuscate",
            kind: NoteKind::Refusal,
            message: notes::DEOBFUSCATE_SPANS_EXHAUSTED.to_string(),
            span: None,
        });
    }
}

fn finalize_unresolved_machinery<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    (opts, shared): (&PassOptions, &Shared),
    aggregate: &mut Aggregate,
) -> Semantics {
    if !aggregate.stamped || !unstamp::has_survivor(program.source_text, &sem.scoping) {
        return sem;
    }
    shared.observed.at(aggregate.total_changes);
    if shared
        .observed
        .judge(program, &sem.scoping, opts.nested_root)
        .is_some()
    {
        return sem;
    }
    let mut group = PassGroup::new(vec![unstamp::boxed()], stage_options(INLINE_DECODED, opts));
    let (next, report) = run_once(alloc, program, sem, &mut group);
    aggregate.absorb(INLINE_DECODED, report);
    next
}

fn undone_at_cap<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (sem, before): (Semantics, &Program<'_>),
    aggregate: &mut Aggregate,
) -> Semantics {
    if !aggregate.capped || aggregate.per_pass[0] == 0 {
        return sem;
    }
    *program = before.clone_in(alloc);
    aggregate.undone = true;
    rebuild_scoping(program, sem.stats)
}

fn holds_stamped_array(program: &Program<'_>, sem: &Semantics) -> bool {
    names_stamped_array(&sem.scoping) && has_stamped_array(program)
}

pub fn drop_cleared_observation(notes: &mut Vec<Note>, sweep: &RunReport) {
    let removed = sweep
        .per_pass
        .iter()
        .any(|(name, count)| *name == removal::NAME && *count > 0);
    let observed = |note: &Note| note.message == notes::MACHINERY_NAME_OBSERVED;
    if removed && !sweep.notes.iter().any(observed) {
        notes.retain(|note| !observed(note));
    }
}

pub fn run_machinery_sweep<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    if !holds_stamped_array(program, &sem) {
        return (sem, RunReport::default());
    }
    let mut group = group_of(&[removal::boxed], opts.clone());
    run_once(alloc, program, sem, &mut group)
}

pub fn run_final_machinery<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    if !holds_stamped_array(program, &sem) {
        return (sem, RunReport::default());
    }
    let mut group = group_of(&[removal::boxed, unstamp::boxed], opts.clone());
    run_to_fixpoint(alloc, program, sem, &mut group)
}

fn run_rounds<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
    shared: &Shared,
    aggregate: &mut Aggregate,
) -> Semantics {
    let mut finder = PassGroup::new(
        vec![string_array::boxed_with(Rc::clone(&shared.refused))],
        stage_options(0, opts),
    );
    let mut sem = sem;
    let disabled = shared.defer_disabled;
    loop {
        let (next, changes) = run_round(alloc, program, sem, opts, &mut finder, shared, aggregate);
        sem = next;
        aggregate.rounds += 1;
        let retry = changes == 0 && aggregate.deferred && !opts.defer_disabled.get();
        if changes == 0 && !retry {
            opts.defer_disabled.set(disabled);
            return sem;
        }
        if untwist_core::budget::check().is_err() {
            aggregate.stage_stop = Some(StopReason::Budget);
            opts.defer_disabled.set(disabled);
            return sem;
        }
        if aggregate.rounds >= max_rounds() {
            aggregate.capped = true;
            opts.defer_disabled.set(disabled);
            return sem;
        }
        opts.defer_disabled.set(disabled || retry);
    }
}

pub fn run_deobfuscate<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    run_deobfuscate_sharing(alloc, program, sem, opts, &memo)
}

pub fn run_deobfuscate_prepared<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (sem, prepared): (Semantics, PreparedSlot),
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    deobfuscate_with(alloc, program, (sem, prepared), opts, &memo)
}

pub fn run_deobfuscate_sharing<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
    memo: &Rc<RefCell<RotationMemo>>,
) -> (Semantics, RunReport) {
    deobfuscate_with(alloc, program, (sem, PreparedSlot::default()), opts, memo)
}

fn deobfuscate_with<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    prepared: (Semantics, PreparedSlot),
    opts: &PassOptions,
    memo: &Rc<RefCell<RotationMemo>>,
) -> (Semantics, RunReport) {
    let _deferral = crate::legacy_reach::open_stage(!opts.unsafe_transforms && !opts.nested_root);
    let mut aggregate = Aggregate::new();
    let sem = prepare(alloc, program, prepared, opts, &mut aggregate);
    preflight(&sem, &mut aggregate);

    let shared = Shared {
        memo: Rc::clone(memo),
        refused: Rc::default(),
        heap: SharedHeap::default(),
        observed: Rc::default(),
        defer_disabled: opts.defer_disabled.get(),
    };
    clear_kept_order();
    let snapshot = Allocator::default();
    let before = program.clone_in(&snapshot);
    let sem = run_rounds(alloc, program, sem, opts, &shared, &mut aggregate);
    let sem = undone_at_cap(alloc, program, (sem, &before), &mut aggregate);
    drop(snapshot);
    clear_kept_order();
    let sem = finalize_unresolved_machinery(alloc, program, sem, (opts, &shared), &mut aggregate);

    let mut report = aggregate.finish();
    report.heap_charged = shared.heap.borrow().peak();
    (sem, report)
}
