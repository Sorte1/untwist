use std::time::Duration;

use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_traverse::{traverse_mut, Traverse, TraverseCtx};

use crate::clock::Instant;

use super::interest::{
    ExpressionKind, ExpressionVariants, Interest, StatementKind, StatementVariants,
    EXPRESSION_KINDS, HOOK_COUNT, STATEMENT_KINDS,
};
use super::{Cx, Locality, Note, Pass, PassCtor, PassOptions, Safety};

type ProgramHook<'a> = fn(&mut (dyn Pass<'a> + 'a), &mut Program<'a>, &mut Cx<'a, '_>);

type HookLists = [Vec<usize>; HOOK_COUNT];

const MASK_WIDTH: usize = u64::BITS as usize;

const RETIRABLE: u16 = !Interest::ENTER_PROGRAM.hook_bits();

mod nested;
#[cfg(feature = "test-hooks")]
mod probe;
mod scope;
#[cfg(feature = "test-hooks")]
mod span_check;
mod walk;

use scope::Scope;
pub(super) use scope::Scratch;

#[cfg(feature = "test-hooks")]
pub use scope::mutants::{skip_innermost_list_for_tests, treat_as_neutral_for_tests};

#[cfg(feature = "test-hooks")]
pub use nested::mutants::{cascade_off_for_tests, cascade_root_lists_for_tests};

#[cfg(feature = "test-hooks")]
pub use probe::{probe_variants_for_tests, take_variant_violations_for_tests};

#[cfg(feature = "test-hooks")]
pub use span_check::{check_shared_spans_for_tests, take_shared_span_violations_for_tests};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    ByVariant,
    List,
    #[cfg(feature = "test-hooks")]
    Probe,
}

struct Tables {
    enter_expression: [u64; EXPRESSION_KINDS],
    exit_expression: [u64; EXPRESSION_KINDS],
    enter_statement: [u64; STATEMENT_KINDS],
    exit_statement: [u64; STATEMENT_KINDS],
}

pub struct PassGroup<'a> {
    passes: Vec<Box<dyn Pass<'a> + 'a>>,
    names: Vec<&'static str>,
    counts: Vec<u32>,
    elapsed: Vec<Duration>,
    hooks: HookLists,
    walks: bool,
    parked: Option<AVec<'a, Statement<'a>>>,
    mode: Mode,
    tables: Tables,
    opts: PassOptions,
    notes: Vec<Note>,
    scratch: Scratch,
    declared: bool,
    retiring: bool,
    scope: Scope<'a>,
    cascades: bool,
    nesting: u8,
    bodies: Vec<AVec<'a, Statement<'a>>>,
    spare_scoping: Option<Scoping>,
}

fn filtered<'a>(
    passes: Vec<Box<dyn Pass<'a> + 'a>>,
    opts: &PassOptions,
) -> Vec<Box<dyn Pass<'a> + 'a>> {
    passes
        .into_iter()
        .filter(|pass| opts.unsafe_transforms || pass.safety() != Safety::Unsafe)
        .filter(|pass| opts.extended || !pass.requires_extended())
        .filter(|pass| !opts.rule_disabled(pass.name()))
        .collect()
}

fn hook_lists(interests: &[u16]) -> HookLists {
    std::array::from_fn(|slot| {
        let bit = Interest::bit_at(slot).hook_bits();
        let wants = |hooks: u16| hooks & bit == bit;
        let mut list = Vec::with_capacity(interests.iter().filter(|&&h| wants(h)).count());
        list.extend(
            interests
                .iter()
                .enumerate()
                .filter(|&(_, &hooks)| wants(hooks))
                .map(|(index, _)| index),
        );
        list
    })
}

fn retired_bits(pass: &dyn Pass<'_>) -> u16 {
    pass.retired().hook_bits() & RETIRABLE
}

fn is_exhaustive(pass: &dyn Pass<'_>) -> bool {
    let interest = pass.interest();
    interest.expressions() == ExpressionVariants::ALL
        && interest.statements() == StatementVariants::ALL
}

fn mode_for(passes: &[Box<dyn Pass<'_> + '_>], declared: bool) -> Mode {
    if !declared {
        return Mode::List;
    }
    #[cfg(feature = "test-hooks")]
    if probe::probing() {
        return Mode::Probe;
    }
    let uniform = passes.iter().all(|pass| is_exhaustive(pass.as_ref()));
    if uniform || passes.len() > MASK_WIDTH {
        return Mode::List;
    }
    Mode::ByVariant
}

