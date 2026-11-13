use std::hash::{Hash, Hasher};

use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use untwist_core::guard_shape::{may_be_a_guard, read_target};
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Pass, StatementKind, StatementVariants,
};
use untwist_core::semantic::{
    assumed_hazard, block_function_symbols, is_constant_binding, scope_hazard,
};

use crate::{var_capture, var_placement};

pub const PASS: &str = "var-to-let";

pub const SHAPE_STATEMENT: &str = "statement";
pub const SHAPE_FOR_HEAD: &str = "for-head";
pub const SHAPE_FOR_IN_OF_HEAD: &str = "for-in-of-head";
pub const PLACED_AT_FIRST_WRITE: &str = "placed-at-first-write";
pub const HOISTED_TO_TOP: &str = "hoisted-to-top";

const REFUSED_NAMES: [&str; 5] = ["let", "yield", "await", "arguments", "eval"];
const SMALL_NAME_SET: usize = 8;

const WATCHED_STATEMENTS: [StatementKind; 6] = [
    StatementKind::FunctionDeclaration,
    StatementKind::SwitchStatement,
    StatementKind::ClassDeclaration,
    StatementKind::ForStatement,
    StatementKind::ForInStatement,
    StatementKind::ForOfStatement,
];

pub(crate) fn is_text_read(function: &Function<'_>) -> bool {
    may_be_a_guard(function) || read_target(function).is_some()
}

#[derive(Default)]
pub(crate) struct Frozen {
    open: Vec<ScopeId>,
}

impl Frozen {
    pub(crate) fn clear(&mut self) {
        self.open.clear();
    }

    pub(crate) fn is_open(&self) -> bool {
        !self.open.is_empty()
    }

    pub(crate) fn enter(&mut self, function: &Function<'_>) {
        if let (Some(scope), true) = (function.scope_id.get(), is_text_read(function)) {
            self.open.push(scope);
        }
    }

    pub(crate) fn exit(&mut self, function: &Function<'_>) {
        if function.scope_id.get().is_some() && self.open.last().copied() == function.scope_id.get()
        {
            self.open.pop();
        }
    }
}

#[derive(Default)]
pub struct VarToLet {
    frozen: Frozen,
    active: bool,
    pending: Vec<u64>,
    bare: Vec<u64>,
    class_scopes: FxHashSet<ScopeId>,
    switch_scopes: FxHashSet<ScopeId>,
    block_functions: Option<FxHashSet<u64>>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(VarToLet::default())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Site {
    Statement,
    ForHead,
    ForInOfHead,
}

#[cfg(feature = "test-hooks")]
fn note_shape(site: Site) {
    let shape = match site {
        Site::Statement => SHAPE_STATEMENT,
        Site::ForHead => SHAPE_FOR_HEAD,
        Site::ForInOfHead => SHAPE_FOR_IN_OF_HEAD,
    };
    crate::untranspile::shapes::note(PASS, shape);
}

#[cfg(not(feature = "test-hooks"))]
fn note_shape(_site: Site) {}

#[cfg(feature = "test-hooks")]
fn note_placement(shape: &'static str) {
    crate::untranspile::shapes::note(PASS, shape);
}

#[cfg(not(feature = "test-hooks"))]
fn note_placement(_shape: &'static str) {}

fn is_function_scope(scoping: &Scoping, scope: ScopeId) -> bool {
    let flags = scoping.scope_flags(scope);
    flags.intersects(ScopeFlags::Function | ScopeFlags::ClassStaticBlock) && !flags.is_top()
}

fn bare_var_scope(scoping: &Scoping, declaration: &VariableDeclaration<'_>) -> Option<ScopeId> {
    declaration.declarations.iter().find_map(|declarator| {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        if declarator.init.is_some() {
            return None;
        }
        Some(scoping.symbol_scope_id(id.symbol_id.get()?))
    })
}

type Bindings<'a> = Vec<(SymbolId, Ident<'a>)>;

struct Candidate<'a> {
    index: usize,
    bindings: Bindings<'a>,
    refused: bool,
    captured: bool,
}

struct Facts<'f> {
    scoping: &'f Scoping,
    class_scopes: &'f FxHashSet<ScopeId>,
    block_functions: &'f FxHashSet<u64>,
}

fn name_hash(name: &str) -> u64 {
    let mut hasher = FxHasher::default();
    name.hash(&mut hasher);
    hasher.finish()
}

fn block_function_names(scoping: &Scoping) -> FxHashSet<u64> {
    block_function_symbols(scoping)
        .into_iter()
        .map(|symbol| name_hash(scoping.symbol_name(symbol)))
        .collect()
}

