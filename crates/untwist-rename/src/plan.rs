use oxc_allocator::GetAllocator;
use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::rename::Renames;
use untwist_core::machinery_name::is_machinery_name;
use untwist_core::pass::Cx;
use untwist_core::semantic::build_scoping;

use crate::collect::{collect, Collected};
use crate::oracle::OuterRule;

const HAZARD_SUFFIX: &str = "may defeat scope analysis; renaming anyway";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RenamePlan {
    names: Vec<(SymbolId, String)>,
    pub notes: Vec<String>,
}

impl RenamePlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, symbol: SymbolId, name: String) {
        self.names.push((symbol, name));
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn apply<'a>(
        &self,
        program: &mut Program<'a>,
        scoping: &Scoping,
        alloc: &impl GetAllocator<'a>,
    ) -> usize {
        let mut renames = Renames::new();
        let mut changed = 0usize;
        for (symbol, name) in &self.names {
            let current = scoping.symbol_name(*symbol);
            if current == name || is_machinery_name(current) {
                continue;
            }
            renames.insert(*symbol, Ident::from_str_in(name, alloc));
            changed += 1;
        }
        if changed == 0 {
            return 0;
        }
        renames.apply(program, scoping);
        changed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HazardNote {
    Report,
    Suppress,
}

pub fn run_mode<'a, F>(
    done: &mut bool,
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    hazard_note: HazardNote,
    rule: OuterRule,
    decide: F,
) where
    F: Fn(&Program<'a>, &Collected, &Scoping) -> RenamePlan,
{
    #[cfg(feature = "test-hooks")]
    let keep = cx.opts.keep_script_globals;
    run_mode_once(
        done,
        program,
        cx,
        hazard_note,
        rule,
        |program, collected, scoping| {
            let plan = decide(program, collected, scoping);
            #[cfg(feature = "test-hooks")]
            crate::fresh::assert_same_plan(program, (rule, keep), &plan, &decide);
            plan
        },
    );
}

pub fn run_mode_once<'a, F>(
    done: &mut bool,
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    hazard_note: HazardNote,
    rule: OuterRule,
    decide: F,
) where
    F: FnOnce(&Program<'a>, &Collected, &Scoping) -> RenamePlan,
{
    if *done {
        return;
    }
    *done = true;
    let rebuilt = (!cx.is_program_unchanged_this_traversal()).then(|| build_scoping(program));
    if rebuilt.is_none() {
        cx.compact_patched_scoping(program);
    }
    let scoping = rebuilt.as_ref().map_or(cx.scoping(), |sem| &sem.scoping);
    #[cfg(feature = "test-hooks")]
    if rebuilt.is_none() {
        crate::fresh::assert_fresh(program, scoping);
    }
    let mut collected = collect(program, scoping, rule);
    crate::collect::pin_script_globals(&mut collected, scoping, cx.opts.keep_script_globals);
    let hazard = match hazard_note {
        HazardNote::Report => collected.hazard,
        HazardNote::Suppress => None,
    };
    let mut plan = decide(program, &collected, scoping);
    let renamed = plan.apply(program, scoping, &*cx);
    drop(rebuilt);
    if let Some(hazard) = hazard {
        cx.note(format!("{hazard} {HAZARD_SUFFIX}"), None);
    }
    for note in plan.notes.drain(..) {
        cx.note(note, None);
    }
    cx.changed_by(renamed);
}
