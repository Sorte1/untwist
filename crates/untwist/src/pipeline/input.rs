use std::borrow::Cow;
use std::cell::Cell;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::ast_util::respan::reset_synthetic_spans;
use untwist_core::ast_util::statements::statement_spans;
use untwist_core::depth::{limits_for, scan};
use untwist_core::parse::{
    early_errors, has_error, parse_failure, parse_named_module, parse_named_script, parse_program,
    parse_with, Early, ParseFailure, Parsed, SourceType,
};
use untwist_core::pass::{InputRoom, Note, NoteKind, PassOptions, RoundCapUndo};
use untwist_core::print::pretty_codegen;
use untwist_core::reflow::{print_placing, Check, GuardRecord, PendingReaders, Printed};
use untwist_core::semantic::{
    assume_observers, assume_shared_globals, build_scoping, forget_failures, Observers, Semantics,
    SharedGlobals,
};
use untwist_core::source_map::{self, Mapped, OwnedSourceMap, Recording};
use untwist_core::type_space::{declares_an_interface, holds_type_space};
use untwist_core::visible::{respelled_as, respelling, Respelling};
use untwist_deobfuscate::layers::{verdict, LayerForm, Refusal};
use untwist_eval::stack::room;
use untwist_rename::unique::TurnGuard;
use untwist_rename::RenameMode;
use untwist_unpack::holds_a_bundle_with;

use super::boundary::Boundary;
use super::layer::{Counters, InlineSite, Layering, Peeled, RoomSeed};
use super::notes::{
    carry_reach, consolidate_reach, dedupe_spanned, guard_notes, reach_refusals, unsafe_lane_hint,
    SOURCE_READER_UNGUARDED,
};
use super::shell::{scope_shell_body, strip_shell};
use super::snapshots::Snapshots;
use super::stages::StageCx;
use super::trail::Trail;
use super::{
    summarize_diagnostics, Pipeline, Reservation, ANNEX_B_UNMODELLED, PANICKED, PARSE_UNSUPPORTED,
    TYPESCRIPT_IN_JAVASCRIPT,
};
use crate::options::Progress;
use crate::report::StageReport;

