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
mod claims;
mod dead;
mod hooks;
mod mutants;
mod params;
mod references;
mod scopes;

use claims::Claims;

const SEEN: u32 = 1 << 31;
const REPLACED: u32 = 1 << 30;
const LISTED: u32 = 1 << 29;
const PARAMS: u32 = 1 << 28;
const NO_KEY: u32 = PARAMS - 1;
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
    moved: bool,
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
        scope: Option<ScopeId>,
    },
    CatchParam(SymbolFlags),
}

struct Fresh<'a> {
    at: u32,
    name: Ident<'a>,
    symbol: Option<SymbolId>,
    scope: ScopeId,
    flags: ReferenceFlags,
    params: bool,
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
    scopes: Claims,
    symbols: Claims,
    parent_fixes: Vec<(ScopeId, ScopeId)>,
    scope_flag_fixes: Vec<(ScopeId, ScopeFlags)>,
    nodes: u32,
    references: u32,
    failed: bool,
    params_open: u32,
}

struct Walked<'a> {
    slots: Vec<Slot>,
    fresh: Vec<Fresh<'a>>,
    flag_fixes: Vec<(ReferenceId, ReferenceFlags)>,
    parent_fixes: Vec<(ScopeId, ScopeId)>,
    scope_flag_fixes: Vec<(ScopeId, ScopeFlags)>,
    dead: dead::Dead,
    canonical: bool,
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
    super::renamed::forget();
}

pub fn refresh_scoping(
    program: &Program<'_>,
    mut held: Scoping,
    stats: Stats,
    round: Round,
) -> Semantics {
    super::renamed::replay(program, &mut held);
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
    if forced_rebuild()
        || failed_before(round.passes)
        || program.source_type.is_typescript()
        || has_module_syntax(program)
    {
        return None;
    }
    let mut walk = walker(&held);
    walk.visit_program(program);
    let patched = walk.finish().and_then(|walked| apply::apply(held, walked));
    if patched.is_none() && round.passes != 0 {
        remember_failure(round.passes);
    }
    patched
}

fn walker<'s, 'a>(scoping: &'s Scoping) -> Walk<'s, 'a> {
    Walk {
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
        scopes: Claims::new(scoping.scopes_len()),
        symbols: Claims::new(scoping.symbols_len()),
        parent_fixes: Vec::new(),
        scope_flag_fixes: Vec::new(),
        nodes: 0,
        references: 0,
        failed: false,
        params_open: 0,
    }
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

    #[inline]
    fn prepare(&mut self) {
        if !self.prepared {
            self.index_unresolved();
        }
    }

    #[inline(never)]
    fn index_unresolved(&mut self) {
        self.prepared = true;
        (self.slots, self.keys) = unresolved_index(self.scoping);
    }

    fn finish(mut self) -> Option<Walked<'a>> {
        self.prepare();
        let complete = !self.failed
            && self.frames.is_empty()
            && self.keys.len() < NO_KEY as usize
            && self.redeclarations_complete();
        if !complete {
            return None;
        }
        let dead = dead::Dead::of(&self)?;
        let canonical = mutants::trusts_id_order()
            || (dead.is_empty() && self.scopes.in_order() && self.symbols.in_order());
        let stats = Stats::new(
            self.nodes,
            u32::try_from(self.scopes.count()).ok()?,
            u32::try_from(self.symbols.count()).ok()?,
            self.references,
        );
        Some(Walked {
            slots: self.slots,
            fresh: self.fresh,
            flag_fixes: self.flag_fixes,
            parent_fixes: self.parent_fixes,
            scope_flag_fixes: self.scope_flag_fixes,
            dead,
            canonical,
            stats,
        })
    }

    fn redeclarations_complete(&self) -> bool {
        self.redeclared
            .iter()
            .all(|(&symbol, &seen)| self.scoping.symbol_redeclarations(symbol).len() == seen)
    }
}

#[cfg(not(feature = "test-hooks"))]
mod tally {
    use oxc_ast::ast::Program;

    use super::Semantics;

    pub(super) fn patched(_program: &Program<'_>, _patched: &Semantics) {}

    pub(super) fn rebuilt() {}

    pub(super) fn built() {}

    pub(super) fn forced_rebuild() -> bool {
        false
    }
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
        pub built: usize,

        pub checked: usize,
        pub stale: usize,
    }

    static CHECK: AtomicBool = AtomicBool::new(false);
    static FORCE: AtomicBool = AtomicBool::new(false);
    static PATCHED: AtomicUsize = AtomicUsize::new(0);
    static REBUILT: AtomicUsize = AtomicUsize::new(0);
    static BUILT: AtomicUsize = AtomicUsize::new(0);

    static CHECKED: AtomicUsize = AtomicUsize::new(0);
    static STALE: AtomicUsize = AtomicUsize::new(0);
    static LAST: Mutex<String> = Mutex::new(String::new());

    pub fn last_stale_for_tests() -> String {
        LAST.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn check_patches_for_tests(on: bool) {
        CHECK.store(on, Ordering::SeqCst);
    }

    pub fn force_rebuilds_for_tests(on: bool) {
        FORCE.store(on, Ordering::SeqCst);
    }

    pub(super) fn forced_rebuild() -> bool {
        FORCE.load(Ordering::SeqCst)
    }

    pub fn refresh_counts_for_tests() -> RefreshCounts {
        RefreshCounts {
            patched: PATCHED.load(Ordering::SeqCst),
            rebuilt: REBUILT.load(Ordering::SeqCst),
            built: BUILT.load(Ordering::SeqCst),

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

    pub(super) fn built() {
        BUILT.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(feature = "test-hooks")]
pub use apply::skip_one_deletion_for_tests;
#[cfg(feature = "test-hooks")]
pub use dead::mutants::keep_one_dead_binding_for_tests;
#[cfg(feature = "test-hooks")]
pub use mutants::trust_id_order_for_tests;
#[cfg(feature = "test-hooks")]
pub use scopes::mutants::misplace_one_scope_for_tests;
use tally::forced_rebuild;

pub(super) fn count_build() {
    tally::built();
}

#[cfg(feature = "test-hooks")]
pub use tally::{
    check_patches_for_tests, force_rebuilds_for_tests, last_stale_for_tests,
    refresh_counts_for_tests, RefreshCounts, STALE_PATCH,
};
