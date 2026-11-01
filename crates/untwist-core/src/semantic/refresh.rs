use std::cell::Cell;
use std::hash::{Hash, Hasher};

use oxc_ast::ast::Program;
use oxc_ast_visit::VisitJs;
use oxc_semantic::{Scoping, Stats};
use oxc_str::Ident;
use oxc_syntax::reference::{ReferenceFlags, ReferenceId};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;

use super::{has_module_syntax, rebuild_scoping, Semantics};

mod apply;
mod bindings;
mod hooks;
mod params;
mod references;
mod scopes;

const SEEN: u32 = 1 << 31;
const REPLACED: u32 = 1 << 30;
const LISTED: u32 = 1 << 29;
const NO_KEY: u32 = LISTED - 1;
const FRAMES: usize = 64;
const GROWTH_SHARE: u32 = 8;

#[derive(Clone, Copy)]
struct Slot {
    state: u32,
    at: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Plain,
    Function,
    Catch,
}

struct Frame {
    scope: ScopeId,
    expected: ScopeFlags,
    direct_eval: bool,
    region: Region,
    in_params: bool,
    names: u64,
    early: usize,
    outer: Option<usize>,
}

struct Packed(u64);

impl Hasher for Packed {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0 = bytes
            .iter()
            .fold(self.0, |acc, &byte| acc.rotate_left(8) ^ u64::from(byte));
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

fn name_bit(name: Ident<'_>) -> u64 {
    let mut packed = Packed(0);
    name.hash(&mut packed);
    1 << ((packed.0 >> 32) & 63)
}

#[derive(Clone, Copy)]
enum Context {
    Var,
    Lexical(SymbolFlags),
    Param,
    FunctionName {
        declaration: bool,
        includes: SymbolFlags,
    },
    ClassName {
        declaration: bool,
    },
    CatchParam(SymbolFlags),
}

struct Fresh<'a> {
    at: u32,
    name: Ident<'a>,
    symbol: Option<SymbolId>,
    scope: ScopeId,
    flags: ReferenceFlags,
}

struct Walk<'s, 'a> {
    scoping: &'s Scoping,
    frames: Vec<Frame>,
    innermost: Option<usize>,
    frame_of: Vec<u32>,
    early: Vec<SymbolId>,
    redeclared: FxHashMap<SymbolId, usize>,
    slots: Vec<Slot>,
    keys: Vec<Ident<'s>>,
    prepared: bool,
    fresh: Vec<Fresh<'a>>,
    flag_fixes: Vec<(ReferenceId, ReferenceFlags)>,
    context: Option<Context>,
    flags: ReferenceFlags,
    next_scope: usize,
    next_symbol: usize,
    nodes: u32,
    references: u32,
    failed: bool,
}

struct Walked<'a> {
    slots: Vec<Slot>,
    fresh: Vec<Fresh<'a>>,
    flag_fixes: Vec<(ReferenceId, ReferenceFlags)>,
    stats: Stats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    scopes: usize,
    symbols: usize,
    references: usize,
}

impl Shape {
    pub fn of(scoping: &Scoping) -> Self {
        Self {
            scopes: scoping.scopes_len(),
            symbols: scoping.symbols_len(),
            references: scoping.references_len(),
        }
    }

    fn bound_as(self, before: Self) -> bool {
        (self.scopes, self.symbols) == (before.scopes, before.symbols)
    }
}

fn presized(stats: Stats, before: Shape, held: Shape) -> Stats {
    let grown = |count: u32, now: usize, then: usize| {
        let added = u32::try_from(now.saturating_sub(then)).unwrap_or(0);
        let large = added > 0 && added >= count / GROWTH_SHARE;
        if large {
            count.saturating_add(added)
        } else {
            count
        }
    };
    Stats::new(
        stats.nodes,
        grown(stats.scopes, held.scopes, before.scopes),
        grown(stats.symbols, held.symbols, before.symbols),
        grown(stats.references, held.references, before.references),
    )
}

#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub before: Shape,
    pub passes: u64,
}

const REMEMBERED: usize = 16;

thread_local! {
    static FAILED: Cell<([u64; REMEMBERED], usize)> = const { Cell::new(([0; REMEMBERED], 0)) };
}

fn failed_before(passes: u64) -> bool {
    let (failed, _) = FAILED.with(Cell::get);
    failed
        .iter()
        .any(|&earlier| earlier != 0 && earlier & !passes == 0)
}

fn remember_failure(passes: u64) {
    FAILED.with(|cell| {
        let (mut failed, next) = cell.get();
        failed[next % REMEMBERED] = passes;
        cell.set((failed, next.wrapping_add(1)));
    });
}

pub fn forget_failures() {
    FAILED.with(|cell| cell.set(([0; REMEMBERED], 0)));
}

pub fn refresh_scoping(
    program: &Program<'_>,
    held: Scoping,
    stats: Stats,
    round: Round,
) -> Semantics {
    let presize = presized(stats, round.before, Shape::of(&held));
    match patch(program, held, round) {
        Some(mut patched) => {
            super::observers::settle_eval_scopes(program, &mut patched.scoping);
            tally::patched(program, &patched);
            patched
        }
        None => {
            tally::rebuilt();
            rebuild_scoping(program, presize)
        }
    }
}

fn patch(program: &Program<'_>, held: Scoping, round: Round) -> Option<Semantics> {
    if !Shape::of(&held).bound_as(round.before)
        || failed_before(round.passes)
        || program.source_type.is_typescript()
        || has_module_syntax(program)
    {
        return None;
    }
    let patched = walk(program, &held).and_then(|walked| apply::apply(held, walked));
    if patched.is_none() && round.passes != 0 {
        remember_failure(round.passes);
    }
    patched
}