fn set_bit(bits: &mut [u64], index: usize) {
    if let Some(word) = bits.get_mut(index / 64) {
        *word |= 1u64 << (index % 64);
    }
}

fn take_bit(bits: &mut [u64], index: usize) -> bool {
    let Some(word) = bits.get_mut(index / 64) else {
        return false;
    };
    let mask = 1u64 << (index % 64);
    let was = *word & mask != 0;
    *word &= !mask;
    was
}

fn is_plain_var(declaration: &VariableDeclaration<'_>) -> bool {
    declaration.kind == VariableDeclarationKind::Var && !declaration.declare
}

fn head_mut<'s, 'a>(
    stmt: &'s mut Statement<'a>,
) -> Option<(Site, &'s mut VariableDeclaration<'a>)> {
    let (site, declaration) = match stmt {
        Statement::ForStatement(node) => match &mut node.init {
            Some(ForStatementInit::VariableDeclaration(declaration)) => {
                (Site::ForHead, declaration)
            }
            _ => return None,
        },
        Statement::ForInStatement(node) => match &mut node.left {
            ForStatementLeft::VariableDeclaration(declaration) => (Site::ForInOfHead, declaration),
            _ => return None,
        },
        Statement::ForOfStatement(node) => match &mut node.left {
            ForStatementLeft::VariableDeclaration(declaration) => (Site::ForInOfHead, declaration),
            _ => return None,
        },
        _ => return None,
    };
    is_plain_var(declaration).then_some((site, &mut **declaration))
}

fn loop_scope(stmt: &Statement<'_>) -> Option<ScopeId> {
    match stmt {
        Statement::ForStatement(node) => node.scope_id.get(),
        Statement::ForInStatement(node) => node.scope_id.get(),
        Statement::ForOfStatement(node) => node.scope_id.get(),
        _ => None,
    }
}

fn bindings_clear(scoping: &Scoping, symbol: SymbolId, name: Ident<'_>, scope: ScopeId) -> bool {
    let owner = scoping.symbol_scope_id(symbol);
    for ancestor in scoping.scope_ancestors(scope) {
        if scoping
            .get_binding(ancestor, name)
            .is_some_and(|other| other != symbol)
        {
            return false;
        }
        if ancestor == owner {
            return true;
        }
    }
    false
}

impl Facts<'_> {
    fn reaches_without_closure(&self, from: ScopeId, scope: ScopeId) -> bool {
        let closure = ScopeFlags::Function | ScopeFlags::ClassStaticBlock;
        for ancestor in self.scoping.scope_ancestors(from) {
            if ancestor == scope {
                return true;
            }
            if self.scoping.scope_flags(ancestor).intersects(closure)
                || (!self.class_scopes.is_empty() && self.class_scopes.contains(&ancestor))
            {
                return false;
            }
        }
        false
    }

    fn references_stay_inside(&self, symbol: SymbolId, scope: ScopeId) -> bool {
        self.scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|id| {
                let from = self.scoping.get_reference(*id).scope_id();
                self.reaches_without_closure(from, scope)
            })
    }

    fn is_exact(&self, symbol: SymbolId, name: Ident<'_>, scope: ScopeId, captures: bool) -> bool {
        let scoping = self.scoping;
        let text = name.as_str();
        scoping.symbol_flags(symbol) == SymbolFlags::FunctionScopedVariable
            && scoping.symbol_redeclarations(symbol).is_empty()
            && scoping.symbol_scope_id(symbol) != scoping.root_scope_id()
            && (captures || self.references_stay_inside(symbol, scope))
            && !REFUSED_NAMES.contains(&text)
            && !self.block_functions.contains(&name_hash(text))
            && bindings_clear(scoping, symbol, name, scope)
    }

    fn initialisation_allows(
        &self,
        declarator: &VariableDeclarator<'_>,
        symbol: SymbolId,
        site: Site,
        list_scope: ScopeId,
    ) -> bool {
        match site {
            Site::ForInOfHead => declarator.init.is_none(),
            Site::Statement | Site::ForHead => {
                declarator.init.is_some()
                    || (self.scoping.symbol_scope_id(symbol) == list_scope
                        && !is_constant_binding(self.scoping, symbol))
            }
        }
    }

    fn exact_bindings<'a>(
        &self,
        declaration: &VariableDeclaration<'a>,
        site: Site,
        scope: ScopeId,
        list_scope: ScopeId,
        captures: bool,
    ) -> Option<Bindings<'a>> {
        let exact = |declarator: &VariableDeclarator<'a>| {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let symbol = id.symbol_id.get()?;
            let exact = self.initialisation_allows(declarator, symbol, site, list_scope)
                && self.is_exact(symbol, id.name, scope, captures);
            exact.then_some((symbol, id.name))
        };
        let mut bindings = Vec::new();
        for declarator in &declaration.declarations {
            let binding = exact(declarator)?;
            if bindings.is_empty() {
                bindings.reserve_exact(declaration.declarations.len());
            }
            bindings.push(binding);
        }
        Some(bindings)
    }

    fn one_kind(&self, declaration: &VariableDeclaration<'_>, bindings: &Bindings<'_>) -> bool {
        let constant =
            |(symbol, _): &(SymbolId, Ident<'_>)| is_constant_binding(self.scoping, *symbol);
        let initialised = declaration.declarations.iter().all(|d| d.init.is_some());
        !initialised || bindings.iter().all(constant) || !bindings.iter().any(constant)
    }

    fn statement_candidates<'a>(
        &self,
        stmts: &AVec<'a, Statement<'a>>,
        list_scope: ScopeId,
        captures: bool,
    ) -> Vec<Candidate<'a>> {
        let mut found = Vec::new();
        for (index, stmt) in stmts.iter().enumerate() {
            let Statement::VariableDeclaration(declaration) = stmt else {
                continue;
            };
            if !is_plain_var(declaration) {
                continue;
            }
            if let Some(bindings) = self
                .exact_bindings(
                    declaration,
                    Site::Statement,
                    list_scope,
                    list_scope,
                    captures,
                )
                .filter(|bindings| self.one_kind(declaration, bindings))
            {
                let captured = captures
                    && bindings
                        .iter()
                        .any(|(symbol, _)| !self.references_stay_inside(*symbol, list_scope));
                found.push(Candidate {
                    index,
                    bindings,
                    refused: false,
                    captured,
                });
            }
        }
        found
    }
}

