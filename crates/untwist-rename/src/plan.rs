use oxc_allocator::GetAllocator;
use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::rename::Renames;
use untwist_core::machinery_name::is_machinery_name;
use untwist_core::pass::Cx;
use untwist_core::semantic::build_scoping;

use crate::collect::Collected;
use crate::oracle::OuterRule;
use crate::pins::{prepare, Setting};

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
            debug_assert!(!is_machinery_name(current), "planned a rename of {current}");
            if current == name {
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

fn observed_note(hazard: &str, collected: &Collected) -> String {
    if collected.assumed_unobserved {
        return format!("{hazard} may defeat scope analysis; --unsafe assumes it sees no binding");
    }
    let kept = match collected.observed.len() {
        0 => "it can see no binding to rename".to_string(),
        1 => "the 1 binding it can see keeps its name".to_string(),
        count => format!("the {count} bindings it can see keep their names"),
    };
    format!("{hazard} may defeat scope analysis; {kept}")
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
    let setting = crate::pins::Setting::of(program, cx.opts);
    run_mode_once(
        done,
        program,
        cx,
        hazard_note,
        rule,
        |program, collected, scoping| {
            let plan = decide(program, collected, scoping);
            #[cfg(feature = "test-hooks")]
            crate::fresh::assert_same_plan(program, (rule, setting), &plan, &decide);
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
    let collected = prepare(program, scoping, rule, Setting::of(program, cx.opts));
    let hazard = match hazard_note {
        HazardNote::Report => collected.hazard,
        HazardNote::Suppress => None,
    };
    let mut plan = decide(program, &collected, scoping);
    let renamed = plan.apply(program, scoping, &*cx);
    drop(rebuilt);
    if let Some(hazard) = hazard {
        cx.note(observed_note(hazard, &collected), None);
    }
    for note in plan.notes.drain(..) {
        cx.note(note, None);
    }
    cx.changed_by(renamed);
}
