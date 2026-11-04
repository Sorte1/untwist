use oxc_allocator::{Address, GetAddress, UnstableAddress, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::{Ancestor, TraverseCtx};
use rustc_hash::{FxHashMap, FxHashSet};

use super::super::{Note, PassOptions};

#[derive(Clone, Copy)]
pub(super) enum Edited {
    List,
    Statement(Address),
    Within(Option<Address>),
}

pub(super) trait Edit {
    fn edited(&self) -> Edited;
}

impl Edit for AVec<'_, Statement<'_>> {
    fn edited(&self) -> Edited {
        Edited::List
    }
}

impl Edit for Statement<'_> {
    fn edited(&self) -> Edited {
        Edited::Statement(self.address())
    }
}

impl Edit for Function<'_> {
    fn edited(&self) -> Edited {
        Edited::Within(Some(self.unstable_address()))
    }
}

impl Edit for VariableDeclaration<'_> {
    fn edited(&self) -> Edited {
        Edited::Within(Some(self.unstable_address()))
    }
}

macro_rules! nested_edits {
    ($($node:ident),*) => {
        $(
            impl Edit for $node<'_> {
                fn edited(&self) -> Edited {
                    Edited::Within(None)
                }
            }
        )*
    };
}

nested_edits!(
    Expression,
    ArrowFunctionExpression,
    ObjectProperty,
    MethodDefinition,
    PropertyDefinition,
    TemplateLiteral,
    AssignmentTarget
);

fn holds_statements(ancestor: &Ancestor<'_, '_>) -> bool {
    matches!(
        ancestor,
        Ancestor::ProgramBody(_)
            | Ancestor::BlockStatementBody(_)
            | Ancestor::FunctionBodyStatements(_)
            | Ancestor::StaticBlockBody(_)
            | Ancestor::SwitchCaseConsequent(_)
            | Ancestor::TSModuleBlockBody(_)
    )
}

enum Start {
    List(Address),
    Statement(Address),
}

impl Start {
    fn parts(&self) -> (Address, Option<Address>) {
        match *self {
            Start::List(owner) => (owner, None),
            Start::Statement(statement) => (statement, Some(statement)),
        }
    }
}

fn start_of<'a, 't>(
    edited: Edited,
    ctx: &'t TraverseCtx<'a, ()>,
) -> Option<(Start, impl Iterator<Item = Ancestor<'a, 't>>)> {
    let mut ancestors = ctx.ancestors().peekable();
    match edited {
        Edited::List => Some((Start::List(ctx.parent().address()), ancestors)),
        Edited::Statement(statement) => Some((Start::Statement(statement), ancestors)),
        Edited::Within(own) => {
            let mut below = own;
            while let Some(ancestor) = ancestors.peek() {
                if ancestor.is_parent_of_statement() {
                    return below.map(|statement| (Start::Statement(statement), ancestors));
                }
                below = Some(ancestor.address());
                ancestors.next();
            }
            None
        }
    }
}

#[derive(Default)]
pub struct Scratch {
    pub(crate) invented: FxHashSet<String>,
    pub(crate) renamed: FxHashMap<SymbolId, String>,
    pub(crate) marks: Marks,
}

#[derive(Default)]
pub struct Marks {
    recording: bool,
    pub(super) touched: bool,
    changed: bool,
    pending: Vec<Note>,
    lists: FxHashSet<Address>,
    statements: FxHashSet<Address>,
    whole_lists: FxHashSet<Address>,
    whole_statements: FxHashSet<Address>,
    notes: FxHashMap<Address, Vec<Note>>,
    unplaced: bool,
    scopes: usize,
}

impl Marks {
    fn unplaced(recording: bool) -> Self {
        Self {
            recording,
            unplaced: true,
            ..Self::default()
        }
    }

    #[inline]
    pub(crate) fn changed(&mut self) {
        if self.marks_path() {
            self.touched = true;
            self.changed = true;
        }
    }

    #[inline]
    fn marks_path(&self) -> bool {
        self.recording && !self.unplaced
    }

    fn mark_path<'a, 't>(
        &mut self,
        mut below: Option<Address>,
        ancestors: impl Iterator<Item = Ancestor<'a, 't>>,
    ) {
        #[cfg(feature = "test-hooks")]
        let mut skip = mutants::skipped();
        for ancestor in ancestors {
            if holds_statements(&ancestor) {
                #[cfg(feature = "test-hooks")]
                if skip > 0 {
                    skip -= 1;
                    below = Some(ancestor.address());
                    continue;
                }
                if let Some(statement) = below {
                    self.statements.insert(statement);
                }
                if !self.lists.insert(ancestor.address()) {
                    return;
                }
            }
            below = Some(ancestor.address());
        }
    }