#[derive(Default)]
pub(super) struct InputResult {
    pub(super) code: String,
    pub(super) stages: Vec<StageReport>,
    pub(super) notes: Vec<Note>,
    pub(super) parse_diagnostics: Vec<String>,
    pub(super) parse_errors: u32,
    pub(super) parse_refusal: Option<&'static str>,
    pub(super) panicked: bool,
    pub(super) refused: bool,
    pub(super) heap_peak: u64,
    pub(super) memory_peak: u64,
    pub(super) budget_hit: Option<untwist_core::budget::Hit>,
    pub(super) guards: GuardRecord,
    pub(super) readers: PendingReaders,
    pub(super) unbundled: bool,
    pub(super) module_goal: bool,
    pub(super) snapshots: Snapshots,
    pub(super) decoded_strings: Vec<super::DecodedSite>,
    pub(super) statement_spans: Option<Vec<(u32, u32)>>,
    pub(super) respelling: Respelling,
    #[cfg(feature = "refusal-spans")]
    pub(super) refusal_sites: Vec<super::RefusalSite>,
    pub(super) mapped: Option<Mapped>,
    pub(super) source_map: Option<OwnedSourceMap>,
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

const MODULE_WORDS: [&str; 3] = ["import", "export", "await"];
const MODULE_MARK: &str = "\nexport {};\n";

fn kept_module_goal(code: String, module_goal: bool) -> String {
    if !module_goal {
        return code;
    }
    let spelled = MODULE_WORDS.iter().any(|word| code.contains(word));
    let module = spelled && {
        let alloc = Allocator::default();
        parse_program(&alloc, &code).program.source_type.is_module()
    };
    match module {
        true => code,
        false => code + MODULE_MARK,
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
    fn named(&self, index: usize, extension: &str) -> bool {
        self.options
            .input_names
            .get(index)
            .is_some_and(|name| name.ends_with(extension))
    }

    fn module_named(&self, index: usize) -> bool {
        self.options.module_goal || self.named(index, ".mjs")
    }

    fn script_named(&self, index: usize) -> bool {
        !self.module_named(index) && (self.options.script_goal || self.named(index, ".cjs"))
    }

    pub(super) fn run_input(
        &self,
        index: usize,
        raw: &str,
        reservation: Reservation<'_>,
    ) -> InputResult {
        let reservation = Reservation {
            module_named: self.module_named(index),
            script_named: self.script_named(index),
            ..reservation
        };
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
        result.module_goal |= reservation.module_named;
        if !result.refused {
            result.code = kept_module_goal(std::mem::take(&mut result.code), result.module_goal);
        }
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
        let forbidden = untwist_deobfuscate::legacy_reach::unverified()
            .then(untwist_deobfuscate::legacy_reach::Forbidden::enter);
        let verified = match &forbidden {
            None => first,
            Some(_) => {
                let reached = reach_refusals(&first.notes);
                drop(first);
                let mut second = self.rerun(source, reservation, (room.clone(), reserved));
                second.notes = carry_reach(std::mem::take(&mut second.notes), reached);
                second
            }
        };
        if !untwist_deobfuscate::self_defending::partial_removal_unsettled(&verified.code) {
            return verified;
        }
        let reached = reach_refusals(&verified.notes);
        drop(verified);
        let _whole = untwist_deobfuscate::self_defending::PartialRemovalsForbidden::enter();
        let mut whole = self.rerun(source, reservation, (room, reserved));
        whole.notes = carry_reach(std::mem::take(&mut whole.notes), reached);
        whole
    }

    fn rerun(
        &self,
        source: &str,
        reservation: Reservation<'_>,
        (room, reserved): (Option<InputRoom>, u64),
    ) -> InputResult {
        reset_synthetic_spans();
        untwist_deobfuscate::reset_run_state();
        let _ = crate::stats::RunStats::taken();
        untwist_deobfuscate::init_order::reset_capacity_hint();
        forget_failures();
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
            let code = respelled_as(code, result.respelling);
            if result.readers.unguarded(&code) {
                return Some(Err(SOURCE_READER_UNGUARDED));
            }
            result.notes.extend(guard_notes(guards.tally()));
            result.guards = guards;
            Some(Ok(code))
        }));
        let outcome = match outcome {
            Ok(None) if result.parse_errors > 0 => return result.refused_for_syntax(source),
            Ok(None) => {
                let note = result.parse_refusal.unwrap_or(TYPESCRIPT_IN_JAVASCRIPT);
                return InputResult::refused(source, note);
            }
            Ok(Some(Err(note))) => return InputResult::refused(source, note),
            Ok(Some(Ok(code))) => Ok(code),
            Err(panic) => Err(panic),
        };
        settle(outcome, source, &mut result);
        if self.options.source_map {
            result.source_map = Some(final_map(source, &mut result));
        }
        result.notes = consolidate_reach(dedupe_spanned(std::mem::take(&mut result.notes)));
        if !self.options.unsafe_transforms {
            result.notes.extend(unsafe_lane_hint(&result.notes));
        }
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
        let mut replaying = false;
        loop {
            let marks = Marks::of(result, &layering, room.as_ref());
            let layer = (reservation, room.take(), form, scoped);
            let undo = match std::mem::take(&mut replaying) {
                true => RoundCapUndo::Replaying,
                false => RoundCapUndo::Replay,
            };
            let step = self.process_layer(
                &current,
                (layer, undo),
                &mut layering,
                &mut fallback,
                result,
            );
            match step {
                Step::Done(code) => return code,
                Step::Replay => {
                    marks.restore(result, &mut layering);
                    room = marks.room.map(RoomSeed::room);
                    replaying = true;
                }
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
    pub(super) fn records_refusal_spans(&self) -> bool {
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
        ((reservation, room, form, scoped), undo): (
            (Reservation<'_>, Option<InputRoom>, Option<LayerForm>, bool),
            RoundCapUndo,
        ),
        layering: &mut Layering,
        fallback: &mut Option<Fallback<'_>>,
        result: &mut InputResult,
    ) -> Step {
        let foreign = reservation.shared.cloned().map(SharedGlobals::foreign_only);
        let _foreign = assume_shared_globals(foreign);
        let first = layering.peeled() == 0;
        if layering.start_run() {
            fresh_tree_state(room.as_ref());
        }
        self.options.progress(Progress::StageBegins("parse"));
        let alloc = Allocator::default();
        let mut parsed = match (first, reservation.module_named, reservation.script_named) {
            (true, true, _) => parse_named_module(&alloc, text),
            (true, false, true) => parse_named_script(&alloc, text),
            (true, false, false) => parse_program(&alloc, text),
            (false, _, _) => parse_with(&alloc, text, SourceType::script()),
        };
        result.module_goal |= first && parsed.program.source_type.is_module();
        let errors = match parse_outcome(text, &parsed, first, result) {
            Ok(errors) => errors,
            Err(step) => return step,
        };
        result
            .notes
            .extend(super::notes::vm_interpreter_notes(&parsed.program));
        let unsafe_transforms = self.options.unsafe_transforms;
        if let Some(Err(reason)) =
            form.map(|form| verdict(form, &parsed.program, unsafe_transforms))
        {
            return Step::Kept(reason);
        }
        *fallback = None;
        if errors == 0 {
            let site = InlineSite {
                notes: &mut result.notes,
                stages: &mut result.stages,
                heap_peak: &mut result.heap_peak,
                scoped,
            };
            if let Some(peeled) = self.peel_inline(&parsed.program, layering, site) {
                return Step::Peel(peeled);
            }
        }
        result.snapshots.enter_layer(layering.peeled());
        let decoding = self.options.decoded_strings.then(|| {
            (
                layering.peeled(),
                untwist_deobfuscate::decoded_strings::Recorder::start(),
            )
        });
        let origins = self.options.source_map.then(Recording::start);
        #[cfg(feature = "refusal-spans")]
        let recorder = self.records_refusal_spans().then(|| {
            (
                layering.peeled(),
                untwist_deobfuscate::refusal_spans::Recorder::start(),
            )
        });
        let mut trail = Trail::default();
        let staged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let tree = (&alloc, &mut parsed.program, &mut trail);
            let checked = first && errors == 0;
            let layer = (reservation, room, checked, scoped);
            self.stage_layer(tree, (layer, undo), layering, result)
        }));
        let replay = matches!(staged, Ok(Staged::Replay));
        if self.options.decoded_strings || self.options.refusal_spans {
            self.options.progress(Progress::StageBegins("locate"));
        }
        #[cfg(feature = "refusal-spans")]
        if let Some((layer, recorder)) = recorder.filter(|_| !replay) {
            let sites = super::refusal_spans::located(text, layer, recorder.finish());
            result.refusal_sites.extend(sites);
        }
        if let Some((layer, recorder)) = decoding.filter(|_| !replay) {
            let found = super::decoded::sited(text.len(), layer, recorder.finish());
            result.decoded_strings.extend(found);
        }
        match staged {
            Ok(Staged::Replay) => Step::Replay,
            Ok(Staged::TypeScript) => Step::Done(None),
            Ok(Staged::Peeled(peeled)) => Step::Peel(peeled),
            Ok(Staged::Done) => {
                if first && self.options.statement_spans {
                    result.statement_spans = Some(traced(&trail, statement_spans(&parsed.program)));
                }
                result.readers = PendingReaders::of(&parsed.program);
                self.options.progress(Progress::StageBegins("print"));
                let printed = self.printed(&alloc, &mut parsed.program, errors == 0);
                if let Some(origins) = origins {
                    result.mapped = (layering.peeled() == 0)
                        .then(|| source_map::mapped(&mut parsed.program, &origins.finish()))
                        .flatten()
                        .map(|mapped| Mapped {
                            map: trail.composed(mapped.map),
                            ..mapped
                        });
                }
                result.respelling = respelling(&parsed.program, printed.text());
                Step::Done(Some(printed))
            }
            Err(_) => {
                result.note_panicked();
                result.readers = readers_after_a_panic(&parsed.program);
                let printed = Printed::unplaced(pretty_codegen(&parsed.program));
                result.respelling = respelling(&parsed.program, printed.text());
                Step::Done(Some(printed))
            }
        }
    }

