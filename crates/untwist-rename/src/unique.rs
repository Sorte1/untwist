use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};

use crate::base_name::{base_name, BaseNamer, Context};
use crate::collect::{BindingInfo, Collected};
use crate::generate::uid_candidates;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode_once, HazardNote, RenamePlan};
use crate::simple::is_renameable;
use crate::usage::{collect_naming_usage, Usage};
use crate::RenameConfig;

pub const NUMBER_LIMIT: u32 = 4096;

#[derive(Clone, Copy, Debug)]
pub struct FileSlot {
    index: usize,
}

impl FileSlot {
    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(Default)]
#[cfg_attr(feature = "test-hooks", derive(Clone))]
pub struct UniqueState {
    counters: FxHashMap<String, u32>,
    taken: FxHashSet<String>,
    served: usize,
    turn: usize,
}

impl UniqueState {
    pub fn reserve(&mut self, index: usize) -> Option<FileSlot> {
        if index != self.served {
            return None;
        }
        self.served += 1;
        Some(FileSlot { index })
    }
}

pub struct Turnstile {
    state: Mutex<UniqueState>,
    turned: Condvar,
}

pub type SharedUniqueState = Arc<Turnstile>;

pub fn shared_state() -> SharedUniqueState {
    Arc::new(Turnstile {
        state: Mutex::new(UniqueState::default()),
        turned: Condvar::new(),
    })
}

fn lock(state: &SharedUniqueState) -> MutexGuard<'_, UniqueState> {
    match state.state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn reserve(state: &SharedUniqueState, index: usize) -> Option<FileSlot> {
    lock(state).reserve(index)
}

fn wait_turn(state: &SharedUniqueState, index: usize) -> MutexGuard<'_, UniqueState> {
    let mut guard = lock(state);
    while guard.turn < index {
        guard = match state.turned.wait(guard) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
    guard
}

fn advance_turn(state: &SharedUniqueState, guard: &mut MutexGuard<'_, UniqueState>, index: usize) {
    if guard.turn == index {
        guard.turn = index + 1;
        state.turned.notify_all();
    }
}

pub fn release_turn(state: &SharedUniqueState, index: usize) {
    let mut guard = wait_turn(state, index);
    advance_turn(state, &mut guard, index);
}

pub struct TurnGuard {
    turnstile: SharedUniqueState,
    index: usize,
}

impl TurnGuard {
    pub fn new(turnstile: &SharedUniqueState, index: usize) -> Self {
        Self {
            turnstile: Arc::clone(turnstile),
            index,
        }
    }
}

impl Drop for TurnGuard {
    fn drop(&mut self) {
        release_turn(&self.turnstile, self.index);
    }
}

pub struct Unique {
    config: RenameConfig,
    state: SharedUniqueState,
    slot: FileSlot,
    namer: Option<Box<dyn BaseNamer>>,
    done: bool,
}

impl Unique {
    pub fn new(config: RenameConfig, state: SharedUniqueState, slot: FileSlot) -> Self {
        Self {
            config,
            state,
            slot,
            namer: None,
            done: false,
        }
    }

    pub fn with_namer(mut self, namer: Box<dyn BaseNamer>) -> Self {
        self.namer = Some(namer);
        self
    }
}

pub fn boxed<'a>(
    config: RenameConfig,
    state: SharedUniqueState,
    slot: FileSlot,
) -> Box<dyn Pass<'a> + 'a> {
    Box::new(Unique::new(config, state, slot))
}

impl<'a> Pass<'a> for Unique {
    fn name(&self) -> &'static str {
        "rename-unique"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let Self {
            config,
            state,
            slot,
            namer,
            done,
        } = self;
        let config = *config;
        let shared = Arc::clone(state);
        let index = slot.index();
        let namer = namer.as_deref();
        #[cfg(feature = "test-hooks")]
        let keep = cx.opts.keep_script_globals;
        run_mode_once(
            done,
            program,
            cx,
            HazardNote::Report,
            OuterRule::UsedEnclosing,
            move |program, collected, scoping| {
                let usage = collect_naming_usage(program, scoping);
                let mut state = wait_turn(&shared, index);
                #[cfg(feature = "test-hooks")]
                let before = state.clone();
                let plan = decide(collected, &usage, scoping, config, namer, &mut state);
                #[cfg(feature = "test-hooks")]
                crate::fresh::assert_same_unique_plan(
                    program,
                    (&plan, keep),
                    |collected, _, fresh| {
                        let usage = collect_naming_usage(program, fresh);
                        let mut replayed = before;
                        decide(collected, &usage, fresh, config, namer, &mut replayed)
                    },
                );
                advance_turn(&shared, &mut state, index);
                plan
            },
        );
    }
}

