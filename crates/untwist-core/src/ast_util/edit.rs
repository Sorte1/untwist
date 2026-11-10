use std::cell::Cell;

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast_util::respan::{fresh_spans_expression, fresh_spans_statement};
use crate::source_map::origins::note_replaced;

fn is_usable(span: Span) -> bool {
    span.start < span.end
}

struct Seed<'s> {
    scoping: &'s mut Scoping,
    parent: ScopeId,
}

impl Seed<'_> {
    fn enter(&mut self, cell: &Cell<Option<ScopeId>>, flags: ScopeFlags) -> ScopeId {
        let flags = self.scoping.get_new_scope_flags(flags, self.parent);
        let fresh = self
            .scoping
            .add_scope(Some(self.parent), NodeId::DUMMY, flags);
        cell.set(Some(fresh));
        std::mem::replace(&mut self.parent, fresh)
    }
}

macro_rules! seeded {
    ($($hook:ident($node:ty, $walk:ident, $flags:expr);)*) => {
        $(
            fn $hook(&mut self, it: &mut $node) {
                let previous = self.enter(&it.scope_id, $flags);
                walk_mut::$walk(self, it);
                self.parent = previous;
            }
        )*
    };
}

impl<'a> VisitMut<'a> for Seed<'_> {
    seeded! {
        visit_block_statement(BlockStatement<'a>, walk_block_statement, ScopeFlags::empty());
        visit_catch_clause(CatchClause<'a>, walk_catch_clause, ScopeFlags::CatchClause);
        visit_for_statement(ForStatement<'a>, walk_for_statement, ScopeFlags::empty());
        visit_for_in_statement(ForInStatement<'a>, walk_for_in_statement, ScopeFlags::empty());
        visit_for_of_statement(ForOfStatement<'a>, walk_for_of_statement, ScopeFlags::empty());
        visit_switch_statement(SwitchStatement<'a>, walk_switch_statement, ScopeFlags::empty());
        visit_with_statement(WithStatement<'a>, walk_with_statement, ScopeFlags::With);
        visit_class(Class<'a>, walk_class, ScopeFlags::empty());
        visit_static_block(StaticBlock<'a>, walk_static_block, ScopeFlags::ClassStaticBlock);
        visit_arrow_function_expression(
            ArrowFunctionExpression<'a>,
            walk_arrow_function_expression,
            ScopeFlags::Function | ScopeFlags::Arrow
        );
    }

    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        let previous = self.enter(&it.scope_id, flags | ScopeFlags::Function);
        walk_mut::walk_function(self, it, flags);
        self.parent = previous;
    }
}

fn seed_expression(expr: &mut Expression<'_>, parent: ScopeId, scoping: &mut Scoping) {
    Seed { scoping, parent }.visit_expression(expr);
}

fn seed_statement(stmt: &mut Statement<'_>, parent: ScopeId, scoping: &mut Scoping) {
    Seed { scoping, parent }.visit_statement(stmt);
}

#[derive(Default)]
pub struct ExprEdits<'a> {
    edits: FxHashMap<Span, Expression<'a>>,
}

impl<'a> ExprEdits<'a> {
    pub fn new() -> Self {
        Self {
            edits: FxHashMap::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.edits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn insert(&mut self, span: Span, mut expr: Expression<'a>) -> bool {
        if !is_usable(span) || self.edits.contains_key(&span) {
            return false;
        }
        if !fresh_spans_expression(&mut expr) {
            return false;
        }
        self.edits.insert(span, expr);
        true
    }

    pub fn remove(&mut self, span: Span) -> bool {
        self.edits.remove(&span).is_some()
    }

    pub fn apply(
        self,
        program: &mut Program<'a>,
        alloc: &'a Allocator,
        scoping: &mut Scoping,
    ) -> usize {
        if self.edits.is_empty() {
            return 0;
        }
        let root = scoping.root_scope_id();
        let mut applier = ExprApplier {
            edits: self.edits,
            applied: FxHashSet::default(),
            alloc,
            scoping,
            root,
        };
        applier.visit_program(program);
        applier.applied.len()
    }
}

struct ExprApplier<'a, 's> {
    edits: FxHashMap<Span, Expression<'a>>,
    applied: FxHashSet<Span>,
    alloc: &'a Allocator,
    scoping: &'s mut Scoping,
    root: ScopeId,
}

impl<'a> VisitMut<'a> for ExprApplier<'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let span = it.span();
        if let Some(replacement) = self.edits.get(&span) {
            *it = replacement.clone_in(self.alloc);
            note_replaced(it, span);
            seed_expression(it, self.root, self.scoping);
            self.applied.insert(span);
        }
        walk_mut::walk_expression(self, it);
    }
}

pub enum StmtEdit<'a> {
    Remove,
    ReplaceMany(Vec<Statement<'a>>),
}

#[derive(Default)]
pub struct StmtEdits<'a> {
    edits: FxHashMap<Span, StmtEdit<'a>>,
}

impl<'a> StmtEdits<'a> {
    pub fn new() -> Self {
        Self {
            edits: FxHashMap::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.edits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn remove(&mut self, span: Span) -> bool {
        if !is_usable(span) {
            return false;
        }
        self.edits.insert(span, StmtEdit::Remove);
        true
    }

    pub fn replace(&mut self, span: Span, mut statements: Vec<Statement<'a>>) -> bool {
        if !is_usable(span) || self.edits.contains_key(&span) {
            return false;
        }
        for statement in &mut statements {
            if !fresh_spans_statement(statement) {
                return false;
            }
        }
        self.edits.insert(span, StmtEdit::ReplaceMany(statements));
        true
    }

    pub fn apply(
        self,
        program: &mut Program<'a>,
        alloc: &'a Allocator,
        scoping: &mut Scoping,
    ) -> usize {
        if self.edits.is_empty() {
            return 0;
        }
        let root = scoping.root_scope_id();
        let mut applier = StmtApplier {
            edits: self.edits,
            applied: FxHashSet::default(),
            alloc,
            scoping,
            root,
        };
        applier.visit_program(program);
        applier.applied.len()
    }
}

struct StmtApplier<'a, 's> {
    edits: FxHashMap<Span, StmtEdit<'a>>,
    applied: FxHashSet<Span>,
    alloc: &'a Allocator,
    scoping: &'s mut Scoping,
    root: ScopeId,
}

impl<'a> VisitMut<'a> for StmtApplier<'a, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        walk_mut::walk_statements(self, it);
        let hits = it
            .iter()
            .filter(|stmt| self.edits.contains_key(&stmt.span()))
            .count();
        if hits == 0 {
            return;
        }
        let taken: Vec<Statement<'a>> = it.drain(..).collect();
        for statement in taken {
            let span = statement.span();
            let mut replacements: Vec<Statement<'a>> = match self.edits.get(&span) {
                None => {
                    it.push(statement);
                    continue;
                }
                Some(StmtEdit::Remove) => Vec::new(),
                Some(StmtEdit::ReplaceMany(items)) => {
                    items.iter().map(|item| item.clone_in(self.alloc)).collect()
                }
            };
            for replacement in &mut replacements {
                note_replaced(replacement, span);
                seed_statement(replacement, self.root, self.scoping);
            }
            self.applied.insert(span);
            it.extend(replacements);
        }
    }
}
