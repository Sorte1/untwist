use std::cell::Cell;

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::reference::{Reference, ReferenceFlags};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast_util::respan::{fresh_spans_expression, fresh_spans_statement};
use crate::semantic::renamed_since_refresh;
use crate::semantic::unbound::note_unbound;
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
    let declared = declared_names(|names| names.visit_expression(expr));
    Bind::new(scoping, parent, declared).visit_expression(expr);
}

fn seed_statement(stmt: &mut Statement<'_>, parent: ScopeId, scoping: &mut Scoping) {
    Seed { scoping, parent }.visit_statement(stmt);
    let declared = declared_names(|names| names.visit_statement(stmt));
    Bind::new(scoping, parent, declared).visit_statement(stmt);
}

#[derive(Default)]
struct DeclaredNames {
    names: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for DeclaredNames {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.to_string());
    }
}

fn declared_names(visit: impl FnOnce(&mut DeclaredNames)) -> FxHashSet<String> {
    let mut names = DeclaredNames::default();
    visit(&mut names);
    names.names
}

struct Bind<'s> {
    scoping: &'s mut Scoping,
    open: Vec<ScopeId>,
    declared: FxHashSet<String>,
    opaque: u32,
}

impl<'s> Bind<'s> {
    fn new(scoping: &'s mut Scoping, parent: ScopeId, declared: FxHashSet<String>) -> Self {
        Self {
            scoping,
            open: vec![parent],
            declared,
            opaque: 0,
        }
    }

    fn bind(&mut self, it: &IdentifierReference<'_>, flags: ReferenceFlags) {
        if it.reference_id.get().is_some() {
            return;
        }
        let name = it.name.as_str();
        let Some(&scope) = self.open.last() else {
            note_unbound(name);
            return;
        };
        if self.opaque > 0 || name == "arguments" || self.declared.contains(name) {
            note_unbound(name);
            return;
        }
        let id = match self.scoping.find_binding(scope, it.name) {
            Some(symbol) => {
                let reference = Reference::new_with_symbol_id(NodeId::DUMMY, symbol, scope, flags);
                let id = self.scoping.create_reference(reference);
                self.scoping.add_resolved_reference(symbol, id);
                id
            }
            None => {
                let id = self
                    .scoping
                    .create_reference(Reference::new(NodeId::DUMMY, scope, flags));
                self.scoping.add_root_unresolved_reference(it.name, id);
                id
            }
        };
        it.reference_id.set(Some(id));
    }

    fn target(&mut self, it: &mut AssignmentTarget<'_>, flags: ReferenceFlags) {
        match it {
            AssignmentTarget::AssignmentTargetIdentifier(id) => self.bind(id, flags),
            AssignmentTarget::ArrayAssignmentTarget(_)
            | AssignmentTarget::ObjectAssignmentTarget(_) => {
                self.opaque += 1;
                walk_mut::walk_assignment_target(self, it);
                self.opaque -= 1;
            }
            _ => walk_mut::walk_assignment_target(self, it),
        }
    }
}

impl<'a> VisitMut<'a> for Bind<'_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        let scope = scope_id.get().or_else(|| self.open.last().copied());
        if let Some(scope) = scope {
            self.open.push(scope);
        }
    }

    fn leave_scope(&mut self) {
        if self.open.len() > 1 {
            self.open.pop();
        }
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        self.bind(it, ReferenceFlags::Read);
    }

    fn visit_assignment_expression(&mut self, it: &mut AssignmentExpression<'a>) {
        let flags = if it.operator == AssignmentOperator::Assign {
            ReferenceFlags::Write
        } else {
            ReferenceFlags::Read | ReferenceFlags::Write
        };
        self.target(&mut it.left, flags);
        self.visit_expression(&mut it.right);
    }

    fn visit_assignment_target(&mut self, it: &mut AssignmentTarget<'a>) {
        self.target(it, ReferenceFlags::Write);
    }

    fn visit_simple_assignment_target(&mut self, it: &mut SimpleAssignmentTarget<'a>) {
        match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.bind(id, ReferenceFlags::Read | ReferenceFlags::Write);
            }
            _ => walk_mut::walk_simple_assignment_target(self, it),
        }
    }
}