fn walk<'a>(program: &Program<'a>, scoping: &Scoping) -> Option<Walked<'a>> {
    let mut walk = Walk {
        scoping,
        frames: Vec::new(),
        innermost: None,
        frame_of: vec![0; scoping.scopes_len()],
        early: Vec::new(),
        redeclared: FxHashMap::default(),
        slots: Vec::new(),
        keys: Vec::new(),
        prepared: false,
        fresh: Vec::new(),
        flag_fixes: Vec::new(),
        context: None,
        flags: ReferenceFlags::empty(),
        next_scope: 0,
        next_symbol: 0,
        nodes: 0,
        references: 0,
        failed: false,
    };
    walk.visit_program(program);
    walk.finish()
}

fn unresolved_index(scoping: &Scoping) -> (Vec<Slot>, Vec<Ident<'_>>) {
    let mut slots = vec![
        Slot {
            state: NO_KEY,
            at: u32::MAX
        };
        scoping.references_len()
    ];
    let mut keys = Vec::with_capacity(scoping.root_unresolved_references().len());
    for (key, ids) in scoping.root_unresolved_references() {
        let Ok(index) = u32::try_from(keys.len()) else {
            break;
        };
        for id in ids {
            if let Some(slot) = slots.get_mut(id.index()) {
                slot.state = index.min(NO_KEY);
            }
        }
        keys.push(*key);
    }
    (slots, keys)
}

impl<'a> Walk<'_, 'a> {
    fn fail(&mut self) {
        self.failed = true;
    }

    fn prepare(&mut self) {
        if !self.prepared {
            self.prepared = true;
            (self.slots, self.keys) = unresolved_index(self.scoping);
        }
    }

    fn finish(mut self) -> Option<Walked<'a>> {
        self.prepare();
        let complete = !self.failed
            && self.frames.is_empty()
            && self.next_scope == self.scoping.scopes_len()
            && self.next_symbol == self.scoping.symbols_len()
            && self.keys.len() < NO_KEY as usize
            && self.redeclarations_complete()
            && self.bindings_complete();
        if !complete {
            return None;
        }
        let stats = Stats::new(
            self.nodes,
            u32::try_from(self.scoping.scopes_len()).ok()?,
            u32::try_from(self.scoping.symbols_len()).ok()?,
            self.references,
        );
        Some(Walked {
            slots: self.slots,
            fresh: self.fresh,
            flag_fixes: self.flag_fixes,
            stats,
        })
    }

    fn redeclarations_complete(&self) -> bool {
        self.redeclared
            .iter()
            .all(|(&symbol, &seen)| self.scoping.symbol_redeclarations(symbol).len() == seen)
    }

    fn bindings_complete(&self) -> bool {
        let bound: usize = self
            .scoping
            .iter_bindings()
            .map(|(_, bindings)| bindings.len())
            .sum();
        bound == self.scoping.symbols_len()
    }
}

#[cfg(not(feature = "test-hooks"))]
mod tally {
    use oxc_ast::ast::Program;

    use super::Semantics;

    pub(super) fn patched(_program: &Program<'_>, _patched: &Semantics) {}

    pub(super) fn rebuilt() {}
}

#[cfg(feature = "test-hooks")]
mod tally {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Mutex, PoisonError};

    use oxc_ast::ast::Program;

    use super::Semantics;

    pub const STALE_PATCH: &str = "a patched scoping differs from a fresh build";

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct RefreshCounts {
        pub patched: usize,
        pub rebuilt: usize,
        pub checked: usize,
        pub stale: usize,
    }

    static CHECK: AtomicBool = AtomicBool::new(false);
    static PATCHED: AtomicUsize = AtomicUsize::new(0);
    static REBUILT: AtomicUsize = AtomicUsize::new(0);
    static CHECKED: AtomicUsize = AtomicUsize::new(0);
    static STALE: AtomicUsize = AtomicUsize::new(0);
    static LAST: Mutex<String> = Mutex::new(String::new());

    pub fn last_stale_for_tests() -> String {
        LAST.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn check_patches_for_tests(on: bool) {
        CHECK.store(on, Ordering::SeqCst);
    }

    pub fn refresh_counts_for_tests() -> RefreshCounts {
        RefreshCounts {
            patched: PATCHED.load(Ordering::SeqCst),
            rebuilt: REBUILT.load(Ordering::SeqCst),
            checked: CHECKED.load(Ordering::SeqCst),
            stale: STALE.load(Ordering::SeqCst),
        }
    }

    pub(super) fn patched(program: &Program<'_>, patched: &Semantics) {
        PATCHED.fetch_add(1, Ordering::SeqCst);
        if !CHECK.load(Ordering::SeqCst) {
            return;
        }
        if let Some(difference) = crate::semantic::compare::patch_difference(program, patched) {
            STALE.fetch_add(1, Ordering::SeqCst);
            *LAST.lock().unwrap_or_else(PoisonError::into_inner) = difference.clone();
            panic!("{STALE_PATCH}: {difference}");
        }
        CHECKED.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn rebuilt() {
        REBUILT.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(feature = "test-hooks")]
pub use apply::skip_one_deletion_for_tests;
#[cfg(feature = "test-hooks")]
pub use tally::{
    check_patches_for_tests, last_stale_for_tests, refresh_counts_for_tests, RefreshCounts,
    STALE_PATCH,
};
