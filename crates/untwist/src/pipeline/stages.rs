use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{
    run_to_fixpoint, run_to_fixpoint_noting_last, Note, NoteKind, Pass, PassCtor, PassGroup,
    PassOptions, RoundCapUndo, RunReport,
};
use untwist_core::semantic::Semantics;
use untwist_deobfuscate::driver::{run_cleanup_settling, CleanupSlot, PreparedSlot};
use untwist_deobfuscate::{
    drop_cleared_observation, run_deobfuscate_prepared, run_final_machinery, run_machinery_sweep,
    run_protections,
};
use untwist_rename::{rename_passes_with_state, shorthand, RenameConfig, RenameMode};
use untwist_unminify::{
    lexical_ctors, prepare_ctors, run_last_traversal, transpile_ctors, unminify_ctors,
    untranspile_ctors,
};

use super::notes::{
    retain_last_spanned, retain_latest_protection_notes, StageRun, SLOT_NOT_RESERVED,
};
use super::snapshots::Snapshots;
use super::{Pipeline, Reservation};
use crate::options::Progress;
use crate::plugin::Stage;
use crate::report::StageReport;

const SETTLE_ROUNDS: usize = 3;

const SETTLE_CAPPED: &str = "settle: unminify and cleanup did not reach a fixpoint within \
     the extra rounds; running untwist on this output again may change it";

pub(super) struct StageCx<'a, 'r> {
    pub(super) alloc: &'a Allocator,
    pub(super) program: &'r mut Program<'a>,
    pub(super) pass_opts: &'r PassOptions,
    pub(super) stages: &'r mut Vec<StageReport>,
    pub(super) notes: &'r mut Vec<Note>,
    pub(super) heap_peak: &'r mut u64,
    pub(super) snapshots: &'r mut Snapshots,
}

impl StageCx<'_, '_> {
    fn record(&mut self, name: &'static str, started: Instant, report: RunReport) {
        let round = round_of(self.stages, name);
        self.stages
            .push(StageReport::from_run(name, round, started, &report));
        self.notes.extend(report.notes);
        self.snapshots.take(name, round, self.program);
    }
}

pub(super) fn round_of(stages: &[StageReport], name: &str) -> u32 {
    let earlier = stages.iter().filter(|stage| stage.stage == name).count();
    u32::try_from(earlier + 1).unwrap_or(u32::MAX)
}

fn record_sweep(site: &mut StageCx<'_, '_>, started: Instant, report: RunReport) {
    drop_cleared_observation(site.notes, &report);
    site.record("deobfuscate", started, report);
}

fn settle_notes(site: &mut StageCx<'_, '_>, runs: &[StageRun], latest: Option<usize>) {
    if runs.len() > 2 {
        *site.notes = retain_last_spanned(std::mem::take(site.notes), runs);
    }
    if let Some(latest) = latest {
        retain_latest_protection_notes(site.notes, latest);
    }
}

impl Pipeline {
    pub(super) fn begin(&self, stage: &'static str) -> Instant {
        self.options.progress(Progress::StageBegins(stage));
        Instant::now()
    }