#[cfg(feature = "test-hooks")]
static WALK_EVERY_GROUP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn walk_every_group_for_tests(on: bool) {
    WALK_EVERY_GROUP.store(on, std::sync::atomic::Ordering::SeqCst);
}

fn walks_nodes(hooks: &HookLists) -> bool {
    #[cfg(feature = "test-hooks")]
    if WALK_EVERY_GROUP.load(std::sync::atomic::Ordering::SeqCst) {
        return true;
    }
    let program = Interest::ENTER_PROGRAM.union(Interest::EXIT_PROGRAM);
    hooks
        .iter()
        .enumerate()
        .any(|(slot, list)| !list.is_empty() && !program.includes(Interest::bit_at(slot)))
}

fn kind_table<const N: usize>(
    passes: &[Box<dyn Pass<'_> + '_>],
    list: &[usize],
    includes: fn(&Interest, usize) -> bool,
) -> [u64; N] {
    let mut table = [0; N];
    for &index in list {
        let Some(pass) = passes.get(index) else {
            continue;
        };
        let interest = pass.interest();
        for (kind, bits) in table.iter_mut().enumerate() {
            if includes(&interest, kind) {
                *bits |= 1 << index;
            }
        }
    }
    table
}

fn tables(passes: &[Box<dyn Pass<'_> + '_>], hooks: &HookLists, mode: Mode) -> Tables {
    let empty: &[usize] = &[];
    let list = |hook: Interest| match mode {
        Mode::ByVariant => hooks[hook.slot()].as_slice(),
        _ => empty,
    };
    Tables {
        enter_expression: kind_table(
            passes,
            list(Interest::ENTER_EXPRESSION),
            Expression::includes,
        ),
        exit_expression: kind_table(
            passes,
            list(Interest::EXIT_EXPRESSION),
            Expression::includes,
        ),
        enter_statement: kind_table(passes, list(Interest::ENTER_STATEMENT), Statement::includes),
        exit_statement: kind_table(passes, list(Interest::EXIT_STATEMENT), Statement::includes),
    }
}

impl<'a> PassGroup<'a> {
    fn assemble(
        passes: Vec<Box<dyn Pass<'a> + 'a>>,
        interests: Vec<u16>,
        opts: PassOptions,
        declared: bool,
    ) -> Self {
        let names = passes.iter().map(|pass| pass.name()).collect();
        let counts = vec![0; passes.len()];
        let elapsed = vec![Duration::ZERO; passes.len()];
        let hooks = hook_lists(&interests);
        let walks = walks_nodes(&hooks);
        let mode = mode_for(&passes, declared);
        let tables = tables(&passes, &hooks, mode);
        let cascades = passes.iter().any(|pass| pass.cascades());
        Self {
            passes,
            names,
            counts,
            elapsed,
            hooks,
            walks,
            parked: None,
            mode,
            tables,
            opts,
            notes: Vec::new(),
            scratch: Scratch::default(),
            declared,
            retiring: false,
            scope: Scope::default(),
            cascades,
            nesting: 0,
            bodies: Vec::new(),
            spare_scoping: None,
        }
    }

    fn declared_bits(&self, pass: &dyn Pass<'_>) -> u16 {
        if self.declared {
            pass.interest().hook_bits()
        } else {
            Interest::ALL.hook_bits()
        }
    }

    fn apply_retirement(&mut self) {
        let retiring = self
            .passes
            .iter()
            .any(|pass| retired_bits(pass.as_ref()) != 0);
        if !retiring && !self.retiring {
            return;
        }
        self.retiring = retiring;
        self.hooks.iter_mut().for_each(Vec::clear);
        for (index, pass) in self.passes.iter().enumerate() {
            let bits = self.declared_bits(pass.as_ref()) & !retired_bits(pass.as_ref());
            for (slot, list) in self.hooks.iter_mut().enumerate() {
                if bits & (1 << slot) != 0 {
                    list.push(index);
                }
            }
        }
        self.walks = walks_nodes(&self.hooks);
        self.tables = tables(&self.passes, &self.hooks, self.mode);
    }

    pub fn new(passes: Vec<Box<dyn Pass<'a> + 'a>>, opts: PassOptions) -> Self {
        let passes = filtered(passes, &opts);
        let interests = passes
            .iter()
            .map(|pass| pass.interest().hook_bits())
            .collect();
        Self::assemble(passes, interests, opts, true)
    }