struct RenamedNames<'p, 'q> {
    program: &'p Program<'q>,
    found: bool,
}

impl<'a> VisitJs<'a> for RenamedNames<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= renamed_since_refresh(self.program, it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= renamed_since_refresh(self.program, it.name.as_str());
    }
}

fn mentions_a_renamed_name(program: &Program<'_>, expr: &Expression<'_>) -> bool {
    let mut names = RenamedNames {
        program,
        found: false,
    };
    names.visit_expression(expr);
    names.found
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

    pub fn forget_renamed_since_refresh(&mut self, program: &Program<'_>) -> usize {
        let before = self.edits.len();
        self.edits
            .retain(|_, expr| !mentions_a_renamed_name(program, expr));
        before - self.edits.len()
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

    pub fn take(&mut self, span: Span) -> Option<Expression<'a>> {
        self.edits.remove(&span)
    }

    pub fn put_back(&mut self, span: Span, expr: Expression<'a>) {
        self.edits.insert(span, expr);
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
        let mut applier = ExprApplier {
            edits: self.edits,
            applied: FxHashSet::default(),
            alloc,
            scopes: Scopes::new(scoping),
        };
        applier.visit_program(program);
        applier.applied.len()
    }
}

struct Scopes<'s> {
    scoping: &'s mut Scoping,
    open: Vec<ScopeId>,
}

impl<'s> Scopes<'s> {
    fn new(scoping: &'s mut Scoping) -> Self {
        let root = scoping.root_scope_id();
        Self {
            scoping,
            open: vec![root],
        }
    }

    fn innermost(&self) -> ScopeId {
        self.open
            .last()
            .copied()
            .unwrap_or_else(|| self.scoping.root_scope_id())
    }

    fn enter(&mut self, scope_id: &Cell<Option<ScopeId>>) {
        let scope = scope_id.get().unwrap_or_else(|| self.innermost());
        self.open.push(scope);
    }

    fn leave(&mut self) {
        if self.open.len() > 1 {
            self.open.pop();
        }
    }
}

struct ExprApplier<'a, 's> {
    edits: FxHashMap<Span, Expression<'a>>,
    applied: FxHashSet<Span>,
    alloc: &'a Allocator,
    scopes: Scopes<'s>,
}

impl<'a> VisitMut<'a> for ExprApplier<'a, '_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.enter(scope_id);
    }

    fn leave_scope(&mut self) {
        self.scopes.leave();
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let span = it.span();
        if let Some(replacement) = self.edits.get(&span) {
            *it = replacement.clone_in(self.alloc);
            note_replaced(it, span);
            let parent = self.scopes.innermost();
            seed_expression(it, parent, self.scopes.scoping);
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
        let mut applier = StmtApplier {
            edits: self.edits,
            applied: FxHashSet::default(),
            alloc,
            scopes: Scopes::new(scoping),
        };
        applier.visit_program(program);
        applier.applied.len()
    }
}

struct StmtApplier<'a, 's> {
    edits: FxHashMap<Span, StmtEdit<'a>>,
    applied: FxHashSet<Span>,
    alloc: &'a Allocator,
    scopes: Scopes<'s>,
}

impl<'a> VisitMut<'a> for StmtApplier<'a, '_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.enter(scope_id);
    }

    fn leave_scope(&mut self) {
        self.scopes.leave();
    }

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
            let parent = self.scopes.innermost();
            for replacement in &mut replacements {
                note_replaced(replacement, span);
                seed_statement(replacement, parent, self.scopes.scoping);
            }
            self.applied.insert(span);
            it.extend(replacements);
        }
    }
}
