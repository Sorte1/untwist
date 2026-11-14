mod boundary;
mod decoded;
mod global_writes;
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
mod relex;
mod shared_scan;
mod shell;
mod snapshots;
mod stages;
mod types;
mod unpack;

use std::num::NonZeroUsize;

use oxc_diagnostics::OxcDiagnostic;
use untwist_core::budget::{Deadline, Hit, Limits};
use untwist_core::parse::error_count;
use untwist_core::pass::Note;
use untwist_core::reflow::GuardRecord;
use untwist_core::source_map::{self, OwnedSourceMap};
use untwist_eval::stack_reserve::StackReserve;
use untwist_rename::unique::{reserve, shared_state, FileSlot, SharedUniqueState};

use crate::bundle::BundleOutput;
use crate::diagnostics::format_diagnostic;
use crate::options::{Options, Progress};
use crate::report::StageReport;
pub use boundary::contained;
pub(crate) use boundary::Boundary;
pub use decoded::DecodedSite;
use input::InputResult;
use ladder::Ladder;
#[cfg(feature = "refusal-spans")]
pub use refusal_spans::{RefusalSite, SiteAt};
pub use snapshots::{Snapshot, SnapshotCode};
pub use types::{TYPES_NOT_BUILT, TYPES_PASS, TYPES_WITH_SOURCE_MAPS};

pub use notes::{
    ADMIT_PANICKED, ANONYMOUS_SHELL_KEPT, ANONYMOUS_SHELL_SCOPED, ANONYMOUS_SHELL_STRIPPED,
    BUDGET_MEMORY_REACHED, BUDGET_TIME_REACHED, BUDGET_UNPACK, GUARD_KEPT_ACROSS_LINES,
    GUARD_ON_ONE_LINE, NESTED_TOO_DEEP, OVER_MEMORY_REFUSED, OVER_TIME_REFUSED, PANICKED,
    PARENTHESES_TOO_DEEP, PIPELINE_NOTES, PRINT_TOO_LARGE, SOURCE_READER_UNGUARDED,
    SOURCE_READER_UNGUARDED_MODULE, TOO_LARGE_FOR_THE_BUDGET, TYPESCRIPT_IN_JAVASCRIPT,
    UNPACK_PANICKED, UNPACK_TEXT_TOO_DEEP, UNSAFE_LANE_DECODES, WORKER_DIED,
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
    pub unpack_budget: Option<Hit>,
    pub single_file: bool,
    pub heap_peak: u64,
    pub memory_peak: u64,
    pub budget_hits: Vec<(usize, Hit)>,
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
    pub wasm_check: crate::wasm::WasmCheck,
    pub wasm_unchecked: bool,
    pub wasm_panicked: bool,
    pub source_maps: Vec<Option<OwnedSourceMap>>,
    pub source_map_link: Option<crate::maps::MapLink>,
    pub types: Option<crate::types::TypesReport>,
}

pub(crate) fn admits_reparse(text: &str) -> bool {
    input::guard_admits(input::FinalTextGuard::Scan, text)
}

impl Output {
    pub fn code(&self) -> &str {
        self.codes.first().map_or("", String::as_str)
    }

    pub fn libraries(&self) -> Vec<untwist_rename::library::pass::LibrarySummary> {
        let messages = self
            .notes
            .iter()
            .filter(|note| note.pass == untwist_rename::library::pass::NAME)
            .map(|note| note.message.as_str());
        untwist_rename::library::pass::summaries(messages)
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
    limits: Limits,
    shared: Option<&'r untwist_core::semantic::SharedGlobals>,
}

struct Batch<'s> {
    sources: &'s [&'s str],
    rename_state: SharedUniqueState,
    slots: Vec<Option<FileSlot>>,
    stack: StackReserve<'static>,
    stats: std::sync::Mutex<crate::stats::RunStats>,
    limits: Limits,
    shared: Vec<Option<untwist_core::semantic::SharedGlobals>>,
}

