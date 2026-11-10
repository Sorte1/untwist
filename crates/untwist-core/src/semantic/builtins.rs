use oxc_ast::ast::Program;

use crate::pass::Cx;
use crate::semantic::has_module_syntax;

mod scan;

pub use scan::{intact, scan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtins {
    Unknown,
    Intact,
    Aliased,
    Written,
}

impl Builtins {
    pub fn allows_safe(self, unsafe_transforms: bool) -> bool {
        self == Builtins::Intact || (unsafe_transforms && self == Builtins::Aliased)
    }

    pub fn allows_unsafe(self) -> bool {
        matches!(self, Builtins::Intact | Builtins::Aliased)
    }
}

pub type Members = (&'static str, bool, &'static [&'static str]);

#[derive(Clone, Copy, Debug)]
pub struct Reliance {
    pub roots: &'static [&'static str],
    pub members: &'static [Members],
}

impl Reliance {
    pub fn covers(&self, root: &str, on_prototype: bool, key: Option<&str>) -> bool {
        let Some(key) = key else {
            return true;
        };
        match self
            .members
            .iter()
            .find(|(owner, prototype, _)| *owner == root && *prototype == on_prototype)
        {
            Some((_, _, keys)) => keys.contains(&key),
            None => true,
        }
    }
}

pub struct Scanned {
    pub verdict: Builtins,
    pub promise_written: bool,
}

pub struct Unpatched {
    reliance: &'static Reliance,
    verdict: Builtins,
    asked: bool,
}

impl Unpatched {
    pub const fn new(reliance: &'static Reliance) -> Self {
        Self {
            reliance,
            verdict: Builtins::Unknown,
            asked: false,
        }
    }

    pub fn learn(&mut self, program: &Program<'_>, cx: &Cx<'_, '_>) {
        if !self.asked || self.verdict != Builtins::Unknown {
            return;
        }
        let module = has_module_syntax(program);
        self.verdict = scan(program, cx.scoping(), module, false, self.reliance).verdict;
        if self.verdict == Builtins::Intact {
            cx.walk_whole_round();
        }
    }

    pub fn holds(&mut self, cx: &mut Cx<'_, '_>) -> bool {
        if cx.opts.unsafe_transforms {
            return true;
        }
        if self.verdict == Builtins::Unknown && !self.asked {
            self.asked = true;
            cx.request_round();
        }
        self.verdict == Builtins::Intact
    }
}
