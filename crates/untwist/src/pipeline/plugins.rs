use std::time::Duration;

use oxc_allocator::Allocator;
use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{run_to_fixpoint, Note, PassGroup, PassOptions, RunReport, StopReason};
use untwist_core::semantic::{build_scoping, Semantics};

use super::stages::{round_of, StageCx};
use super::Pipeline;
use crate::plugin::Stage;
use crate::report::StageReport;

fn stop_rank(stop: StopReason) -> u8 {
    match stop {
        StopReason::Converged => 0,
        StopReason::IterationCap => 1,
        StopReason::Cycle => 2,
    }
}

fn add_by_name(acc: &mut Vec<(&'static str, u32)>, latest: &[(&'static str, u32)]) {
    for (name, count) in latest {
        match acc.iter_mut().find(|(seen, _)| seen == name) {
            Some(slot) => slot.1 += count,
            None => acc.push((name, *count)),
        }
    }
}

fn add_elapsed_by_name(
    acc: &mut Vec<(&'static str, Duration)>,
    latest: &[(&'static str, Duration)],
) {
    for (name, elapsed) in latest {
        match acc.iter_mut().find(|(seen, _)| seen == name) {
            Some(slot) => slot.1 += *elapsed,
            None => acc.push((name, *elapsed)),
        }
    }
}

fn fold_module_run(row: &mut StageReport, report: &RunReport) {
    row.iterations = row.iterations.max(report.iterations);
    row.changes += report.total_changes;
    add_by_name(&mut row.per_pass, &report.per_pass);
    add_elapsed_by_name(&mut row.per_pass_elapsed, &report.per_pass_elapsed);
    add_by_name(&mut row.fixpoint_per_pass, &report.fixpoint_per_pass);
    add_elapsed_by_name(
        &mut row.fixpoint_per_pass_elapsed,
        &report.fixpoint_per_pass_elapsed,
    );
    row.iteration_elapsed
        .reserve_exact(report.iteration_elapsed.len());
    row.iteration_elapsed
        .extend_from_slice(&report.iteration_elapsed);
    if stop_rank(report.stop) > stop_rank(row.stop) {
        row.stop = report.stop;
    }
}

impl Pipeline {
    pub(super) fn pass_options(&self) -> PassOptions {
        PassOptions {
            unsafe_transforms: self.options.unsafe_transforms,
            extended: self.options.extended(),
            max_iterations: self.options.max_iterations,
            fuel: self.options.fuel,
            heap_budget: self.options.hooks().heap_budget,
            disabled_rules: self.options.disabled_passes.clone(),
            ..PassOptions::default()
        }
    }

    pub(super) fn plugin_stage<'a>(
        &self,
        stage: Stage,
        site: &mut StageCx<'a, '_>,
        sem: Semantics,
    ) -> Semantics {
        let passes = self.options.plugins.passes_at(stage, site.alloc);
        if passes.is_empty() {
            return sem;
        }
        self.stage_passes(stage.label(), passes, site, sem)
    }

    pub(super) fn after_unpack<'a>(
        &self,
        alloc: &'a Allocator,
        bundle: &mut untwist_unpack::Bundle<'a>,
        stages: &mut Vec<StageReport>,
        notes: &mut Vec<Note>,
    ) {
        if !self.options.plugins.has(Stage::AfterUnpack) {
            return;
        }
        let pass_opts = self.pass_options();
        let started = Instant::now();
        let label = Stage::AfterUnpack.label();
        let mut row = StageReport::from_run(
            label,
            round_of(stages, label),
            started,
            &RunReport::default(),
        );
        for module in bundle.modules.values_mut() {
            let program = module.program_mut();
            let sem = build_scoping(program);
            let passes = self.options.plugins.passes_at(Stage::AfterUnpack, alloc);
            let mut group = PassGroup::new(passes, pass_opts.clone());
            let (_sem, report) = run_to_fixpoint(alloc, program, sem, &mut group);
            fold_module_run(&mut row, &report);
            notes.extend(report.notes);
        }
        row.elapsed = started.elapsed();
        stages.push(row);
    }
}