    fn stage_layer<'a>(
        &self,
        (alloc, program, trail): (&'a Allocator, &mut Program<'a>, &mut Trail<'a>),
        ((reservation, room, checked, scoped), undo): (
            (Reservation<'_>, Option<InputRoom>, bool, bool),
            RoundCapUndo,
        ),
        layering: &mut Layering,
        result: &mut InputResult,
    ) -> Staged {
        let stripped = strip_shell((alloc, program), &mut result.stages, &mut result.notes);
        let nested_root = stripped || scoped;
        let pass_opts = PassOptions {
            room,
            nested_root,
            round_cap_undo: Cell::new(undo),
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
            trail,
        };
        let own_globals = !nested_root && !site.program.source_type.is_module();
        let shared = reservation.shared.cloned().map(|shared| match own_globals {
            true => shared,
            false => shared.foreign_only(),
        });
        let _shared = assume_shared_globals(shared);
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
        if !site.pass_opts.extended {
            let sem = self.front_stages(site, sem);
            if site.pass_opts.round_cap_undo.get() == RoundCapUndo::Requested {
                return Staged::Replay;
            }
            if let Some(peeled) = self.peel(site, &sem, layering) {
                return Staged::Peeled(peeled);
            }
            self.shaping_stages(site, sem, reservation);
            return Staged::Done;
        }
        self.layer_rounds(site, (sem, reservation), layering)
    }
}

fn traced(trail: &Trail<'_>, spans: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    match trail.is_empty() {
        true => spans,
        false => trail
            .spots(spans.into_iter().map(Some).collect())
            .into_iter()
            .flatten()
            .filter(|(start, end)| start < end)
            .collect(),
    }
}