    pub(super) fn front_stages<'a>(&self, site: &mut StageCx<'a, '_>, sem: Semantics) -> Semantics {
        let sem = self.plugin_stage(Stage::AfterParse, site, sem);
        let (sem, prepared) = self.prepare_stage(site, sem);
        let sem = self.plugin_stage(Stage::AfterPrepare, site, sem);
        let sem = self.deobfuscate_stage(site, (sem, prepared));
        if site.pass_opts.round_cap_undo.get() == RoundCapUndo::Requested {
            return sem;
        }
        self.plugin_stage(Stage::AfterDeobfuscate, site, sem)
    }

    pub(super) fn shaping_stages<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
        reservation: Reservation<'_>,
    ) {
        let unminify = self.unminify_passes();
        let mut runs: Vec<StageRun> = Vec::new();
        let started = site.notes.len();
        let sem = match &unminify {
            Some(ctors) => self.stage("unminify", ctors, site, sem),
            None => sem,
        };
        runs.push(("unminify", started, site.notes.len()));
        let sem = self.plugin_stage(Stage::AfterUnminify, site, sem);
        let sem = self.machinery_sweep_stage(site, sem);
        let extended = site.pass_opts.extended;
        let sem = match extended {
            true => sem,
            false => self.rename_stage(site, sem, reservation),
        };
        let sem = self.protections_stage(site, sem);
        let sem = self.jsx_stage(site, sem);
        let started = site.notes.len();
        let mut slot = CleanupSlot::default();
        let cleaned = self.cleanup_stage(site, sem, &mut slot);
        runs.push(("cleanup", started, site.notes.len()));
        let (sem, changes, latest) =
            self.late_protections_stage(site, cleaned, &mut slot, &mut runs);
        if !extended {
            let (sem, changed) = self.final_machinery_stage(site, sem);
            if changed {
                self.rename_stage(site, sem, reservation);
            }
            settle_notes(site, &runs, latest);
            return;
        }
        let settle_from = (changes, slot);
        let sem = self.settle(site, sem, unminify.as_deref(), settle_from, &mut runs);
        let (sem, _) = self.final_machinery_stage(site, sem);
        if let Some(sem) = self.lexical_stage(site, sem) {
            self.rename_stage(site, sem, reservation);
        }
        settle_notes(site, &runs, latest);
    }

    fn late_protections_stage<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        (sem, changes): (Semantics, u32),
        slot: &mut CleanupSlot,
        runs: &mut Vec<StageRun>,
    ) -> (Semantics, u32, Option<usize>) {
        if changes == 0 {
            return (sem, changes, None);
        }
        let started = Instant::now();
        let (sem, report) = run_protections(site.alloc, site.program, sem, site.pass_opts);
        if report.total_changes == 0 {
            return (sem, changes, None);
        }
        let latest = report.notes.len();
        site.record("protections", started, report);
        let noted = site.notes.len();
        let (sem, cleaned) = self.cleanup_stage(site, sem, slot);
        runs.push(("cleanup", noted, site.notes.len()));
        (sem, changes.saturating_add(cleaned), Some(latest))
    }

    fn unminify_passes<'a>(&self) -> Option<Vec<PassCtor<'a>>> {
        if !self.options.unminify {
            return None;
        }
        let lists = [transpile_ctors(), unminify_ctors(), untranspile_ctors()];
        let mut ctors: Vec<PassCtor<'a>> = Vec::with_capacity(lists.iter().map(Vec::len).sum());
        lists.into_iter().for_each(|list| ctors.extend(list));
        Some(ctors)
    }

    fn lexical_stage<'a>(&self, site: &mut StageCx<'a, '_>, sem: Semantics) -> Option<Semantics> {
        if !self.options.unminify {
            return Some(sem);
        }
        let started = self.begin("lexical");
        let opts = PassOptions {
            keep_captured_vars: self.options.unpack
                && untwist_unpack::holds_a_bundle_with(site.program, self.unpack_options()),
            ..site.pass_opts.clone()
        };
        let mut group = PassGroup::from_ctors(&lexical_ctors(), opts);
        let renames = self.options.rename != RenameMode::None || self.names_wanted(site);
        let (kept, report) = run_last_traversal(site.alloc, site.program, sem, &mut group, renames);
        site.record("lexical", started, report);
        kept
    }

    fn settle<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        mut sem: Semantics,
        unminify: Option<&[PassCtor<'a>]>,
        (mut changes, mut slot): (u32, CleanupSlot),
        runs: &mut Vec<StageRun>,
    ) -> Semantics {
        let Some(ctors) = unminify else {
            return sem;
        };
        for _ in 0..SETTLE_ROUNDS {
            if changes == 0 || untwist_core::budget::check().is_err() {
                return sem;
            }
            let started = site.notes.len();
            sem = self.stage("unminify", ctors, site, sem);
            runs.push(("unminify", started, site.notes.len()));
            let started = site.notes.len();
            let settled = self.cleanup_stage(site, sem, &mut slot);
            runs.push(("cleanup", started, site.notes.len()));
            sem = settled.0;
            changes = settled.1;
        }
        if changes > 0 {
            site.notes.push(Note {
                pass: "pipeline",
                kind: NoteKind::Refusal,
                message: SETTLE_CAPPED.to_string(),
                span: None,
            });
        }
        sem
    }

    fn rename_stage<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
        reservation: Reservation<'_>,
    ) -> Semantics {
        let sem = self.names_stage(site, sem);
        if self.options.rename == RenameMode::None {
            return sem;
        }
        let config = RenameConfig {
            mode: self.options.rename,
            rename_all: self.options.rename_all,
            extended: self.options.extended(),
        };
        let Some(slot) = reservation.slot else {
            site.notes.push(Note {
                pass: "rename",
                kind: NoteKind::Refusal,
                message: SLOT_NOT_RESERVED.to_string(),
                span: None,
            });
            return sem;
        };
        let (keep_script_globals, keep_script_lexicals) = keeps_script_globals(site, &sem);
        let opts = PassOptions {
            keep_script_globals,
            keep_script_lexicals,
            ..site.pass_opts.clone()
        };
        let passes = rename_passes_with_state(config, reservation.state, slot);
        let sem = self.stage_group("rename", passes, site, (sem, opts.clone()));
        let shorthand = vec![shorthand::boxed()];
        self.stage_group("rename-shorthand", shorthand, site, (sem, opts))
    }

    fn names_wanted(&self, site: &StageCx<'_, '_>) -> bool {
        !site.pass_opts.rule_disabled(untwist_rename::commonjs::PASS)
            && untwist_rename::commonjs::may_hold_wrappers(site.program)
    }

    fn names_stage<'a>(&self, site: &mut StageCx<'a, '_>, sem: Semantics) -> Semantics {
        if !self.names_wanted(site) {
            return sem;
        }
        let (keep_script_globals, keep_script_lexicals) = keeps_script_globals(site, &sem);
        let opts = PassOptions {
            keep_script_globals,
            keep_script_lexicals,
            ..site.pass_opts.clone()
        };
        let passes = vec![untwist_rename::commonjs::boxed()];
        self.stage_group("rename", passes, site, (sem, opts))
    }

    fn protections_stage<'a>(&self, site: &mut StageCx<'a, '_>, sem: Semantics) -> Semantics {
        if !self.options.deobfuscate {
            return sem;
        }
        let started = self.begin("protections");
        let (sem, report) = run_protections(site.alloc, site.program, sem, site.pass_opts);
        site.record("protections", started, report);
        sem
    }

    fn cleanup_stage<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
        slot: &mut CleanupSlot,
    ) -> (Semantics, u32) {
        if !self.options.deobfuscate {
            return (sem, 0);
        }
        let started = self.begin("cleanup");
        let settling = (site.pass_opts, slot);
        let (sem, report) = run_cleanup_settling(site.alloc, site.program, sem, settling);
        let changes = report.total_changes;
        site.record("cleanup", started, report);
        (sem, changes)
    }

    fn prepare_stage<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
    ) -> (Semantics, PreparedSlot) {
        if !self.options.deobfuscate {
            let sem = self.stage("prepare", &prepare_ctors(), site, sem);
            return (sem, PreparedSlot::default());
        }
        let started = self.begin("prepare");
        let mut group = PassGroup::from_ctors(&prepare_ctors(), site.pass_opts.clone());
        let (sem, report, last) =
            run_to_fixpoint_noting_last(site.alloc, site.program, sem, &mut group);
        site.record("prepare", started, report);
        (sem, PreparedSlot::settled(last))
    }

    fn deobfuscate_stage<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        (sem, prepared): (Semantics, PreparedSlot),
    ) -> Semantics {
        if !self.options.deobfuscate {
            return sem;
        }
        let started = self.begin("deobfuscate");
        let (sem, report) =
            run_deobfuscate_prepared(site.alloc, site.program, (sem, prepared), site.pass_opts);
        *site.heap_peak = (*site.heap_peak).max(report.heap_charged);
        site.record("deobfuscate", started, report);
        sem
    }

    fn machinery_sweep_stage<'a>(&self, site: &mut StageCx<'a, '_>, sem: Semantics) -> Semantics {
        if !self.options.deobfuscate {
            return sem;
        }
        let started = self.begin("deobfuscate");
        let (sem, report) = run_machinery_sweep(site.alloc, site.program, sem, site.pass_opts);
        if report.iterations > 0 {
            record_sweep(site, started, report);
        }
        sem
    }

    fn final_machinery_stage<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
    ) -> (Semantics, bool) {
        if !self.options.deobfuscate {
            return (sem, false);
        }
        let started = self.begin("deobfuscate");
        let (sem, report) = run_final_machinery(site.alloc, site.program, sem, site.pass_opts);
        if report.iterations == 0 {
            return (sem, false);
        }
        let changed = report.total_changes > 0;
        record_sweep(site, started, report);
        (sem, changed)
    }

    fn stage<'a>(
        &self,
        name: &'static str,
        ctors: &[PassCtor<'a>],
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
    ) -> Semantics {
        let passes = ctors.iter().map(|ctor| ctor()).collect();
        self.stage_passes(name, passes, site, sem)
    }

    pub(super) fn stage_passes<'a>(
        &self,
        name: &'static str,
        passes: Vec<Box<dyn Pass<'a> + 'a>>,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
    ) -> Semantics {
        let opts = site.pass_opts.clone();
        self.stage_group(name, passes, site, (sem, opts))
    }

    fn stage_group<'a>(
        &self,
        name: &'static str,
        passes: Vec<Box<dyn Pass<'a> + 'a>>,
        site: &mut StageCx<'a, '_>,
        (sem, opts): (Semantics, PassOptions),
    ) -> Semantics {
        let started = self.begin(name);
        let mut group = PassGroup::new(passes, opts);
        let (sem, report) = run_to_fixpoint(site.alloc, site.program, sem, &mut group);
        site.record(name, started, report);
        sem
    }

    fn jsx_stage<'a>(&self, site: &mut StageCx<'a, '_>, sem: Semantics) -> Semantics {
        if !self.options.jsx {
            return sem;
        }
        let started = self.begin("jsx");
        let (sem, report) = untwist_jsx::run_jsx(site.alloc, site.program, sem, site.pass_opts);
        site.record("jsx", started, report);
        sem
    }
}

const RENAME_KEEPS_GLOBALS: &str = "rename-keeps-globals";

fn keeps_script_globals(site: &StageCx<'_, '_>, sem: &Semantics) -> (bool, bool) {
    let program: &Program<'_> = site.program;
    let guarded = site.pass_opts.extended
        && !site.pass_opts.unsafe_transforms
        && !site.pass_opts.nested_root
        && !site.pass_opts.rule_disabled(RENAME_KEEPS_GLOBALS)
        && !untwist_core::semantic::has_module_syntax(program);
    if !guarded {
        return (false, false);
    }
    let agnostic = untwist_deobfuscate::calls::agnostic::globals_reachable_by_key;
    let lexicals = untwist_deobfuscate::calls::agnostic::script_lexicals_reachable_by_code;
    (
        agnostic(program, &sem.scoping),
        lexicals(program, &sem.scoping),
    )
}
