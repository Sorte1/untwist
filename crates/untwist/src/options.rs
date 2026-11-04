use std::fmt;
use std::num::NonZeroUsize;
use std::sync::Arc;

use untwist_rename::RenameMode;
use untwist_unpack::Mapping;

use crate::plugin::Plugins;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    InputBegins(usize),
    InputFinished(usize),
    InputsFinished,
    UnpackBegins,
    StageBegins(&'static str),
}

type ProgressFn = dyn Fn(Progress) + Send + Sync;

#[derive(Clone)]
pub struct ProgressSink(Arc<ProgressFn>);

impl ProgressSink {
    pub fn new(sink: impl Fn(Progress) + Send + Sync + 'static) -> Self {
        Self(Arc::new(sink))
    }

    pub fn emit(&self, progress: Progress) {
        (self.0)(progress);
    }
}

impl fmt::Debug for ProgressSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProgressSink")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Isolation {
    pub progress: Option<ProgressSink>,
    pub refused: Vec<usize>,
    pub unpack_lost: bool,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestHooks {
    pub ladder_refuse: usize,
    pub ladder_first: Option<usize>,
    pub scan_off: bool,
    pub heap_budget: Option<u64>,
    pub admit_panic: Option<usize>,
    pub layer_stub: Option<&'static [(&'static str, &'static str)]>,
    pub refusal_spans: bool,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub unminify: bool,
    pub deobfuscate: bool,
    pub unpack: bool,
    pub jsx: bool,
    pub unsafe_transforms: bool,
    #[cfg(feature = "test-hooks")]
    pub extended: bool,
    pub max_iterations: Option<u32>,
    pub mappings: Vec<Mapping>,
    pub split_hoisted: bool,
    pub rename: RenameMode,
    pub rename_all: bool,
    pub plugins: Plugins,
    pub single_file: bool,
    pub jobs: Option<NonZeroUsize>,
    pub fuel: Option<u64>,
    pub isolation: Isolation,
    #[cfg(feature = "test-hooks")]
    pub test_hooks: TestHooks,
    pub disabled_passes: Vec<&'static str>,
    pub refusal_spans: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            unminify: true,
            deobfuscate: true,
            unpack: true,
            jsx: true,
            unsafe_transforms: false,
            #[cfg(feature = "test-hooks")]
            extended: true,
            max_iterations: None,
            mappings: Vec::new(),
            split_hoisted: false,
            rename: RenameMode::None,
            rename_all: true,
            plugins: Plugins::default(),
            single_file: false,
            jobs: None,
            fuel: None,
            isolation: Isolation::default(),
            #[cfg(feature = "test-hooks")]
            test_hooks: TestHooks::default(),
            disabled_passes: Vec::new(),
            refusal_spans: false,
        }
    }
}

#[doc(hidden)]
pub const TEST_HOOKS: bool = cfg!(feature = "test-hooks");

impl Options {
    #[cfg(feature = "test-hooks")]
    pub(crate) fn extended(&self) -> bool {
        self.extended
    }

    #[cfg(not(feature = "test-hooks"))]
    pub(crate) fn extended(&self) -> bool {
        true
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn hooks(&self) -> TestHooks {
        self.test_hooks
    }

    #[cfg(not(feature = "test-hooks"))]
    pub(crate) fn hooks(&self) -> TestHooks {
        TestHooks::default()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn with_test_hooks(self, test_hooks: TestHooks) -> Self {
        Self { test_hooks, ..self }
    }

    #[cfg(not(feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn with_test_hooks(self, _: TestHooks) -> Self {
        self
    }

    pub(crate) fn progress(&self, progress: Progress) {
        if let Some(sink) = &self.isolation.progress {
            sink.emit(progress);
        }
    }
}