fn final_map(source: &str, result: &mut InputResult) -> OwnedSourceMap {
    match result.mapped.take() {
        Some(mapped) if !result.panicked => {
            source_map::retarget(mapped.map, &mapped.printed, &result.code)
        }
        _ if result.code == source => source_map::identity(source),
        _ => source_map::unmapped(source),
    }
}

fn parse_outcome(
    text: &str,
    parsed: &Parsed<'_>,
    first: bool,
    result: &mut InputResult,
) -> Result<usize, Step> {
    let parse_failed = has_error(&parsed.diagnostics);
    if !first {
        return match parse_failed || !early_errors(&parsed.program).is_clean() {
            true => Err(Step::Kept(Refusal::Unparsable)),
            false => Ok(0),
        };
    }
    if parse_failed && holds_type_space(&parsed.program) {
        return Err(Step::Done(None));
    }
    let early = match parse_failed {
        true => {
            let failure = parse_failure(text, &parsed.diagnostics);
            result.parse_refusal =
                (failure == ParseFailure::Unsupported).then_some(PARSE_UNSUPPORTED);
            Vec::new()
        }
        false => match early_errors(&parsed.program) {
            Early::Clean => Vec::new(),
            Early::SyntaxError(found) => found,
            Early::ReadByNode(found) => {
                result.parse_refusal = Some(PARSE_UNSUPPORTED);
                found
            }
            Early::UnmodelledAnnexB(_) => {
                result.parse_refusal = Some(ANNEX_B_UNMODELLED);
                return Err(Step::Done(None));
            }
        },
    };
    let (mut diagnostics, errors) = summarize_diagnostics(text, &parsed.diagnostics);
    let (early_diagnostics, early_count) = summarize_diagnostics(text, &early);
    diagnostics.extend(early_diagnostics);
    let errors = errors + early_count;
    result.parse_diagnostics = diagnostics;
    result.parse_errors = u32::try_from(errors).unwrap_or(u32::MAX);
    match errors {
        0 => Ok(0),
        _ => Err(Step::Done(None)),
    }
}

pub(super) enum Staged {
    TypeScript,
    Peeled(Peeled),
    Replay,
    Done,
}

enum Step {
    Done(Option<Printed>),
    Peel(Peeled),
    Kept(Refusal),
    Replay,
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

fn readers_after_a_panic(program: &oxc_ast::ast::Program<'_>) -> PendingReaders {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| PendingReaders::of(program)))
        .unwrap_or(PendingReaders::Unresolved)
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
            shared: None,
            module_named: false,
            script_named: false,
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
