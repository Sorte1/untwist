use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::{Collected, PROTO_KEY};
use crate::generate::is_identifier_name;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::usage::collect_usage;

#[derive(Default)]
pub struct Shorthand {
    done: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Shorthand::default())
}

impl<'a> Pass<'a> for Shorthand {
    fn name(&self) -> &'static str {
        "rename-shorthand"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Suppress,
            OuterRule::UsedEnclosing,
            decide,
        );
    }
}

fn decide(program: &Program<'_>, collected: &Collected, scoping: &Scoping) -> RenamePlan {
    let usage = collect_usage(program, scoping);
    let mut oracle = NameOracle::new(scoping, collected);
    let mut moved: FxHashSet<SymbolId> = FxHashSet::default();
    loop {
        let mut changed = false;
        for hint in &usage.hints {
            if collected.pinned.contains(&hint.symbol) || moved.contains(&hint.symbol) {
                continue;
            }
            if !is_identifier_name(&hint.key) || hint.key == PROTO_KEY {
                continue;
            }
            if oracle.current_name(hint.symbol) == hint.key {
                moved.insert(hint.symbol);
                continue;
            }
            if oracle
                .take(hint.symbol, std::iter::once(hint.key.clone()))
                .is_some()
            {
                moved.insert(hint.symbol);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    oracle.into_plan()
}
