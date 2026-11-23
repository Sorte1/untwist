use oxc_allocator::{Allocator, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use std::cell::OnceCell;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::shorthand::walk_object_property_keeping_key;
use untwist_core::pass::Cx;
use untwist_core::semantic::may_call_eval;

use super::{object_declaration, single_declarator};

pub(super) struct AliasFold<'a> {
    alias_at: usize,
    object: SymbolId,
    alias: SymbolId,
    name: Ident<'a>,
}

fn settled(symbol: SymbolId, scoping: &Scoping) -> bool {
    !scoping.symbol_is_mutated(symbol)
        && scoping.symbol_redeclarations(symbol).is_empty()
        && scoping.symbol_scope_id(symbol) != scoping.root_scope_id()
}

fn var_scope(scoping: &Scoping, scope: ScopeId) -> Option<ScopeId> {
    scoping
        .scope_ancestors(scope)
        .find(|scope| scoping.scope_flags(*scope).is_var())
}

fn is_block_function(scoping: &Scoping, symbol: SymbolId) -> bool {
    let flags = scoping.symbol_flags(symbol);
    flags.contains(SymbolFlags::Function)
        && !flags.contains(SymbolFlags::FunctionExpression)
        && !scoping
            .scope_flags(scoping.symbol_scope_id(symbol))
            .is_var()
}

#[derive(Default)]
pub(super) struct BlockFunctions(OnceCell<FxHashSet<(String, Option<ScopeId>)>>);

impl BlockFunctions {
    fn index(&self, scoping: &Scoping) -> &FxHashSet<(String, Option<ScopeId>)> {
        self.0.get_or_init(|| {
            scoping
                .symbol_ids()
                .filter(|symbol| is_block_function(scoping, *symbol))
                .map(|symbol| home_of(scoping, symbol))
                .collect()
        })
    }

    fn may_write(&self, symbol: SymbolId, scoping: &Scoping) -> bool {
        !crate::mutants::annex_b_writes_trusted()
            && self.index(scoping).contains(&home_of(scoping, symbol))
    }
}

fn home_of(scoping: &Scoping, symbol: SymbolId) -> (String, Option<ScopeId>) {
    (
        scoping.symbol_name(symbol).to_owned(),
        var_scope(scoping, scoping.symbol_scope_id(symbol)),
    )
}

fn is_var(stmt: &Statement<'_>) -> Option<bool> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    match declaration.kind {
        VariableDeclarationKind::Var => Some(true),
        VariableDeclarationKind::Let | VariableDeclarationKind::Const => Some(false),
        _ => None,
    }
}

fn same_kind(object: &Statement<'_>, alias: &Statement<'_>) -> bool {
    matches!((is_var(object), is_var(alias)), (Some(one), Some(other)) if one == other)
}

fn has_with(scoping: &Scoping) -> bool {
    scoping
        .scope_descendants_from_root()
        .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
}

fn reads_see_the_object(
    alias: SymbolId,
    object: SymbolId,
    name: Ident<'_>,
    scoping: &Scoping,
) -> bool {
    scoping
        .get_resolved_reference_ids(alias)
        .iter()
        .all(|&reference| {
            let reference = scoping.get_reference(reference);
            reference.is_read()
                && !reference.is_write()
                && scoping.find_binding(reference.scope_id(), name) == Some(object)
        })
}

pub(super) fn opaque_scope(scoping: &Scoping) -> bool {
    has_with(scoping) || may_call_eval(scoping)
}

pub(super) fn plan_alias<'a>(
    stmts: &[Statement<'a>],
    (index, alias_at): (usize, usize),
    scoping: &Scoping,
    (extended, others_required): (bool, bool),
    block_functions: &BlockFunctions,
) -> Option<AliasFold<'a>> {
    let declared = object_declaration(stmts.get(index)?, extended)?;
    let (alias, init) = single_declarator(stmts.get(alias_at)?)?;
    if init.and_then(identifier_name) != Some(declared.name) {
        return None;
    }
    if !same_kind(&stmts[index], &stmts[alias_at]) {
        return None;
    }
    let alias = alias.symbol_id.get()?;
    let object = declared.symbol;
    if !settled(object, scoping) || !settled(alias, scoping) {
        return None;
    }
    if others_required && scoping.get_resolved_reference_ids(object).len() < 2 {
        return None;
    }
    let (id, _) = single_declarator(&stmts[index])?;
    let name = id.name;
    let fold = AliasFold {
        alias_at,
        object,
        alias,
        name,
    };
    let lexical = is_var(&stmts[alias_at]) == Some(false);
    let follows = reads_see_the_object(alias, object, name, scoping)
        && all_follow(stmts, &fold, lexical, scoping);
    (follows
        && !block_functions.may_write(alias, scoping)
        && !block_functions.may_write(object, scoping))
    .then_some(fold)
}

