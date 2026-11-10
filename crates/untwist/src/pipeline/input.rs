use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::ast_util::respan::reset_synthetic_spans;
use untwist_core::ast_util::statements::statement_spans;
use untwist_core::depth::{limits_for, scan};
use untwist_core::parse::{has_error, parse_program, parse_with, Parsed, SourceType};
use untwist_core::pass::{InputRoom, Note, NoteKind, PassOptions};
use untwist_core::print::pretty_codegen;
use untwist_core::reflow::{print_placing, Check, GuardRecord, Printed};
use untwist_core::semantic::{
    assume_observers, build_scoping, forget_failures, Observers, Semantics,
};
use untwist_core::type_space::{declares_an_interface, holds_type_space};
use untwist_deobfuscate::layers::{verdict, LayerForm, Refusal};
use untwist_eval::stack::room;
use untwist_rename::unique::TurnGuard;
use untwist_rename::RenameMode;
use untwist_unpack::holds_a_bundle_with;

use super::boundary::Boundary;
use super::layer::{Counters, Layering, Peeled, RoomSeed};
use super::notes::{consolidate_reach, dedupe_spanned, guard_notes};
use super::shell::{scope_shell_body, strip_shell};
use super::snapshots::Snapshots;
use super::stages::StageCx;
use super::{summarize_diagnostics, Pipeline, Reservation, PANICKED, TYPESCRIPT_IN_JAVASCRIPT};
use crate::options::Progress;
use crate::report::StageReport;

#[derive(Default)]
pub(super) struct InputResult {
    pub(super) code: String,
    pub(super) stages: Vec<StageReport>,
    pub(super) notes: Vec<Note>,
    pub(super) parse_diagnostics: Vec<String>,
    pub(super) parse_errors: u32,
    pub(super) panicked: bool,
    pub(super) refused: bool,
    pub(super) heap_peak: u64,
    pub(super) memory_peak: u64,
    pub(super) budget_hit: Option<untwist_core::budget::Hit>,
    pub(super) guards: GuardRecord,
    pub(super) unbundled: bool,
    pub(super) snapshots: Snapshots,
    pub(super) decoded_strings: Vec<super::DecodedSite>,
    pub(super) statement_spans: Option<Vec<(u32, u32)>>,
    #[cfg(feature = "refusal-spans")]
    pub(super) refusal_sites: Vec<super::RefusalSite>,
}

impl InputResult {
    #[cfg(feature = "parallel")]
    pub(super) fn source_only(source: &str) -> Self {
        let mut result = InputResult {
            code: source.to_string(),
            ..InputResult::default()
        };
        result.note_panicked();
        result
    }

    fn note_budget(&mut self, peak: u64, hit: Option<untwist_core::budget::Hit>) {
        self.memory_peak = peak;
        self.budget_hit = hit;
        if let Some(hit) = hit {
            self.notes.push(Note {
                pass: "pipeline",
                kind: NoteKind::Refusal,
                message: super::notes::budget_note(hit).to_string(),
                span: None,
            });
        }
    }

    pub(super) fn note_panicked(&mut self) {
        if self.panicked {
            return;
        }
        self.panicked = true;
        self.notes.push(Note {
            pass: "pipeline",
            kind: NoteKind::Refusal,
            message: PANICKED.to_string(),
            span: None,
        });
    }
}

#[cfg(target_arch = "wasm32")]
struct WasmInputNesting;

