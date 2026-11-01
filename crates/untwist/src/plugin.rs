use oxc_allocator::Allocator;
use untwist_core::pass::Pass;

pub type PluginCtor = for<'a> fn(&'a Allocator) -> Box<dyn Pass<'a> + 'a>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stage {
    AfterParse,
    AfterPrepare,
    AfterDeobfuscate,
    AfterUnminify,
    AfterUnpack,
}

impl Stage {
    pub const ALL: [Stage; 5] = [
        Stage::AfterParse,
        Stage::AfterPrepare,
        Stage::AfterDeobfuscate,
        Stage::AfterUnminify,
        Stage::AfterUnpack,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::AfterParse => "after-parse",
            Stage::AfterPrepare => "after-prepare",
            Stage::AfterDeobfuscate => "after-deobfuscate",
            Stage::AfterUnminify => "after-unminify",
            Stage::AfterUnpack => "after-unpack",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Stage::AfterParse => "plugin:after-parse",
            Stage::AfterPrepare => "plugin:after-prepare",
            Stage::AfterDeobfuscate => "plugin:after-deobfuscate",
            Stage::AfterUnminify => "plugin:after-unminify",
            Stage::AfterUnpack => "plugin:after-unpack",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Plugins {
    hooks: Vec<(Stage, PluginCtor)>,
}

impl Plugins {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, stage: Stage, ctor: PluginCtor) -> &mut Self {
        self.hooks.push((stage, ctor));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    pub fn has(&self, stage: Stage) -> bool {
        self.hooks.iter().any(|(at, _)| *at == stage)
    }

    pub fn passes_at<'a>(&self, stage: Stage, alloc: &'a Allocator) -> Vec<Box<dyn Pass<'a> + 'a>> {
        self.hooks
            .iter()
            .filter(|(at, _)| *at == stage)
            .map(|(_, ctor)| ctor(alloc))
            .collect()
    }
}