    fn attribute<'a, 't>(
        &mut self,
        mut below: Option<Address>,
        ancestors: impl Iterator<Item = Ancestor<'a, 't>>,
        notes: &[Note],
    ) {
        for ancestor in ancestors {
            if holds_statements(&ancestor) {
                if let Some(statement) = below {
                    self.notes
                        .entry(statement)
                        .or_default()
                        .extend_from_slice(notes);
                }
                self.notes
                    .entry(ancestor.address())
                    .or_default()
                    .extend_from_slice(notes);
            }
            below = Some(ancestor.address());
        }
    }

    fn record(&mut self, edited: Edited, ctx: &TraverseCtx<'_, ()>) {
        let changed = std::mem::take(&mut self.changed);
        let pending = std::mem::take(&mut self.pending);
        if !pending.is_empty() {
            let Some((start, ancestors)) = start_of(edited, ctx) else {
                self.unplace();
                return;
            };
            let (_, below) = start.parts();
            self.attribute(below, ancestors, &pending);
        }
        if !changed {
            return;
        }
        let Some((start, ancestors)) = start_of(edited, ctx) else {
            self.unplace();
            return;
        };
        let (first, below) = start.parts();
        match start {
            Start::List(_) => self.whole_lists.insert(first),
            Start::Statement(_) => self.whole_statements.insert(first),
        };
        self.mark_path(below, ancestors);
    }

    #[inline]
    pub(crate) fn pins(&self) -> bool {
        self.marks_path()
    }

    pub(crate) fn pin(&mut self, node: Address, ctx: &TraverseCtx<'_, ()>) {
        if self.marks_path() {
            self.mark_path(Some(node), ctx.ancestors());
        }
    }

    pub(crate) fn note(&mut self, note: &Note) {
        if self.marks_path() {
            self.touched = true;
            self.pending.push(note.clone());
        }
    }

    fn carry(&mut self, below: Option<Address>, carried: &[Note], ctx: &TraverseCtx<'_, ()>) {
        if self.marks_path() {
            self.attribute(below, ctx.ancestors(), carried);
        }
    }

    pub(super) fn quiet(&mut self) {
        self.touched = false;
        self.changed = false;
        self.pending.clear();
    }

    pub(super) fn unplace(&mut self) {
        if !self.unplaced {
            *self = Self::unplaced(self.recording);
        }
    }

    fn steers(&self) -> bool {
        let marked = self.lists.len() + self.whole_lists.len();
        marked.saturating_mul(STEER_SHARE) < self.scopes
    }
}

const STEER_SHARE: usize = 2;

const FADING: u32 = 100;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Whole {
    List(usize),
    Statement(usize),
}

struct Shelf<'a> {
    depth: usize,
    shelved: Vec<(usize, Option<Statement<'a>>)>,
    next: usize,
    cursor: usize,
}

#[derive(Default)]
pub(super) struct Scope<'a> {
    held: Option<Marks>,
    neutral: Vec<bool>,
    whole: Option<Whole>,
    parked: Option<AVec<'a, Statement<'a>>>,
    shelves: Vec<Shelf<'a>>,
    placeholders: Vec<Statement<'a>>,
    skipping: bool,
    last: Option<Marks>,
    was_steered: bool,
    pub(super) engaged: bool,
    rounds: u32,
    steering: bool,
    could_record: bool,
    previous_changes: u32,
    faded: bool,
    confirming: bool,
    fallen_back: bool,
    #[cfg(feature = "test-hooks")]
    pub(super) lists_parked: u32,
    #[cfg(feature = "test-hooks")]
    pub(super) statements_shelved: u32,
}

impl<'a> Scope<'a> {
    pub(super) fn forget(&mut self) {
        self.last = None;
    }

    pub(super) fn restart(&mut self) {
        self.last = None;
        self.fallen_back = false;
        self.rounds = 0;
        self.could_record = false;
        self.previous_changes = u32::MAX;
        self.faded = false;
    }

    pub(super) fn was_steered(&self) -> bool {
        self.was_steered
    }

    pub(super) fn steers_next(&self) -> bool {
        self.rounds > 0
            && self.could_record
            && !self.faded
            && !self.confirming
            && !self.fallen_back
            && super::super::settled::parking_allowed()
    }