enum Names<'a> {
    One(&'a str),
    Few(Vec<(&'a str, usize)>),
    Many(FxHashMap<&'a str, usize>),
}

impl<'a> Names<'a> {
    fn of(candidates: &[Candidate<'a>]) -> Self {
        let pairs = candidates.iter().enumerate().flat_map(|(at, candidate)| {
            candidate
                .bindings
                .iter()
                .map(move |(_, name)| (name.as_str(), at))
        });
        let count: usize = candidates.iter().map(|c| c.bindings.len()).sum();
        if let [Candidate { bindings, .. }] = candidates {
            if let [(_, name)] = bindings.as_slice() {
                return Names::One(name.as_str());
            }
        }
        if count <= SMALL_NAME_SET {
            let mut few = Vec::with_capacity(count);
            few.extend(pairs);
            Names::Few(few)
        } else {
            let mut many = FxHashMap::with_capacity_and_hasher(count, Default::default());
            many.extend(pairs);
            Names::Many(many)
        }
    }

    fn candidate(&self, name: &str) -> Option<usize> {
        match self {
            Names::One(known) => (*known == name).then_some(0),
            Names::Few(pairs) => pairs
                .iter()
                .find(|(known, _)| *known == name)
                .map(|(_, at)| *at),
            Names::Many(map) => map.get(name).copied(),
        }
    }
}

struct ReadScan<'n, 'a> {
    names: Names<'a>,
    candidates: &'n mut [Candidate<'a>],
    statement: usize,
}

impl<'n, 'a> ReadScan<'n, 'a> {
    fn new(candidates: &'n mut [Candidate<'a>]) -> Self {
        Self {
            names: Names::of(candidates),
            candidates,
            statement: 0,
        }
    }
}

impl<'a> VisitJs<'a> for ReadScan<'_, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(at) = self.names.candidate(it.name.as_str()) else {
            return;
        };
        let statement = self.statement;
        if let Some(candidate) = self.candidates.get_mut(at) {
            candidate.refused |= statement <= candidate.index;
        }
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn mark_earlier_reads<'a>(stmts: &AVec<'a, Statement<'a>>, candidates: &mut [Candidate<'a>]) {
    let last = candidates.last().map_or(0, |candidate| candidate.index);
    let mut scan = ReadScan::new(candidates);
    for (index, stmt) in stmts.iter().enumerate().take(last + 1) {
        scan.statement = index;
        scan.visit_statement(stmt);
    }
}

