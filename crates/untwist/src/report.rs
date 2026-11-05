use std::time::Duration;

use serde::Serialize;
use untwist_core::clock::StageInstant;
use untwist_core::pass::{IterationTiming, RunReport, StopReason};

#[derive(Clone, Debug)]
pub struct StageReport {
    pub stage: &'static str,
    pub round: u32,
    pub iterations: u32,
    pub changes: u32,
    pub per_pass: Vec<(&'static str, u32)>,
    pub per_pass_elapsed: Vec<(&'static str, Duration)>,
    pub fixpoint_per_pass: Vec<(&'static str, u32)>,
    pub fixpoint_per_pass_elapsed: Vec<(&'static str, Duration)>,
    pub iteration_elapsed: Vec<IterationTiming>,
    pub elapsed: Duration,
    pub stop: StopReason,
    #[cfg(feature = "test-hooks")]
    pub work: Vec<untwist_core::pass::Traversal>,
}

impl StageReport {
    pub fn from_run(
        stage: &'static str,
        round: u32,
        started: StageInstant,
        report: &RunReport,
    ) -> Self {
        Self {
            stage,
            round,
            iterations: report.iterations,
            changes: report.total_changes,
            per_pass: report.per_pass.clone(),
            per_pass_elapsed: report.per_pass_elapsed.clone(),
            fixpoint_per_pass: report.fixpoint_per_pass.clone(),
            fixpoint_per_pass_elapsed: report.fixpoint_per_pass_elapsed.clone(),
            iteration_elapsed: report.iteration_elapsed.clone(),
            elapsed: started.elapsed(),
            stop: report.stop,
            #[cfg(feature = "test-hooks")]
            work: untwist_core::pass::take_work_for_tests(),
        }
    }
}

type IterationMs = (f64, Option<f64>, Option<f64>);

#[derive(Serialize)]
pub struct StageRow<'a> {
    stage: &'a str,
    round: u32,
    iterations: u32,
    changes: u32,
    per_pass: Vec<(&'a str, u32)>,
    per_pass_ms: Vec<(&'a str, f64)>,
    fixpoint_per_pass: Vec<(&'a str, u32)>,
    fixpoint_per_pass_ms: Vec<(&'a str, f64)>,
    iterations_ms: Vec<IterationMs>,
    elapsed_ms: f64,
    stop: &'static str,
    #[cfg(feature = "test-hooks")]
    work: Vec<WorkRow>,
}

#[cfg(feature = "test-hooks")]
#[derive(Serialize)]
pub struct WorkRow {
    changes: u32,
    walk: &'static str,
    lists_parked: u32,
    statements_shelved: u32,
}

#[cfg(feature = "test-hooks")]
fn work_rows(stage: &StageReport) -> Vec<WorkRow> {
    stage
        .work
        .iter()
        .map(|traversal| WorkRow {
            changes: traversal.changes,
            walk: match (traversal.walked, traversal.steered, traversal.confirming) {
                (false, _, _) => "parked",
                (true, _, true) => "confirming",
                (true, false, false) => "walked",
                (true, true, false) => "steered",
            },
            lists_parked: traversal.lists_parked,
            statements_shelved: traversal.statements_shelved,
        })
        .collect()
}

fn millis(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

fn named_ms(rows: &[(&'static str, Duration)]) -> Vec<(&'static str, f64)> {
    rows.iter()
        .map(|(name, elapsed)| (*name, millis(*elapsed)))
        .collect()
}

fn iterations_ms(stage: &StageReport) -> Vec<IterationMs> {
    stage
        .iteration_elapsed
        .iter()
        .map(|timing| {
            (
                millis(timing.traverse()),
                timing.rebuild().map(millis),
                timing.fingerprint().map(millis),
            )
        })
        .collect()
}

pub fn stage_rows(stages: &[StageReport]) -> Vec<StageRow<'_>> {
    stages
        .iter()
        .map(|s| StageRow {
            stage: s.stage,
            round: s.round,
            iterations: s.iterations,
            changes: s.changes,
            per_pass: s.per_pass.clone(),
            per_pass_ms: named_ms(&s.per_pass_elapsed),
            fixpoint_per_pass: s.fixpoint_per_pass.clone(),
            fixpoint_per_pass_ms: named_ms(&s.fixpoint_per_pass_elapsed),
            iterations_ms: iterations_ms(s),
            elapsed_ms: millis(s.elapsed),
            stop: s.stop.as_str(),
            #[cfg(feature = "test-hooks")]
            work: work_rows(s),
        })
        .collect()
}