    #[cfg(feature = "test-hooks")]
    pub fn exhaustive_for_tests(passes: Vec<Box<dyn Pass<'a> + 'a>>, opts: PassOptions) -> Self {
        let passes = filtered(passes, &opts);
        let interests = vec![Interest::ALL.hook_bits(); passes.len()];
        Self::assemble(passes, interests, opts, false)
    }

    pub fn from_ctors(ctors: &[PassCtor<'a>], opts: PassOptions) -> Self {
        let passes = ctors.iter().map(|ctor| ctor()).collect();
        Self::new(passes, opts)
    }

    pub fn total_changes(&self) -> u32 {
        self.counts.iter().sum()
    }

    pub fn pass_names(&self) -> &[&'static str] {
        &self.names
    }

    pub fn per_pass_changes(&self) -> Vec<(&'static str, u32)> {
        self.names
            .iter()
            .zip(self.counts.iter())
            .map(|(name, count)| (*name, *count))
            .collect()
    }

    pub fn per_pass_elapsed(&self) -> Vec<(&'static str, Duration)> {
        self.names
            .iter()
            .zip(self.elapsed.iter())
            .map(|(name, elapsed)| (*name, *elapsed))
            .collect()
    }

    pub fn begin_iteration(&mut self) {
        self.counts.iter_mut().for_each(|count| *count = 0);
        self.elapsed
            .iter_mut()
            .for_each(|elapsed| *elapsed = Duration::ZERO);
        self.scratch.invented.clear();
        self.scratch.renamed.clear();
        self.opts.deferred_pending.set(false);
        self.opts.round_requested.set(false);
        self.opts.walk_whole.set(false);
        self.opts.binding_moved.set(false);
    }

    pub fn forget_marks(&mut self) {
        self.scope.forget();
    }

    pub fn begin_marks(&mut self) {
        self.scope.restart();
    }

    pub fn was_steered(&self) -> bool {
        self.scope.was_steered()
    }

    pub fn confirm_next(&mut self) {
        self.scope.confirm_next();
    }

    pub fn fall_back(&mut self) {
        self.scope.fall_back();
    }

    fn hooks_nodes(&self, index: usize) -> bool {
        let program = Interest::ENTER_PROGRAM.union(Interest::EXIT_PROGRAM);
        self.hooks
            .iter()
            .enumerate()
            .any(|(slot, list)| !program.includes(Interest::bit_at(slot)) && list.contains(&index))
    }

    fn scopes(&self) -> bool {
        self.passes.iter().enumerate().all(|(index, pass)| {
            locality_of(pass.as_ref()) != Locality::Program || !self.hooks_nodes(index)
        })
    }

    fn settles(&self) -> bool {
        !self.opts.round_requested.get()
            && !self.opts.binding_moved.get()
            && self
                .passes
                .iter()
                .zip(&self.counts)
                .all(|(pass, count)| *count == 0 || locality_of(pass.as_ref()) == Locality::Neutral)
    }

    fn steerable(&self) -> bool {
        #[cfg(feature = "test-hooks")]
        if self.mode == Mode::Probe {
            return false;
        }
        super::settled::parking_allowed()
    }

    fn begin_scope(&mut self, scopes: usize) {
        let recording = self.walks && self.scopes();
        let steerable = self.steerable();
        let neutral = self
            .passes
            .iter()
            .map(|pass| locality_of(pass.as_ref()) == Locality::Neutral);
        let marks = &mut self.scratch.marks;
        self.scope
            .begin(marks, recording, steerable, neutral, scopes);
        if self.total_changes() > 0 {
            self.scratch.marks.unplace();
        }
    }

    pub fn reset_passes(&mut self) {
        for pass in &mut self.passes {
            pass.reset();
        }
    }

    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    pub fn opts(&self) -> &PassOptions {
        &self.opts
    }

    pub fn deferred_pending(&self) -> bool {
        self.opts.deferred_pending.get()
    }

    pub fn round_requested(&self) -> bool {
        self.opts.round_requested.get()
    }

    pub fn set_defer_disabled(&mut self, disabled: bool) {
        self.opts.defer_disabled.set(disabled);
    }

    pub fn find_pass_index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|candidate| *candidate == name)
    }

    pub fn run_single_pass_enter_program(
        &mut self,
        alloc: &'a Allocator,
        program: &mut Program<'a>,
        scoping: Scoping,
        index: usize,
    ) -> (Scoping, u32) {
        self.opts.defer_disabled.set(true);
        let Self {
            passes,
            names,
            counts,
            opts,
            notes,
            scratch,
            ..
        } = self;
        counts[index] = 0;
        opts.changes_by_others_before_program_hook
            .set(counts.iter().sum());
        let mut runner = SinglePassRunner {
            pass: &mut passes[index],
            count: &mut counts[index],
            name: names[index],
            opts,
            notes,
            scratch,
            parked: None,
        };
        let scoping = traverse_mut(&mut runner, alloc, program, scoping, ());
        let count = *runner.count;
        self.opts.defer_disabled.set(false);
        (scoping, count)
    }

    fn run_program_hook(
        &mut self,
        hook: Interest,
        call: ProgramHook<'a>,
        node: &mut Program<'a>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        let Self {
            passes,
            names,
            counts,
            elapsed,
            hooks,
            opts,
            notes,
            scratch,
            ..
        } = self;
        for &index in &hooks[hook.slot()] {
            note_changes_before_hook(opts, counts, index);
            let (Some(pass), Some(count), Some(name), Some(time)) = (
                passes.get_mut(index),
                counts.get_mut(index),
                names.get(index),
                elapsed.get_mut(index),
            ) else {
                continue;
            };
            let started = Instant::now();
            let mut cx = Cx {
                tcx: ctx,
                changes: count,
                opts,
                notes,
                pass_name: name,
                scratch,
            };
            call(pass.as_mut(), node, &mut cx);
            *time += started.elapsed();
            #[cfg(feature = "test-hooks")]
            span_check::check(node, || format!("{name} ({})", hook.named().join(", ")));
        }
        scratch.marks.quiet();
    }
}

