mod boundary;
mod decoded;
mod input;
mod ladder;
mod layer;
mod notes;
#[cfg(feature = "parallel")]
mod parallel;
mod plugins;
mod refusal;
#[cfg(feature = "refusal-spans")]
mod refusal_spans;
mod shell;
mod snapshots;
mod stages;
mod unpack;

use std::num::NonZeroUsize;

use oxc_diagnostics::OxcDiagnostic;
use untwist_core::parse::error_count;
use untwist_core::pass::Note;
use untwist_core::reflow::GuardRecord;
use untwist_eval::stack_reserve::StackReserve;
use untwist_rename::unique::{reserve, shared_state, FileSlot, SharedUniqueState};

use crate::bundle::BundleOutput;
use crate::diagnostics::format_diagnostic;
use crate::options::{Options, Progress};
use crate::report::StageReport;
pub use decoded::DecodedSite;
use input::InputResult;
use ladder::Ladder;
#[cfg(feature = "refusal-spans")]
pub use refusal_spans::{RefusalSite, SiteAt};
pub use snapshots::{Snapshot, SnapshotCode};

pub use notes::{
    ADMIT_PANICKED, ANONYMOUS_SHELL_KEPT, ANONYMOUS_SHELL_STRIPPED, GUARD_KEPT_ACROSS_LINES,
    GUARD_ON_ONE_LINE, NESTED_TOO_DEEP, PANICKED, PARENTHESES_TOO_DEEP, PIPELINE_NOTES,
    PRINT_TOO_LARGE, TYPESCRIPT_IN_JAVASCRIPT, UNPACK_PANICKED, UNPACK_TEXT_TOO_DEEP, WORKER_DIED,
};

pub struct Output {
    pub codes: Vec<String>,
    pub stages: Vec<StageReport>,
    pub parse_diagnostics: Vec<String>,
    pub parse_errors: usize,
    pub notes: Vec<Note>,
    pub bundle: Option<BundleOutput>,
    pub panicked: Vec<usize>,
    pub depth_refused: Vec<usize>,
    pub unpack_panicked: bool,
    pub single_file: bool,
    pub heap_peak: u64,
    pub stats: crate::stats::RunStats,
    pub snapshots: Vec<Snapshot>,
    pub decoded_strings: Vec<DecodedSite>,
    pub statement_spans: Option<Vec<(u32, u32)>>,
    #[cfg(feature = "refusal-spans")]
    pub refusal_spans: Vec<String>,
    #[cfg(feature = "refusal-spans")]
    pub refusal_sites: Vec<RefusalSite>,
    pub wasm: Vec<crate::wasm::WasmFile>,
    pub wasm_unresolved: Vec<crate::wasm::WasmReference>,
}

pub(crate) fn admits_reparse(text: &str) -> bool {
    input::guard_admits(input::FinalTextGuard::Scan, text)
}

impl Output {
    pub fn code(&self) -> &str {
        self.codes.first().map_or("", String::as_str)
    }
}

#[cfg(feature = "parallel")]
pub fn job_width(requested: Option<NonZeroUsize>, inputs: usize) -> usize {
    parallel::width(requested, inputs)
}

#[cfg(not(feature = "parallel"))]
pub fn job_width(_requested: Option<NonZeroUsize>, _inputs: usize) -> usize {
    1
}

pub struct Pipeline {
    options: Options,
}

#[derive(Clone, Copy)]
struct Reservation<'r> {
    state: &'r SharedUniqueState,
    slot: Option<FileSlot>,
    stack: &'r StackReserve<'static>,
    stats: &'r std::sync::Mutex<crate::stats::RunStats>,
}

struct Batch<'s> {
    sources: &'s [&'s str],
    rename_state: SharedUniqueState,
    slots: Vec<Option<FileSlot>>,
    stack: StackReserve<'static>,
    stats: std::sync::Mutex<crate::stats::RunStats>,
}

impl<'s> Batch<'s> {
    fn new(sources: &'s [&'s str]) -> Self {
        let rename_state = shared_state();
        let slots = (0..sources.len())
            .map(|index| reserve(&rename_state, index))
            .collect();
        Self {
            sources,
            rename_state,
            slots,
            stack: StackReserve::default(),
            stats: std::sync::Mutex::default(),
        }
    }

    fn reservation(&self, index: usize) -> Reservation<'_> {
        Reservation {
            state: &self.rename_state,
            slot: self.slots.get(index).copied().flatten(),
            stack: &self.stack,
            stats: &self.stats,
        }
    }
}

#[derive(Default)]
struct OutputParts {
    codes: Vec<String>,
    stages: Vec<StageReport>,
    notes: Vec<Note>,
    parse_diagnostics: Vec<String>,
    parse_errors: usize,
    panicked: Vec<usize>,
    depth_refused: Vec<usize>,
    unpack_panicked: bool,
    heap_peak: u64,
    stats: crate::stats::RunStats,
    guards: GuardRecord,
    unbundled: bool,
    snapshots: Vec<Snapshot>,
    decoded_strings: Vec<DecodedSite>,
    statement_spans: Option<Vec<(u32, u32)>>,
    #[cfg(feature = "refusal-spans")]
    refusal_spans: Vec<String>,
    #[cfg(feature = "refusal-spans")]
    refusal_sites: Vec<RefusalSite>,
}

