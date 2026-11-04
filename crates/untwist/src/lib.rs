mod bundle;
mod diagnostics;
mod options;
mod passes;
mod pipeline;
mod plugin;
mod report;
mod stats;
mod write;

pub use bundle::{BundleOutput, FileOutput, ModuleOutput};
pub use options::{Isolation, Options, Progress, ProgressSink, TestHooks, TEST_HOOKS};
pub use passes::{pass_named, PassInfo, PASSES};
pub use pipeline::{
    job_width, Output, Pipeline, ADMIT_PANICKED, ANONYMOUS_SHELL_KEPT, ANONYMOUS_SHELL_STRIPPED,
    GUARD_KEPT_ACROSS_LINES, GUARD_ON_ONE_LINE, NESTED_TOO_DEEP, PANICKED, PARENTHESES_TOO_DEEP,
    PIPELINE_NOTES, PRINT_TOO_LARGE, TYPESCRIPT_IN_JAVASCRIPT, UNPACK_PANICKED,
    UNPACK_TEXT_TOO_DEEP, WORKER_DIED,
};
pub use pipeline::{DecodedSite, Snapshot, SnapshotCode};
#[cfg(feature = "refusal-spans")]
pub use pipeline::{RefusalSite, SiteAt};
pub use plugin::{PluginCtor, Plugins, Stage};
pub use report::{stage_rows, StageReport, StageRow};
pub use stats::{CollisionStats, DecodeStats, RunStats};
pub use untwist_core::pass::{IterationTiming, Note, PassCtor, StopReason};
pub use untwist_rename::RenameMode;
pub use untwist_unpack::{Mapping, MappingMatcher};
pub use write::{
    clean, is_safe_relative, stale_paths, PlanRefusal, PlannedFile, WriteReport, BUNDLE_JSON,
    DEOBFUSCATED,
};

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