#[cfg(target_arch = "wasm32")]
impl WasmInputNesting {
    fn enter(room: Option<&InputRoom>) -> Self {
        let nesting = room.map_or(untwist_core::depth::WASM_NESTING, InputRoom::nesting);
        untwist_core::depth::set_wasm_input_nesting(nesting);
        WasmInputNesting
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WasmInputNesting {
    fn drop(&mut self) {
        untwist_core::depth::set_wasm_input_nesting(untwist_core::depth::WASM_NESTING);
    }
}

fn settle(
    outcome: Result<String, Box<dyn std::any::Any + Send>>,
    source: &str,
    result: &mut InputResult,
) {
    match outcome {
        Ok(code) => result.code = code,
        Err(_) => {
            result.note_panicked();
            result.unbundled = false;
            result.code = source.to_string();
        }
    }
}

impl Pipeline {
    pub(super) fn run_input(
        &self,
        index: usize,
        raw: &str,
        reservation: Reservation<'_>,
    ) -> InputResult {
        let announced = !self.options.isolation.refused.contains(&index);
        if announced {
            self.options.progress(Progress::InputBegins(index));
        }
        let budget = untwist_core::budget::enter(reservation.limits);
        reset_synthetic_spans();
        untwist_deobfuscate::reset_run_state();
        let _ = crate::stats::RunStats::taken();
        untwist_deobfuscate::init_order::reset_capacity_hint();
        forget_failures();
        let _observers = assume_observers(self.observers());
        let _flow_rules = untwist_deobfuscate::value_flow::assume_disabled_flow_rules(
            &self.options.disabled_passes,
        );
        let _turn = (self.options.rename == RenameMode::Unique)
            .then(|| TurnGuard::new(reservation.state, index));
        let stats_sink = reservation.stats;
        let (source, admitted) = self.admit_guarded(index, raw, reservation.stack);
        let processed = admitted.map(|(stack, room)| {
            #[cfg(target_arch = "wasm32")]
            let _nesting = WasmInputNesting::enter(room.as_ref());
            self.run_verified(&source, reservation, (room, stack.bytes()))
        });
        let (peak, hit) = (budget.peak(), untwist_core::budget::hit());
        drop(budget);
        let mut result = processed.unwrap_or_else(|note| InputResult::refused(&source, note));
        result.note_budget(peak, hit);
        let taken = crate::stats::RunStats::taken();
        let mut sink = stats_sink
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *sink = sink.merged(taken);
        drop(sink);
        if announced {
            self.options.progress(Progress::InputFinished(index));
        }
        result
    }

    fn run_verified(
        &self,
        source: &str,
        reservation: Reservation<'_>,
        (room, reserved): (Option<InputRoom>, u64),
    ) -> InputResult {
        let rerunnable = untwist_deobfuscate::legacy_reach::Rerunnable::enter();
        let first = self.run_admitted(source, reservation, (room.clone(), reserved));
        drop(rerunnable);
        if !untwist_deobfuscate::legacy_reach::unverified() {
            return first;
        }
        drop(first);
        reset_synthetic_spans();
        untwist_deobfuscate::reset_run_state();
        let _ = crate::stats::RunStats::taken();
        untwist_deobfuscate::init_order::reset_capacity_hint();
        forget_failures();
        let _forbidden = untwist_deobfuscate::legacy_reach::Forbidden::enter();
        self.run_admitted(source, reservation, (room, reserved))
    }

    fn run_admitted(
        &self,
        source: &str,
        reservation: Reservation<'_>,
        (room, reserved): (Option<InputRoom>, u64),
    ) -> InputResult {
        let mut result = InputResult {
            snapshots: Snapshots::new(self.options.stage_snapshots),
            ..InputResult::default()
        };
        let _boundary = Boundary::enter();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let processed = self.process(source, (reservation, room, reserved), &mut result)?;
            let (code, guards) = self.reflow(processed, &mut result.unbundled);
            result.notes.extend(guard_notes(guards.tally()));
            result.guards = guards;
            Some(code)
        }));
        let outcome = match outcome {
            Ok(None) => return InputResult::refused(source, TYPESCRIPT_IN_JAVASCRIPT),
            Ok(Some(code)) => Ok(code),
            Err(panic) => Err(panic),
        };
        settle(outcome, source, &mut result);
        result.notes = consolidate_reach(dedupe_spanned(std::mem::take(&mut result.notes)));
        #[cfg(feature = "refusal-spans")]
        {
            let sites = std::mem::take(&mut result.refusal_sites);
            result.refusal_sites = super::refusal_spans::kept(&result.notes, sites);
        }
        result
    }

    fn reflow(&self, printed: Printed, unbundled: &mut bool) -> (String, GuardRecord) {
        let lending =
            self.options.unpack && (printed.is_placed() || premise_admits(printed.text()));
        let options = self.unpack_options();
        let mut check = |program: &Program<'_>| holds_a_bundle_with(program, options);
        let (code, record, held) = printed.finish(lending.then_some(&mut check as Check<'_>));
        if let Some(held) = held {
            *unbundled = !held;
        }
        (code, record)
    }

    fn printed<'a>(&self, alloc: &'a Allocator, program: &mut Program<'a>, clean: bool) -> Printed {
        if !clean {
            return Printed::unplaced(pretty_codegen(program));
        }
        let options = self.unpack_options();
        let mut check = |program: &Program<'_>| holds_a_bundle_with(program, options);
        let lend = self.options.unpack.then_some(&mut check as Check<'_>);
        let printed = print_placing(alloc, program, lend);
        match premise_admits(printed.text()) {
            true => printed,
            false => printed.unplace(program),
        }
    }

    fn process(
        &self,
        source: &str,
        (reservation, room, reserved): (Reservation<'_>, Option<InputRoom>, u64),
        result: &mut InputResult,
    ) -> Option<Printed> {
        let (fuel, heap) = self.layer_budget(source.len());
        let mut layering = Layering::new(source.len(), reserved, fuel, heap);
        let mut current: Cow<'_, str> = Cow::Borrowed(source);
        let mut room = room;
        let mut form = None;
        let mut scoped = false;
        let mut fallback: Option<Fallback<'_>> = None;
        loop {
            let marks = Marks::of(result, &layering, room.as_ref());
            let layer = (reservation, room.take(), form, scoped);
            match self.process_layer(&current, layer, &mut layering, &mut fallback, result) {
                Step::Done(code) => return code,
                Step::Peel(peeled) => {
                    room = Some(peeled.room.room());
                    let text = std::mem::replace(&mut current, Cow::Owned(peeled.text));
                    fallback = Some(Fallback {
                        text,
                        marks,
                        form,
                        scoped,
                    });
                    form = Some(peeled.form);
                    scoped = peeled.scoped;
                }
                Step::Kept(reason) => {
                    let back = fallback.take()?;
                    back.marks.restore(result, &mut layering);
                    layering.keep(&current, reason);
                    room = back.marks.room.map(RoomSeed::room);
                    form = back.form;
                    scoped = back.scoped;
                    current = back.text;
                }
            }
        }
    }

    #[cfg(feature = "refusal-spans")]
    fn records_refusal_spans(&self) -> bool {
        self.options.refusal_spans || self.options.hooks().refusal_spans
    }

    fn observers(&self) -> Observers {
        match (self.options.extended(), self.options.unsafe_transforms) {
            (false, _) => Observers::Every,
            (true, true) => Observers::None,
            (true, false) => Observers::Open,
        }
    }

    fn process_layer(
        &self,
        text: &str,
        (reservation, room, form, scoped): (
            Reservation<'_>,
            Option<InputRoom>,
            Option<LayerForm>,
            bool,
        ),
        layering: &mut Layering,
        fallback: &mut Option<Fallback<'_>>,
        result: &mut InputResult,
    ) -> Step {
        let first = layering.peeled() == 0;
        if layering.start_run() {
            fresh_tree_state(room.as_ref());
        }
        let alloc = Allocator::default();
        let mut parsed = match first {
            true => parse_program(&alloc, text),
            false => parse_with(&alloc, text, SourceType::script()),
        };
        let errors = match parse_outcome(text, &parsed, first, result) {
            Ok(errors) => errors,
            Err(step) => return step,
        };
        let unsafe_transforms = self.options.unsafe_transforms;
        if let Some(Err(reason)) =
            form.map(|form| verdict(form, &parsed.program, unsafe_transforms))
        {
            return Step::Kept(reason);
        }
        *fallback = None;
        result.snapshots.enter_layer(layering.peeled());
        let decoding = self.options.decoded_strings.then(|| {
            (
                layering.peeled(),
                untwist_deobfuscate::decoded_strings::Recorder::start(),
            )
        });
        #[cfg(feature = "refusal-spans")]
        let recorder = self.records_refusal_spans().then(|| {
            (
                layering.peeled(),
                untwist_deobfuscate::refusal_spans::Recorder::start(),
            )
        });
        let staged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let tree = (&alloc, &mut parsed.program);
            let checked = first && errors == 0;
            self.stage_layer(tree, (reservation, room, checked, scoped), layering, result)
        }));
        #[cfg(feature = "refusal-spans")]
        if let Some((layer, recorder)) = recorder {
            let sites = super::refusal_spans::located(text, layer, recorder.finish());
            result.refusal_sites.extend(sites);
        }
        if let Some((layer, recorder)) = decoding {
            let found = super::decoded::sited(text.len(), layer, recorder.finish());
            result.decoded_strings.extend(found);
        }
        match staged {
            Ok(Staged::TypeScript) => Step::Done(None),
            Ok(Staged::Peeled(peeled)) => Step::Peel(peeled),
            Ok(Staged::Done) => {
                if first && self.options.statement_spans {
                    result.statement_spans = Some(statement_spans(&parsed.program));
                }
                Step::Done(Some(self.printed(&alloc, &mut parsed.program, errors == 0)))
            }
            Err(_) => {
                result.note_panicked();
                Step::Done(Some(Printed::unplaced(pretty_codegen(&parsed.program))))
            }
        }
    }

    fn stage_layer<'a>(
        &self,
        (alloc, program): (&'a Allocator, &mut Program<'a>),
        (reservation, room, checked, scoped): (Reservation<'_>, Option<InputRoom>, bool, bool),
        layering: &mut Layering,
        result: &mut InputResult,
    ) -> Staged {
        let stripped = strip_shell((alloc, program), &mut result.stages, &mut result.notes);
        let nested_root = stripped || scoped;
        let pass_opts = PassOptions {
            room,
            nested_root,
            ..self.pass_options()
        };
        let mut site = StageCx {
            alloc,
            program,
            pass_opts: &pass_opts,
            stages: &mut result.stages,
            notes: &mut result.notes,
            heap_peak: &mut result.heap_peak,
            snapshots: &mut result.snapshots,
        };
        let sem = build_scoping(site.program);
        if checked && declares_an_interface(&sem.scoping) {
            return Staged::TypeScript;
        }
        let staged = self.run_layer_stages(&mut site, sem, reservation, layering);
        if nested_root && matches!(staged, Staged::Done) {
            scope_shell_body((alloc, program), scoped, &mut result.notes);
        }
        staged
    }

    fn run_layer_stages(
        &self,
        site: &mut StageCx<'_, '_>,
        sem: Semantics,
        reservation: Reservation<'_>,
        layering: &mut Layering,
    ) -> Staged {
        if let Some(peeled) = self.peel(site, &sem, layering) {
            return Staged::Peeled(peeled);
        }
        let sem = self.front_stages(site, sem);
        if let Some(peeled) = self.peel(site, &sem, layering) {
            return Staged::Peeled(peeled);
        }
        self.shaping_stages(site, sem, reservation);
        Staged::Done
    }
}

