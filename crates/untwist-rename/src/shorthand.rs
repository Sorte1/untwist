use std::collections::BTreeSet;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::{Collected, PROTO_KEY};
use crate::generate::is_identifier_name;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::usage::{collect_usage, ShorthandHint};

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
    let mut wanting: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    for (index, hint) in usage.hints.iter().enumerate() {
        wanting.entry(hint.key.as_str()).or_default().push(index);
    }
    let mut round: BTreeSet<usize> = (0..usage.hints.len()).collect();
    while !round.is_empty() {
        let mut next: BTreeSet<usize> = BTreeSet::new();
        while let Some(index) = round.pop_first() {
            let hint = &usage.hints[index];
            if !is_open(hint, collected, scoping, &moved) {
                continue;
            }
            let previous = oracle.current_name(hint.symbol).to_string();
            if previous == hint.key {
                moved.insert(hint.symbol);
                continue;
            }
            if oracle
                .take(hint.symbol, std::iter::once(hint.key.clone()))
                .is_none()
            {
                continue;
            }
            moved.insert(hint.symbol);
            for &waiting in wanting.get(previous.as_str()).into_iter().flatten() {
                if waiting > index {
                    round.insert(waiting);
                } else {
                    next.insert(waiting);
                }
            }
        }
        round = next;
    }
    oracle.into_plan()
}

fn is_open(
    hint: &ShorthandHint,
    collected: &Collected,
    scoping: &Scoping,
    moved: &FxHashSet<SymbolId>,
) -> bool {
    !collected.pinned.contains(&hint.symbol)
        && !moved.contains(&hint.symbol)
        && !makes_a_global_property(collected, scoping, hint.symbol)
        && is_identifier_name(&hint.key)
        && hint.key != PROTO_KEY
}

fn makes_a_global_property(collected: &Collected, scoping: &Scoping, symbol: SymbolId) -> bool {
    let declared = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
    collected.classic_script
        && scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
        && scoping.symbol_flags(symbol).intersects(declared)
}