    pub(super) fn set_steering(&mut self, steering: bool) {
        self.steering = steering;
    }

    pub(super) fn confirm_next(&mut self) {
        self.last = None;
        self.confirming = true;
    }

    #[cfg(feature = "test-hooks")]
    pub(super) fn confirming(&self) -> bool {
        self.confirming
    }

    pub(super) fn fall_back(&mut self) {
        self.last = None;
        self.fallen_back = true;
    }

    pub(super) fn begin(
        &mut self,
        next: &mut Marks,
        recording: bool,
        steered: bool,
        neutral: impl Iterator<Item = bool>,
        scopes: usize,
    ) {
        let last = self.last.take();
        self.rounds = self.rounds.saturating_add(1);
        self.could_record = recording;
        let recording = recording && std::mem::take(&mut self.steering);
        let steered = steered && !self.fallen_back;
        self.held = last.filter(|marks| recording && steered && marks.steers());
        self.was_steered = self.held.is_some();
        self.engaged = self.was_steered;
        self.neutral.clear();
        self.neutral.extend(neutral);
        *next = Marks {
            recording,
            scopes,
            ..Marks::default()
        };
        self.whole = None;
        #[cfg(feature = "test-hooks")]
        {
            self.lists_parked = 0;
            self.statements_shelved = 0;
        }
    }

    pub(super) fn finish(&mut self, next: &mut Marks, settles: bool, changes: u32) {
        let previous = std::mem::replace(&mut self.previous_changes, changes);
        let converging = changes.saturating_mul(FADING) < previous;
        self.faded = converging && self.rounds > 1;
        self.held = None;
        self.engaged = false;
        self.confirming = false;
        let next = std::mem::take(next);
        self.last = (next.recording && settles && !next.unplaced && !converging).then_some(next);
    }

    #[inline]
    pub(super) fn steered(&self) -> bool {
        self.held.is_some()
    }

    #[cold]
    #[inline(never)]
    pub(super) fn touched(
        &self,
        next: &mut Marks,
        edited: Edited,
        ctx: &TraverseCtx<'_, ()>,
        counts: &[u32],
        opts: &PassOptions,
    ) {
        next.touched = false;
        if !next.marks_path() {
            next.quiet();
            return;
        }
        let settling = !opts.binding_moved.get()
            && counts
                .iter()
                .zip(&self.neutral)
                .all(|(count, neutral)| *count == 0 || *neutral);
        if settling {
            next.record(edited, ctx);
        } else {
            next.unplace();
        }
    }

    #[inline(never)]
    pub(super) fn leave_list_before(
        &mut self,
        list: &mut AVec<'a, Statement<'a>>,
        depth: usize,
    ) -> bool {
        if self.unpark(list) {
            return true;
        }
        if self.shelving() {
            self.restore(list, depth);
        }
        false
    }

    pub(super) fn park(
        &mut self,
        next: &mut Marks,
        list: &mut AVec<'a, Statement<'a>>,
        ctx: &TraverseCtx<'a, ()>,
        notes: &mut Vec<Note>,
    ) -> Steer {
        let Some(held) = &self.held else {
            return Steer::Walk;
        };
        if self.whole.is_some() {
            return Steer::Walk;
        }
        let owner = ctx.parent().address();
        if held.whole_lists.contains(&owner) {
            self.whole = Some(Whole::List(ctx.ancestors_depth()));
            return Steer::Walk;
        }
        if held.lists.contains(&owner) {
            return Steer::Shelve;
        }
        if let Some(carried) = held.notes.get(&owner) {
            notes.extend_from_slice(carried);
            next.carry(None, carried, ctx);
        }
        self.parked = Some(std::mem::replace(list, AVec::new_in(&ctx.ast)));
        #[cfg(feature = "test-hooks")]
        {
            self.lists_parked += 1;
        }
        Steer::Parked
    }

    pub(super) fn shelve(&mut self, list: &mut AVec<'a, Statement<'a>>, ctx: &TraverseCtx<'a, ()>) {
        let Some(held) = &self.held else {
            return;
        };
        let mut shelved = Vec::new();
        for (index, statement) in list.iter_mut().enumerate() {
            let address = statement.address();
            if held.statements.contains(&address) || held.whole_statements.contains(&address) {
                continue;
            }
            let placeholder = self
                .placeholders
                .pop()
                .unwrap_or_else(|| Statement::new_empty_statement(SPAN, &ctx.ast));
            shelved.push((index, Some(std::mem::replace(statement, placeholder))));
        }
        if shelved.is_empty() {
            return;
        }
        #[cfg(feature = "test-hooks")]
        {
            self.statements_shelved += u32::try_from(shelved.len()).unwrap_or(u32::MAX);
        }
        self.shelves.push(Shelf {
            depth: ctx.ancestors_depth(),
            shelved,
            next: 0,
            cursor: 0,
        });
    }