fn parse_outcome(
    text: &str,
    parsed: &Parsed<'_>,
    first: bool,
    result: &mut InputResult,
) -> Result<usize, Step> {
    if !first {
        return match has_error(&parsed.diagnostics) {
            true => Err(Step::Kept(Refusal::Unparsable)),
            false => Ok(0),
        };
    }
    let (diagnostics, errors) = summarize_diagnostics(text, &parsed.diagnostics);
    if errors > 0 && holds_type_space(&parsed.program) {
        return Err(Step::Done(None));
    }
    result.parse_diagnostics = diagnostics;
    result.parse_errors = u32::try_from(errors).unwrap_or(u32::MAX);
    Ok(errors)
}

enum Staged {
    TypeScript,
    Peeled(Peeled),
    Done,
}

enum Step {
    Done(Option<Printed>),
    Peel(Peeled),
    Kept(Refusal),
}

struct Marks {
    stages: usize,
    notes: usize,
    snapshots: usize,
    heap_peak: u64,
    counters: Counters,
    room: Option<RoomSeed>,
}

impl Marks {
    fn of(result: &InputResult, layering: &Layering, room: Option<&InputRoom>) -> Self {
        Self {
            stages: result.stages.len(),
            notes: result.notes.len(),
            snapshots: result.snapshots.mark(),
            heap_peak: result.heap_peak,
            counters: layering.mark(),
            room: room.map(RoomSeed::of),
        }
    }