struct Redirect<'s, 'a> {
    scoping: &'s mut Scoping,
    targets: FxHashSet<ReferenceId>,
    fold: &'s AliasFold<'a>,
    alloc: &'a Allocator,
    redirected: usize,
}

impl<'a> VisitMut<'a> for Redirect<'_, 'a> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        if !self.targets.contains(&reference) {
            return;
        }
        it.name = self.fold.name;
        self.scoping
            .get_reference_mut(reference)
            .set_symbol_id(self.fold.object);
        self.scoping
            .delete_resolved_reference(self.fold.alias, reference);
        self.scoping
            .add_resolved_reference(self.fold.object, reference);
        self.redirected += 1;
    }

    fn visit_object_property(&mut self, it: &mut ObjectProperty<'a>) {
        let alloc = self.alloc;
        walk_object_property_keeping_key(self, it, alloc);
    }
}

struct Found<'s> {
    targets: &'s FxHashSet<ReferenceId>,
    found: usize,
    hoisted_depth: usize,
    in_hoisted: bool,
}

impl<'a> Visit<'a> for Found<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let hoisted = usize::from(it.is_declaration());
        self.hoisted_depth += hoisted;
        walk::walk_function(self, it, flags);
        self.hoisted_depth -= hoisted;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it
            .reference_id
            .get()
            .is_some_and(|reference| self.targets.contains(&reference))
        {
            self.found += 1;
            self.in_hoisted |= self.hoisted_depth > 0;
        }
    }
}

fn alias_targets(fold: &AliasFold<'_>, scoping: &Scoping) -> FxHashSet<ReferenceId> {
    scoping
        .get_resolved_reference_ids(fold.alias)
        .iter()
        .copied()
        .collect()
}

fn all_follow(
    stmts: &[Statement<'_>],
    fold: &AliasFold<'_>,
    lexical: bool,
    scoping: &Scoping,
) -> bool {
    let targets = alias_targets(fold, scoping);
    let mut found = Found {
        targets: &targets,
        found: 0,
        hoisted_depth: 0,
        in_hoisted: false,
    };
    for stmt in stmts.iter().skip(fold.alias_at + 1) {
        found.visit_statement(stmt);
    }
    found.found == targets.len() && !(lexical && found.in_hoisted)
}

fn commit<'a>(stmts: &mut AVec<'a, Statement<'a>>, fold: &AliasFold<'a>, cx: &mut Cx<'a, '_>) {
    let targets = alias_targets(fold, cx.scoping());
    let alloc = cx.allocator();
    let mut redirect = Redirect {
        scoping: cx.scoping_mut(),
        targets,
        fold,
        alloc,
        redirected: 0,
    };
    for stmt in stmts.iter_mut().skip(fold.alias_at + 1) {
        redirect.visit_statement(stmt);
    }
    let removed = stmts.remove(fold.alias_at);
    let Some((_, Some(Expression::Identifier(init)))) = single_declarator(&removed) else {
        return;
    };
    if let Some(reference) = init.reference_id.get() {
        cx.scoping_mut()
            .delete_resolved_reference(fold.object, reference);
    }
}

pub(super) fn fold_object_aliases<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
    block_functions: &BlockFunctions,
) -> usize {
    let scoping = cx.scoping();
    let extended = cx.opts.extended;
    let Some(fold) = (0..stmts.len()).find_map(|index| {
        plan_alias(
            stmts,
            (index, index + 1),
            scoping,
            (extended, true),
            block_functions,
        )
    }) else {
        return 0;
    };
    if opaque_scope(scoping) {
        return 0;
    }
    commit(stmts, &fold, cx);
    1
}
