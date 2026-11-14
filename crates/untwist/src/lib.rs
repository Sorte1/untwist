mod bundle;
mod diagnostics;
mod maps;
mod options;
mod passes;
mod pipeline;
mod plan;
mod plugin;
mod printable;
mod report;
mod split_hint;
mod stats;
mod wasm;
mod wasm_graph;
mod write;

pub use bundle::{BundleListing, BundleOutput, FileOutput, ModuleOutput};
pub use maps::{MapLink, MAP_SUFFIX};
pub use options::{Isolation, Options, Progress, ProgressSink, TestHooks, TEST_HOOKS};
pub use passes::{pass_named, PassInfo, PASSES};
pub use pipeline::{
    contained, job_width, Output, Pipeline, ADMIT_PANICKED, ANONYMOUS_SHELL_KEPT,
    ANONYMOUS_SHELL_SCOPED, ANONYMOUS_SHELL_STRIPPED, BUDGET_MEMORY_REACHED, BUDGET_TIME_REACHED,
    BUDGET_UNPACK, GUARD_KEPT_ACROSS_LINES, GUARD_ON_ONE_LINE, NESTED_TOO_DEEP,
    OVER_MEMORY_REFUSED, OVER_TIME_REFUSED, PANICKED, PARENTHESES_TOO_DEEP, PIPELINE_NOTES,
    PRINT_TOO_LARGE, SOURCE_READER_UNGUARDED, SOURCE_READER_UNGUARDED_MODULE,
    TOO_LARGE_FOR_THE_BUDGET, TYPESCRIPT_IN_JAVASCRIPT, UNPACK_PANICKED, UNPACK_TEXT_TOO_DEEP,
    UNSAFE_LANE_DECODES, WORKER_DIED,
};
pub use pipeline::{DecodedSite, Snapshot, SnapshotCode};
#[cfg(feature = "refusal-spans")]
pub use pipeline::{RefusalSite, SiteAt};
pub use plan::{input_names, is_safe_relative, PlanRefusal, PlannedFile};
pub use plugin::{PluginCtor, Plugins, Stage};
pub use printable::printable;
pub use report::{stage_rows, StageReport, StageRow};
pub use split_hint::SplitHint;
pub use stats::{CollisionStats, DecodeStats, RunStats};
pub use untwist_core::budget::{
    memory_size as budget_memory_size, set_hard_limit as set_hard_memory_limit,
    timeout_secs as budget_timeout_secs, working_total as budget_working_total, CountingAlloc,
    Hit as BudgetHit,
};
pub use untwist_core::pass::{IterationTiming, Note, NoteKind, PassCtor, StopReason};
pub use untwist_core::source_map::{self, Coverage, OwnedSourceMap};
#[cfg(feature = "modern")]
pub use untwist_rename::modern::{
    parse_baseline as parse_modern_baseline, Baseline as ModernBaseline,
};
#[cfg(feature = "modern")]
pub mod modern_report {
    pub use untwist_rename::modern::pass::NAME;
    pub use untwist_rename::modern::report::{detail, is_detail, Detail, Status};
}
pub use untwist_rename::RenameMode;

pub mod libraries {
    pub use untwist_rename::library::table::{load, loaded_ids, LibraryTables};
}
pub use untwist_unpack::{Mapping, MappingMatcher};
pub use wasm::{
    SuppliedWasm, WasmCheck, WasmFile, WasmOrigin, WasmReference, WasmSupply, MAX_WASM_BYTES,
    WASM_DIR, WASM_PANICKED,
};
pub use write::{clean, stale_paths, OutputConflict, WriteReport, BUNDLE_JSON, DEOBFUSCATED};

pub fn all_pass_ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = untwist_unminify::prepare_ctors();
    ctors.extend(untwist_unminify::transpile_ctors());
    ctors.extend(untwist_unminify::unminify_ctors());
    ctors.extend(untwist_unminify::lexical_ctors());
    ctors.extend(untwist_deobfuscate::deobfuscate_ctors());
    ctors.extend(untwist_deobfuscate::cleanup_fixpoint_ctors());
    ctors.extend(untwist_deobfuscate::protections_ctors());
    ctors.extend(untwist_deobfuscate::cleanup_ctors());
    ctors.extend(untwist_jsx::jsx_ctors());
    ctors
}