impl OutputParts {
    fn from_results(results: Vec<InputResult>) -> Self {
        let mut parts = OutputParts {
            unbundled: true,
            ..OutputParts::default()
        };
        let several = results.len() > 1;
        for (index, result) in results.into_iter().enumerate() {
            #[cfg(feature = "refusal-spans")]
            parts.absorb_sites(index, several, result.refusal_sites);
            parts
                .snapshots
                .extend(result.snapshots.rows.into_iter().map(|row| Snapshot {
                    input: index,
                    ..row
                }));
            parts
                .decoded_strings
                .extend(result.decoded_strings.into_iter().map(|site| DecodedSite {
                    input: index,
                    ..site
                }));
            parts.statement_spans = result.statement_spans.filter(|_| !several);
            parts.unbundled &= result.refused || result.unbundled;
            parts.codes.push(result.code);
            parts.stages.extend(result.stages);
            parts.notes.extend(result.notes);
            parts.parse_diagnostics.extend(result.parse_diagnostics);
            parts.parse_errors += result.parse_errors as usize;
            parts.panicked.extend(result.panicked.then_some(index));
            parts.depth_refused.extend(result.refused.then_some(index));
            parts.heap_peak = parts.heap_peak.max(result.heap_peak);
            parts.guards.absorb(result.guards);
        }
        parts
    }

    #[cfg(feature = "refusal-spans")]
    fn absorb_sites(&mut self, input: usize, several: bool, sites: Vec<RefusalSite>) {
        self.refusal_spans
            .extend(refusal_spans::spans_of(&sites).into_iter().map(|span| {
                if several {
                    format!("input {input}: {span}")
                } else {
                    span
                }
            }));
        self.refusal_sites
            .extend(sites.into_iter().map(|site| RefusalSite { input, ..site }));
    }

    fn into_output(self, bundle: Option<BundleOutput>, single_file: bool) -> Output {
        Output {
            codes: self.codes,
            stages: self.stages,
            parse_diagnostics: self.parse_diagnostics,
            parse_errors: self.parse_errors,
            notes: self.notes,
            bundle,
            panicked: self.panicked,
            depth_refused: self.depth_refused,
            unpack_panicked: self.unpack_panicked,
            single_file,
            heap_peak: self.heap_peak,
            stats: self.stats,
            snapshots: self.snapshots,
            decoded_strings: self.decoded_strings,
            statement_spans: self.statement_spans,
            #[cfg(feature = "refusal-spans")]
            refusal_spans: self.refusal_spans,
            #[cfg(feature = "refusal-spans")]
            refusal_sites: self.refusal_sites,
            wasm: Vec::new(),
            wasm_unresolved: Vec::new(),
        }
    }
}

fn summarize_diagnostics(source: &str, diagnostics: &[OxcDiagnostic]) -> (Vec<String>, usize) {
    let formatted = diagnostics
        .iter()
        .map(|diagnostic| format_diagnostic(source, diagnostic))
        .collect();
    (formatted, error_count(diagnostics))
}

impl Pipeline {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub fn run(&self, source: &str) -> Output {
        self.run_many(&[source])
    }

    pub fn run_many(&self, sources: &[&str]) -> Output {
        self.run_batch(sources, true)
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn run_reparsing_for_tests(&self, sources: &[&str]) -> Output {
        self.run_batch(sources, false)
    }

    fn run_batch(&self, sources: &[&str], trust_the_reflow: bool) -> Output {
        let ladder = Ladder::new(self.options.hooks());
        let batch = Batch::new(sources);
        let pooled = self.run_pooled(&batch, &ladder);
        ladder::on_a_worker_stack(&ladder, |_| {
            let results = pooled.unwrap_or_else(|| self.run_sequentially(&batch));
            let mut parts = OutputParts::from_results(results);
            parts.unbundled &= trust_the_reflow;
            let _ = crate::stats::RunStats::taken();
            let bundle = self.unpack_admitted(&mut parts);
            let inputs = *batch
                .stats
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            parts.stats = inputs.merged(crate::stats::RunStats::taken());
            parts.into_output(bundle, self.options.single_file)
        })
    }

    fn run_sequentially(&self, batch: &Batch<'_>) -> Vec<InputResult> {
        let results = batch
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| self.run_input(index, source, batch.reservation(index)))
            .collect();
        self.inputs_finished(batch.sources.len());
        results
    }

    #[cfg(not(feature = "parallel"))]
    fn run_pooled(&self, _batch: &Batch<'_>, _ladder: &Ladder) -> Option<Vec<InputResult>> {
        None
    }

    fn inputs_finished(&self, inputs: usize) {
        let refused = &self.options.isolation.refused;
        if (0..inputs).any(|index| !refused.contains(&index)) {
            self.options.progress(Progress::InputsFinished);
        }
    }
}