    fn restore(&self, result: &mut InputResult, layering: &mut Layering) {
        result.stages.truncate(self.stages);
        result.notes.truncate(self.notes);
        result.snapshots.restore(self.snapshots);
        result.heap_peak = self.heap_peak;
        layering.restore(self.counters);
    }
}

struct Fallback<'s> {
    text: Cow<'s, str>,
    marks: Marks,
    form: Option<LayerForm>,
    scoped: bool,
}

fn fresh_tree_state(room: Option<&InputRoom>) {
    reset_synthetic_spans();
    untwist_deobfuscate::init_order::reset_capacity_hint();
    forget_failures();
    #[cfg(target_arch = "wasm32")]
    if let Some(room) = room {
        untwist_core::depth::set_wasm_input_nesting(room.nesting());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = room;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FinalTextGuard {
    PremiseP1,
    Scan,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FinalTextRefusal {
    Unparsable,
    TooDeep,
}

pub(super) fn guard_admits(guard: FinalTextGuard, text: &str) -> bool {
    match guard {
        FinalTextGuard::PremiseP1 => premise_admits(text),
        FinalTextGuard::Scan => scan_admits(text),
    }
}

fn scan_admits(text: &str) -> bool {
    scan(text, &limits_for(room(), text.len())).is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn premise_admits(_text: &str) -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
fn premise_admits(text: &str) -> bool {
    scan_admits(text)
}

pub(super) fn final_trees<'b, T: AsRef<str>>(
    alloc: &'b Allocator,
    codes: &[T],
    guard: FinalTextGuard,
) -> Result<Vec<Program<'b>>, FinalTextRefusal> {
    let mut programs = Vec::with_capacity(codes.len());
    for code in codes {
        if !guard_admits(guard, code.as_ref()) {
            return Err(FinalTextRefusal::TooDeep);
        }
        let parsed = parse_program(alloc, alloc.alloc_str(code.as_ref()));
        if has_error(&parsed.diagnostics) {
            return Err(FinalTextRefusal::Unparsable);
        }
        programs.push(parsed.program);
    }
    Ok(programs)
}

#[cfg(test)]
mod tests {
    use super::super::{Options, Pipeline, Reservation};
    use super::{settle, InputResult};
    use untwist_core::reflow::reflow;
    use untwist_eval::stack_reserve::StackReserve;
    use untwist_rename::unique::shared_state;

    const ANONYMOUS: &str = "var f = function(){ return [1, 2, 3]; };";

    #[test]
    fn the_arena_scope_hands_back_text_its_reflow_has_not_touched() {
        let pipeline = Pipeline::new(Options {
            unpack: false,
            ..Options::default()
        });
        let state = shared_state();
        let stack = StackReserve::default();
        let stats = std::sync::Mutex::default();
        let reservation = Reservation {
            state: &state,
            slot: None,
            stack: &stack,
            stats: &stats,
            limits: untwist_core::budget::Limits::default(),
        };
        let printed = pipeline
            .process(
                ANONYMOUS,
                (reservation, None, 0),
                &mut InputResult::default(),
            )
            .unwrap();
        let finished = pipeline.run_input(0, ANONYMOUS, reservation);
        assert!(printed.is_placed());
        assert_ne!(printed.text(), finished.code);
        assert_eq!(reflow(printed.text().to_string()), finished.code);
        assert_eq!(printed.finish(None).0, finished.code);
    }

    #[test]
    fn a_task_that_panicked_before_printing_passes_its_source_through() {
        let mut result = InputResult::default();
        settle(Err(Box::new("boom")), "var a = !0;", &mut result);
        assert_eq!(result.code, "var a = !0;");
        assert!(result.panicked);
        assert_eq!(result.notes.len(), 1);
        assert_eq!(result.notes[0].message, super::PANICKED);
    }

    #[test]
    fn a_task_that_printed_keeps_its_code() {
        let mut result = InputResult::default();
        settle(Ok("var a = true;".to_string()), "var a = !0;", &mut result);
        assert_eq!(result.code, "var a = true;");
        assert!(!result.panicked);
        assert!(result.notes.is_empty());
    }
}