impl<'s> Batch<'s> {
    fn new(sources: &'s [&'s str], limits: Limits) -> Self {
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
            limits,
            shared: shared_scan::shared_globals(sources),
        }
    }

    fn reservation(&self, index: usize) -> Reservation<'_> {
        Reservation {
            state: &self.rename_state,
            slot: self.slots.get(index).copied().flatten(),
            stack: &self.stack,
            stats: &self.stats,
            limits: self.limits,
            shared: self.shared.get(index).and_then(Option::as_ref),
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
    unpack_budget: Option<Hit>,
    heap_peak: u64,
    memory_peak: u64,
    budget_hits: Vec<(usize, Hit)>,
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
    source_maps: Vec<Option<OwnedSourceMap>>,
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
            parts.source_maps.push(result.source_map);
            parts.codes.push(result.code);
            parts.stages.extend(result.stages);
            parts.notes.extend(result.notes);
            parts.parse_diagnostics.extend(result.parse_diagnostics);
            parts.parse_errors += result.parse_errors as usize;
            parts.panicked.extend(result.panicked.then_some(index));
            parts.depth_refused.extend(result.refused.then_some(index));
            parts.heap_peak = parts.heap_peak.max(result.heap_peak);
            parts.memory_peak = parts.memory_peak.max(result.memory_peak);
            parts
                .budget_hits
                .extend(result.budget_hit.map(|hit| (index, hit)));
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
            unpack_budget: self.unpack_budget,
            single_file,
            heap_peak: self.heap_peak,
            memory_peak: self.memory_peak,
            budget_hits: self.budget_hits,
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
            wasm_check: crate::wasm::WasmCheck::default(),
            wasm_unchecked: false,
            wasm_panicked: false,
            source_maps: self.source_maps,
            source_map_link: None,
            types: None,
        }
    }
}

fn settle_maps(results: &mut [InputResult], sources: &[&str], inputs: &[Option<String>]) {
    let total = sources.len();
    for (index, (result, source)) in results.iter_mut().zip(sources).enumerate() {
        let map = result
            .source_map
            .take()
            .unwrap_or_else(|| match result.code == *source {
                true => source_map::identity(source),
                false => source_map::unmapped(source),
            });
        let map = source_map::renamed(map, &source_map::input_name(index, total));
        let input = inputs
            .get(index)
            .and_then(Option::as_deref)
            .and_then(source_map::parse);
        result.source_map = Some(match input {
            Some(input) => source_map::compose(map, input),
            None => map,
        });
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
        let pass_disabled = |name: &str| options.disabled_passes.contains(&name);
        let libraries_used = !pass_disabled(untwist_rename::library::pass::NAME)
            || !pass_disabled(untwist_rename::library::rename::NAME);
        if libraries_used {
            untwist_rename::library::table::current();
        }
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
        let batch = Batch::new(sources, self.limits());
        let pooled = self.run_pooled(&batch, &ladder);
        ladder::on_a_worker_stack(&ladder, |_| {
            let mut results = pooled.unwrap_or_else(|| self.run_sequentially(&batch));
            if self.options.source_map {
                settle_maps(&mut results, batch.sources, &self.options.input_source_maps);
            }
            let mut parts = OutputParts::from_results(results);
            parts.unbundled &= trust_the_reflow;
            let _ = crate::stats::RunStats::taken();
            let bundle = self.unpack_budgeted(&mut parts, batch.limits);
            let inputs = *batch
                .stats
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            parts.stats = inputs.merged(crate::stats::RunStats::taken());
            let mut output = Output {
                wasm_check: self.options.wasm_check,
                ..parts.into_output(bundle, self.options.single_file)
            };
            self.types_stage(&mut output);
            output
        })
    }

    fn limits(&self) -> Limits {
        Limits {
            memory: self.options.max_memory,
            deadline: self.options.timeout.map(Deadline::after),
        }
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
