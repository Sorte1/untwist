use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::identifier_name;
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
    (reads_see_the_object(alias, object, name, scoping) && all_follow(stmts, &fold, scoping))
        .then_some(fold)
}

struct Redirect<'s, 'a> {
    scoping: &'s mut Scoping,
    targets: FxHashSet<ReferenceId>,
    fold: &'s AliasFold<'a>,
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
}

struct Found<'s> {
    targets: &'s FxHashSet<ReferenceId>,
    found: usize,
}

impl<'a> Visit<'a> for Found<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it
            .reference_id
            .get()
            .is_some_and(|reference| self.targets.contains(&reference))
        {
            self.found += 1;
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

fn all_follow(stmts: &[Statement<'_>], fold: &AliasFold<'_>, scoping: &Scoping) -> bool {
    let targets = alias_targets(fold, scoping);
    let mut found = Found {
        targets: &targets,
        found: 0,
    };
    for stmt in stmts.iter().skip(fold.alias_at + 1) {
        found.visit_statement(stmt);
    }
    found.found == targets.len()
}

fn commit<'a>(stmts: &mut AVec<'a, Statement<'a>>, fold: &AliasFold<'a>, cx: &mut Cx<'a, '_>) {
    let targets = alias_targets(fold, cx.scoping());
    let mut redirect = Redirect {
        scoping: cx.scoping_mut(),
        targets,
        fold,
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
) -> usize {
    let scoping = cx.scoping();
    let extended = cx.opts.extended;
    let Some(fold) = (0..stmts.len())
        .find_map(|index| plan_alias(stmts, (index, index + 1), scoping, (extended, true)))
    else {
        return 0;
    };
    if opaque_scope(scoping) {
        return 0;
    }
    commit(stmts, &fold, cx);
    1
}
