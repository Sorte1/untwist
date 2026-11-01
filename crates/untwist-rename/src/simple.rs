use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::{BindingInfo, Collected};
use crate::generate::{sequential_name, uid_candidates};
use crate::human::looks_human_written;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::RenameConfig;

pub const SEQUENCE_LIMIT: usize = 4096;

pub const WRAPPER_NAMES: [&str; 4] = ["require", "module", "exports", "define"];

pub struct Simple {
    config: RenameConfig,
    done: bool,
}

impl Simple {
    pub fn new(config: RenameConfig) -> Self {
        Self {
            config,
            done: false,
        }
    }
}

pub fn boxed<'a>(config: RenameConfig) -> Box<dyn Pass<'a> + 'a> {
    Box::new(Simple::new(config))
}

impl<'a> Pass<'a> for Simple {
    fn name(&self) -> &'static str {
        "rename-simple"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let config = self.config;
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Report,
            OuterRule::UsedEnclosing,
            move |_, collected, scoping| decide(collected, scoping, config),
        );
    }
}

pub fn is_renameable(info: &BindingInfo, collected: &Collected, config: RenameConfig) -> bool {
    if collected.pinned.contains(&info.symbol) {
        return false;
    }
    let renames_human_names = config.rename_all && !WRAPPER_NAMES.contains(&info.name.as_str());
    renames_human_names || !looks_human_written(&info.name)
}

pub fn unit_roots(scoping: &Scoping) -> FxHashMap<ScopeId, ScopeId> {
    let mut units: FxHashMap<ScopeId, ScopeId> = FxHashMap::default();
    for scope in scoping.scope_descendants_from_root() {
        resolve_unit(scoping, scope, &mut units);
    }
    units
}

fn resolve_unit(
    scoping: &Scoping,
    scope: ScopeId,
    units: &mut FxHashMap<ScopeId, ScopeId>,
) -> ScopeId {
    let mut pending: Vec<ScopeId> = Vec::new();
    let mut current = scope;
    let unit = loop {
        if let Some(&known) = units.get(&current) {
            break known;
        }
        if is_unit_root(scoping, current) {
            break current;
        }
        pending.push(current);
        let Some(parent) = scoping.scope_parent_id(current) else {
            break current;
        };
        current = parent;
    };
    units.insert(current, unit);
    for visited in pending {
        units.insert(visited, unit);
    }
    unit
}

fn is_unit_root(scoping: &Scoping, scope: ScopeId) -> bool {
    if scoping.scope_parent_id(scope).is_none() {
        return true;
    }
    scoping
        .scope_flags(scope)
        .intersects(ScopeFlags::Function | ScopeFlags::Arrow | ScopeFlags::ClassStaticBlock)
}

fn decide(collected: &Collected, scoping: &Scoping, config: RenameConfig) -> RenamePlan {
    decide_with_limit(collected, scoping, config, SEQUENCE_LIMIT)
}

fn decide_with_limit(
    collected: &Collected,
    scoping: &Scoping,
    config: RenameConfig,
    limit: usize,
) -> RenamePlan {
    let mut oracle = NameOracle::new(scoping, collected);
    let units = unit_roots(scoping);
    let mut counters: FxHashMap<ScopeId, usize> = FxHashMap::default();
    let renameable: Vec<&BindingInfo> = collected
        .bindings
        .iter()
        .filter(|info| is_renameable(info, collected, config))
        .collect();
    oracle.defer(renameable.iter().map(|info| info.symbol));
    for info in renameable {
        let unit = units.get(&info.scope).copied().unwrap_or(info.scope);
        let start = counters.get(&unit).copied().unwrap_or(0);
        let taken = match next_in_sequence(&oracle, info.symbol, start, limit) {
            Some((index, name)) => {
                counters.insert(unit, index + 1);
                oracle.take(info.symbol, std::iter::once(name))
            }
            None => {
                let current = oracle.current_name(info.symbol).to_string();
                oracle.take(info.symbol, uid_candidates(&current))
            }
        };
        if taken.is_none() {
            return aborted(&info.name);
        }
    }
    oracle.into_plan()
}

fn aborted(name: &str) -> RenamePlan {
    let mut plan = RenamePlan::new();
    plan.notes.push(format!(
        "ran out of names for {name}; leaving the program unrenamed"
    ));
    plan
}

fn next_in_sequence(
    oracle: &NameOracle<'_>,
    symbol: SymbolId,
    start: usize,
    limit: usize,
) -> Option<(usize, String)> {
    (start..start.saturating_add(limit))
        .map(|index| (index, sequential_name(index)))
        .find(|(_, name)| oracle.is_available(name, symbol))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect::collect;
    use crate::RenameMode;
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;
    use untwist_core::semantic::build_scoping;

    fn every_fallback_taken() -> String {
        let mut source = String::from("q;");
        for index in 2..1000 {
            source.push_str(&format!("q{index};"));
        }
        source.push_str("function f(q) { return q; }");
        source
    }

    #[test]
    fn exhausting_the_sequence_and_the_fallback_aborts_the_mode() {
        let alloc = Allocator::default();
        let source = every_fallback_taken();
        let parsed = parse_program(&alloc, &source);
        let sem = build_scoping(&parsed.program);
        let collected = collect(&parsed.program, &sem.scoping, OuterRule::UsedEnclosing);
        let config = RenameConfig {
            mode: RenameMode::Simple,
            rename_all: false,
            extended: true,
        };
        let plan = decide_with_limit(&collected, &sem.scoping, config, 0);
        assert!(plan.is_empty(), "the mode must rename nothing");
        assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
        assert!(plan.notes[0].contains('q'), "{:?}", plan.notes);
    }
}