fn note_changes_before_hook(opts: &PassOptions, counts: &[u32], index: usize) {
    let total: u32 = counts.iter().sum();
    let own = counts.get(index).copied().unwrap_or(0);
    opts.changes_before_program_hook.set(total);
    opts.changes_by_others_before_program_hook
        .set(total.saturating_sub(own));
}

trait Dispatched {
    fn kind_index(&self) -> usize;
    fn includes(interest: &Interest, kind: usize) -> bool;
}

impl Dispatched for Expression<'_> {
    #[inline]
    fn kind_index(&self) -> usize {
        ExpressionKind::of(self).index()
    }

    fn includes(interest: &Interest, kind: usize) -> bool {
        interest.expressions().includes_index(kind)
    }
}

impl Dispatched for Statement<'_> {
    #[inline]
    fn kind_index(&self) -> usize {
        StatementKind::of(self).index()
    }

    fn includes(interest: &Interest, kind: usize) -> bool {
        interest.statements().includes_index(kind)
    }
}

#[inline]
fn above(index: usize) -> u64 {
    u32::try_from(index + 1)
        .ok()
        .and_then(|shift| u64::MAX.checked_shl(shift))
        .unwrap_or(0)
}

struct SinglePassRunner<'p, 'a> {
    pass: &'p mut Box<dyn Pass<'a> + 'a>,
    count: &'p mut u32,
    name: &'static str,
    opts: &'p PassOptions,
    notes: &'p mut Vec<Note>,
    scratch: &'p mut Scratch,
    parked: Option<AVec<'a, Statement<'a>>>,
}

impl<'p, 'a> Traverse<'a, ()> for SinglePassRunner<'p, 'a> {
    fn enter_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        let mut cx = Cx {
            tcx: ctx,
            changes: self.count,
            opts: self.opts,
            notes: self.notes,
            pass_name: self.name,
            scratch: self.scratch,
        };
        self.pass.enter_program(node, &mut cx);
        self.parked = park(node, ctx);
    }

    fn exit_program(&mut self, node: &mut Program<'a>, _ctx: &mut TraverseCtx<'a, ()>) {
        unpark(node, &mut self.parked);
        #[cfg(feature = "test-hooks")]
        super::work::record(super::work::Traversal {
            changes: *self.count,
            ..super::work::Traversal::default()
        });
    }
}

fn park<'a>(
    program: &mut Program<'a>,
    ctx: &TraverseCtx<'a, ()>,
) -> Option<AVec<'a, Statement<'a>>> {
    let empty = AVec::new_in(&ctx.ast);
    Some(std::mem::replace(&mut program.body, empty))
}

fn unpark<'a>(program: &mut Program<'a>, parked: &mut Option<AVec<'a, Statement<'a>>>) {
    if let Some(body) = parked.take() {
        program.body = body;
    }
}

pub(super) fn locality_of(pass: &dyn Pass<'_>) -> Locality {
    #[cfg(feature = "test-hooks")]
    if scope::mutants::treated_as_neutral(pass.name()) {
        return Locality::Neutral;
    }
    pass.locality()
}