fn decide(
    collected: &Collected,
    usage: &Usage,
    scoping: &Scoping,
    config: RenameConfig,
    namer: Option<&dyn BaseNamer>,
    state: &mut UniqueState,
) -> RenamePlan {
    decide_with_limit(
        collected,
        usage,
        scoping,
        config,
        namer,
        state,
        NUMBER_LIMIT,
    )
}

fn decide_with_limit(
    collected: &Collected,
    usage: &Usage,
    scoping: &Scoping,
    config: RenameConfig,
    namer: Option<&dyn BaseNamer>,
    state: &mut UniqueState,
    limit: u32,
) -> RenamePlan {
    let mut oracle = NameOracle::new(scoping, collected);
    let cx = Context {
        collected,
        usage,
        namer,
        config,
    };
    let renameable: Vec<&BindingInfo> = collected
        .bindings
        .iter()
        .filter(|info| is_renameable(info, collected, config))
        .collect();
    oracle.defer(renameable.iter().map(|info| info.symbol));
    for info in renameable {
        let base = base_name(info, &cx);
        if assign(&mut oracle, state, info, &base, limit).is_none() {
            return aborted(&info.name);
        }
    }
    oracle.into_plan()
}

fn assign(
    oracle: &mut NameOracle<'_>,
    state: &mut UniqueState,
    info: &BindingInfo,
    base: &str,
    limit: u32,
) -> Option<String> {
    let taken = match pick(base, info.symbol, oracle, state, limit) {
        Some((name, number)) => {
            state
                .counters
                .insert(base.to_string(), number.saturating_add(1));
            oracle.take(info.symbol, std::iter::once(name))
        }
        None => {
            let current = oracle.current_name(info.symbol).to_string();
            let already_taken = &state.taken;
            oracle.take(
                info.symbol,
                uid_candidates(&current).filter(|name| !already_taken.contains(name)),
            )
        }
    };
    let name = taken?;
    state.taken.insert(name.clone());
    Some(name)
}

fn pick(
    base: &str,
    symbol: SymbolId,
    oracle: &NameOracle<'_>,
    state: &UniqueState,
    limit: u32,
) -> Option<(String, u32)> {
    let start = state.counters.get(base).copied().unwrap_or(1);
    for number in start..start.saturating_add(limit) {
        let candidate = format!("{base}{number}");
        if state.taken.contains(&candidate) {
            continue;
        }
        if oracle.is_available(&candidate, symbol) {
            return Some((candidate, number));
        }
    }
    None
}

fn aborted(name: &str) -> RenamePlan {
    let mut plan = RenamePlan::new();
    plan.notes.push(format!(
        "ran out of names for {name}; leaving the program unrenamed"
    ));
    plan
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
    fn exhausting_the_numbers_and_the_fallback_aborts_the_mode() {
        let alloc = Allocator::default();
        let source = every_fallback_taken();
        let parsed = parse_program(&alloc, &source);
        let sem = build_scoping(&parsed.program);
        let collected = collect(&parsed.program, &sem.scoping, OuterRule::UsedEnclosing);
        let usage = collect_naming_usage(&parsed.program, &sem.scoping);
        let config = RenameConfig {
            mode: RenameMode::Unique,
            rename_all: false,
            extended: true,
        };
        let mut state = UniqueState::default();
        let plan = decide_with_limit(
            &collected,
            &usage,
            &sem.scoping,
            config,
            None,
            &mut state,
            0,
        );
        assert!(plan.is_empty(), "the mode must rename nothing");
        assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
        assert!(plan.notes[0].contains("for q;"), "{:?}", plan.notes);
    }

    #[test]
    fn the_uid_fallback_never_hands_one_name_to_two_scopes() {
        let alloc = Allocator::default();
        let source = "function _0xa() { var q = 1; return q; } \
                      function _0xb() { var q = 2; return q; }";
        let parsed = parse_program(&alloc, source);
        let sem = build_scoping(&parsed.program);
        let collected = collect(&parsed.program, &sem.scoping, OuterRule::UsedEnclosing);
        let usage = collect_naming_usage(&parsed.program, &sem.scoping);
        let config = RenameConfig {
            mode: RenameMode::Unique,
            rename_all: true,
            extended: true,
        };
        let mut state = UniqueState::default();
        let plan = decide_with_limit(
            &collected,
            &usage,
            &sem.scoping,
            config,
            None,
            &mut state,
            0,
        );
        assert!(plan.notes.is_empty(), "{:?}", plan.notes);
        assert_eq!(plan.len(), 4, "every binding is renamed");
        assert_eq!(
            state.taken.len(),
            4,
            "Unique promises program-wide distinct names, fallback included: {:?}",
            state.taken
        );
    }
}