    pub(super) fn restore(&mut self, list: &mut AVec<'a, Statement<'a>>, depth: usize) {
        if self.shelves.last().is_none_or(|shelf| shelf.depth != depth) {
            return;
        }
        let Some(shelf) = self.shelves.pop() else {
            return;
        };
        for (index, statement) in shelf.shelved {
            let (Some(statement), Some(slot)) = (statement, list.get_mut(index)) else {
                continue;
            };
            self.placeholders.push(std::mem::replace(slot, statement));
        }
    }

    pub(super) fn visit_shelved(
        &mut self,
        next: &mut Marks,
        slot: &mut Statement<'a>,
        depth: usize,
        walk_whole: bool,
        notes: &mut Vec<Note>,
        ctx: &TraverseCtx<'a, ()>,
    ) -> bool {
        let Some(shelf) = self.shelves.last_mut().filter(|shelf| shelf.depth == depth) else {
            return false;
        };
        let index = shelf.cursor;
        shelf.cursor += 1;
        let Some((shelved_at, statement)) = shelf.shelved.get_mut(shelf.next) else {
            return false;
        };
        if *shelved_at != index {
            return false;
        }
        shelf.next += 1;
        if walk_whole {
            if let Some(statement) = statement.take() {
                self.placeholders.push(std::mem::replace(slot, statement));
            }
            return false;
        }
        let address = statement.as_ref().map(GetAddress::address);
        let carried = address.and_then(|address| {
            self.held
                .as_ref()
                .and_then(|held| held.notes.get(&address))
                .map(|carried| (address, carried.clone()))
        });
        if let Some((address, carried)) = carried {
            notes.extend_from_slice(&carried);
            next.carry(Some(address), &carried, ctx);
        }
        self.skipping = true;
        true
    }

    #[inline]
    pub(super) fn shelving(&self) -> bool {
        !self.shelves.is_empty()
    }

    #[inline]
    pub(super) fn skip_exit(&mut self) -> bool {
        if self.skipping {
            self.skipping = false;
            return true;
        }
        false
    }

    #[inline]
    pub(super) fn unpark(&mut self, list: &mut AVec<'a, Statement<'a>>) -> bool {
        if self.parked.is_none() {
            return false;
        }
        match self.parked.take() {
            Some(body) => {
                *list = body;
                true
            }
            None => false,
        }
    }

    pub(super) fn stop_parking(&mut self) {
        self.held = None;
    }

    pub(super) fn enter_statement(&mut self, statement: &Statement<'a>, depth: usize) {
        let Some(held) = &self.held else {
            return;
        };
        if self.whole.is_none() && held.whole_statements.contains(&statement.address()) {
            self.whole = Some(Whole::Statement(depth));
        }
    }

    #[inline]
    pub(super) fn leave_list(&mut self, depth: usize) {
        if self.whole == Some(Whole::List(depth)) {
            self.whole = None;
        }
    }

    #[inline]
    pub(super) fn leave_statement(&mut self, depth: usize) {
        if self.whole == Some(Whole::Statement(depth)) {
            self.whole = None;
        }
    }
}

pub(super) enum Steer {
    Walk,
    Shelve,
    Parked,
}

#[cfg(feature = "test-hooks")]
pub(crate) mod mutants {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, PoisonError};

    static SKIP_INNERMOST_LIST: AtomicBool = AtomicBool::new(false);

    static NEUTRAL: Mutex<Option<&'static str>> = Mutex::new(None);

    #[doc(hidden)]
    pub fn treat_as_neutral_for_tests(pass: Option<&'static str>) {
        *NEUTRAL.lock().unwrap_or_else(PoisonError::into_inner) = pass;
    }

    pub(in super::super) fn treated_as_neutral(pass: &str) -> bool {
        *NEUTRAL.lock().unwrap_or_else(PoisonError::into_inner) == Some(pass)
    }

    #[doc(hidden)]
    pub fn skip_innermost_list_for_tests(on: bool) {
        SKIP_INNERMOST_LIST.store(on, Ordering::SeqCst);
    }

    pub(super) fn skipped() -> usize {
        usize::from(SKIP_INNERMOST_LIST.load(Ordering::SeqCst))
    }
}