fn refuse_early_captures<'a>(
    scoping: &Scoping,
    stmts: &AVec<'a, Statement<'a>>,
    candidates: &mut [Candidate<'a>],
) {
    let watched: Vec<(SymbolId, usize)> = candidates
        .iter()
        .filter(|candidate| candidate.captured && !candidate.refused)
        .flat_map(|candidate| {
            candidate
                .bindings
                .iter()
                .map(move |(symbol, _)| (*symbol, candidate.index))
        })
        .collect();
    if watched.is_empty() {
        return;
    }
    let follow = var_capture::captures_follow(scoping, stmts, &watched);
    for candidate in candidates.iter_mut().filter(|candidate| candidate.captured) {
        let late = candidate
            .bindings
            .iter()
            .all(|(symbol, _)| follow.get(symbol).copied().unwrap_or(false));
        candidate.refused |= !late;
    }
}

fn head_reads<'a>(stmt: &Statement<'a>, candidate: &mut [Candidate<'a>; 1]) -> bool {
    let mut scan = ReadScan::new(candidate);
    match stmt {
        Statement::ForStatement(node) => {
            if let Some(ForStatementInit::VariableDeclaration(declaration)) = &node.init {
                scan.visit_variable_declaration(declaration);
            }
        }
        Statement::ForInStatement(node) => scan.visit_expression(&node.right),
        Statement::ForOfStatement(node) => scan.visit_expression(&node.right),
        _ => {}
    }
    candidate[0].refused
}

fn lexical_kind(
    scoping: &Scoping,
    site: Site,
    bindings: &Bindings<'_>,
    declaration: &VariableDeclaration<'_>,
) -> VariableDeclarationKind {
    let initialised =
        site == Site::ForInOfHead || declaration.declarations.iter().all(|d| d.init.is_some());
    let constant = initialised
        && bindings
            .iter()
            .all(|(symbol, _)| is_constant_binding(scoping, *symbol));
    if constant {
        VariableDeclarationKind::Const
    } else {
        VariableDeclarationKind::Let
    }
}

fn flags_of(kind: VariableDeclarationKind) -> SymbolFlags {
    if kind == VariableDeclarationKind::Const {
        SymbolFlags::BlockScopedVariable | SymbolFlags::ConstVariable
    } else {
        SymbolFlags::BlockScopedVariable
    }
}

fn rebind(
    scoping: &mut Scoping,
    (symbol, name): (SymbolId, Ident<'_>),
    scope: ScopeId,
    kind: VariableDeclarationKind,
) {
    let owner = scoping.symbol_scope_id(symbol);
    *scoping.symbol_flags_mut(symbol) = flags_of(kind);
    if owner != scope {
        scoping.move_binding(owner, scope, name);
        scoping.set_symbol_scope_id(symbol, scope);
    }
}

fn make_lexical<'a>(
    declaration: &mut VariableDeclaration<'a>,
    site: Site,
    bindings: &Bindings<'a>,
    scope: ScopeId,
    cx: &mut Cx<'a, '_>,
) {
    let kind = lexical_kind(cx.scoping(), site, bindings, declaration);
    declaration.kind = kind;
    for binding in bindings {
        rebind(cx.scoping_mut(), *binding, scope, kind);
    }
    note_shape(site);
}

impl VarToLet {
    fn clear(&mut self) {
        self.frozen.clear();
        self.class_scopes.clear();
        self.switch_scopes.clear();
        self.block_functions = None;
        self.pending.clear();
        self.bare.clear();
    }

    fn facts<'f>(&'f mut self, scoping: &'f Scoping) -> Facts<'f> {
        let block_functions = self
            .block_functions
            .get_or_insert_with(|| block_function_names(scoping));
        Facts {
            scoping,
            class_scopes: &self.class_scopes,
            block_functions,
        }
    }

    fn convert_statements<'a>(
        &mut self,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> usize {
        let scope = cx.current_scope_id();
        let captures = is_function_scope(cx.scoping(), scope);
        let mut candidates = self
            .facts(cx.scoping())
            .statement_candidates(stmts, scope, captures);
        if candidates.is_empty() {
            return 0;
        }
        mark_earlier_reads(stmts, &mut candidates);
        refuse_early_captures(cx.scoping(), stmts, &mut candidates);
        let mut converted = 0;
        for candidate in candidates {
            if candidate.refused {
                continue;
            }
            if let Some(Statement::VariableDeclaration(declaration)) =
                stmts.get_mut(candidate.index)
            {
                make_lexical(declaration, Site::Statement, &candidate.bindings, scope, cx);
                converted += 1;
            }
        }
        converted
    }

    fn place<'a>(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) -> bool {
        let scope = cx.current_scope_id();
        if !is_function_scope(cx.scoping(), scope) {
            return false;
        }
        let facts = self.facts(cx.scoping());
        let plan = var_placement::plan(facts.scoping, stmts, scope, |name| {
            REFUSED_NAMES.contains(&name) || facts.block_functions.contains(&name_hash(name))
        });
        if plan.is_empty() {
            return false;
        }
        for _ in &plan.converted {
            note_placement(PLACED_AT_FIRST_WRITE);
        }
        for _ in &plan.hoisted {
            note_placement(HOISTED_TO_TOP);
        }
        let hoisted = !plan.hoisted.is_empty();
        let changed = var_placement::apply(stmts, &plan, cx);
        cx.changed_by(changed);
        hoisted
    }

    fn convert_head<'a>(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Some(scope) = loop_scope(stmt) else {
            return false;
        };
        let list_scope = cx.current_scope_id();
        let Some((site, declaration)) = head_mut(stmt) else {
            return false;
        };
        let Some(bindings) =
            self.facts(cx.scoping())
                .exact_bindings(declaration, site, scope, list_scope, false)
        else {
            return false;
        };
        let mut candidate = [Candidate {
            index: 0,
            bindings,
            refused: false,
            captured: false,
        }];
        if head_reads(stmt, &mut candidate) {
            return false;
        }
        let Some((site, declaration)) = head_mut(stmt) else {
            return false;
        };
        make_lexical(declaration, site, &candidate[0].bindings, scope, cx);
        true
    }
}

impl<'a> Pass<'a> for VarToLet {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::EXIT_STATEMENTS
            | Interest::EXIT_VARIABLE_DECLARATION
            | Interest::ENTER_STATEMENT
            | Interest::EXIT_STATEMENT
            | Interest::ENTER_EXPRESSION
            | Interest::EXIT_FUNCTION)
            .on_statements(StatementVariants::of(&WATCHED_STATEMENTS))
            .on_expressions(ExpressionVariants::of(&[
                ExpressionKind::ClassExpression,
                ExpressionKind::FunctionExpression,
            ]))
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.clear();
    }

    fn enter_program(&mut self, _program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.clear();
        let scoping = cx.scoping();
        self.active = scope_hazard(scoping)
            .or_else(|| assumed_hazard(scoping))
            .is_none();
        if self.active {
            self.pending.resize(scoping.scopes_len().div_ceil(64), 0);
            self.bare.resize(scoping.scopes_len().div_ceil(64), 0);
        }
    }

    fn enter_statement(&mut self, node: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        match node {
            Statement::SwitchStatement(switch) => {
                if let Some(scope) = switch.scope_id.get() {
                    self.switch_scopes.insert(scope);
                }
            }
            Statement::ClassDeclaration(class) => {
                if let Some(scope) = class.scope_id.get() {
                    self.class_scopes.insert(scope);
                }
            }
            Statement::FunctionDeclaration(function) => self.frozen.enter(function),
            _ => {}
        }
    }

    fn enter_expression(&mut self, node: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        match node {
            Expression::ClassExpression(class) => {
                if let Some(scope) = class.scope_id.get() {
                    self.class_scopes.insert(scope);
                }
            }
            Expression::FunctionExpression(function) => self.frozen.enter(function),
            _ => {}
        }
    }

    fn exit_function(&mut self, node: &mut Function<'a>, _cx: &mut Cx<'a, '_>) {
        self.frozen.exit(node);
    }

    fn exit_variable_declaration(
        &mut self,
        node: &mut VariableDeclaration<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        if self.active && is_plain_var(node) {
            set_bit(&mut self.pending, cx.current_scope_id().index());
            if let Some(scope) = bare_var_scope(cx.scoping(), node) {
                set_bit(&mut self.bare, scope.index());
            }
        }
    }

    fn exit_statement(&mut self, node: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if self.active && !self.frozen.is_open() && self.convert_head(node, cx) {
            cx.changed();
        }
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if !self.active {
            return;
        }
        let scope = cx.current_scope_id();
        let pending = take_bit(&mut self.pending, scope.index());
        let bare = take_bit(&mut self.bare, scope.index());
        if (!pending && !bare) || self.frozen.is_open() {
            return;
        }
        let hoisted = bare && self.place(stmts, cx);
        if !pending && !hoisted {
            return;
        }
        if !self.switch_scopes.is_empty() && self.switch_scopes.contains(&scope) {
            return;
        }
        let converted = self.convert_statements(stmts, cx);
        cx.changed_by(converted);
    }
}
