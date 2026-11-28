use oxc_allocator::{Box as ABox, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::rename::{is_name_taken, taken_names, Renames, UNIQUE_NAME_SUFFIX_CAP};
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Pass, StatementKind, StatementVariants,
};
use untwist_core::semantic::unbound::is_unbound_name;
use untwist_core::semantic::{may_call_eval, misbound_catch_vars, program_is_strict, scope_hazard};
use untwist_eval::fold::{fold_to_const, Const, FoldOptions};

use crate::constant_test::literal_truthiness;
use crate::merge_rename::{hoist_needs_braces, shadows_dropped, DroppedBranch, Givers};
use crate::notes;

pub const PASS: &str = "dead-code";

type Moved = FxHashMap<ScopeId, Vec<(SymbolId, String)>>;

fn is_string_comparison(expr: &Expression<'_>) -> bool {
    let Expression::BinaryExpression(binary) = expr else {
        return false;
    };
    let operator = matches!(
        binary.operator,
        BinaryOperator::StrictEquality
            | BinaryOperator::Equality
            | BinaryOperator::StrictInequality
            | BinaryOperator::Inequality
    );
    operator
        && matches!(binary.left, Expression::StringLiteral(_))
        && matches!(binary.right, Expression::StringLiteral(_))
}

pub fn opaque_test(expr: &Expression<'_>) -> Option<bool> {
    let (target, negate) = match expr {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            (&unary.argument, true)
        }
        other => (other, false),
    };
    if !is_string_comparison(target) {
        return None;
    }
    let Some(Const::Bool(value)) = fold_to_const(target, &FoldOptions::default()) else {
        return None;
    };
    Some(value != negate)
}

fn collides(name: &str, outer: ScopeId, moved: &Moved, cx: &Cx<'_, '_>) -> bool {
    let ident = Ident::from(name);
    cx.scoping().get_binding(outer, ident).is_some()
        || cx
            .scoping()
            .scope_ancestors(outer)
            .skip(1)
            .any(|ancestor| cx.scoping().get_binding(ancestor, ident).is_some())
        || moved
            .get(&outer)
            .is_some_and(|entries| entries.iter().any(|(_, existing)| existing == name))
}

fn is_available(
    name: &str,
    taken: &FxHashSet<&str>,
    moved: &Moved,
    handed_out: &FxHashSet<String>,
) -> bool {
    !taken.contains(name)
        && !handed_out.contains(name)
        && !moved
            .values()
            .any(|entries| entries.iter().any(|(_, existing)| existing == name))
}

fn fresh_name(
    base: &str,
    scoping: &Scoping,
    moved: &Moved,
    handed_out: &FxHashSet<String>,
) -> Option<String> {
    let first = format!("_{base}");
    let first_free = !is_name_taken(scoping, &first);
    let taken = FxHashSet::default();
    if first_free && is_available(&first, &taken, moved, handed_out) {
        return Some(first);
    }
    let taken = taken_names(scoping);
    (2..=UNIQUE_NAME_SUFFIX_CAP)
        .map(|suffix| format!("_{base}{suffix}"))
        .find(|candidate| is_available(candidate, &taken, moved, handed_out))
}

struct Planner<'p, 'b, 'a> {
    outer: ScopeId,
    moved: &'p Moved,
    handed_out: &'p mut FxHashSet<String>,
    staged: Vec<(SymbolId, Ident<'a>)>,
    givers: Givers<'b, 'a>,
    dropped: &'p FxHashSet<ReferenceId>,
    drop: Option<&'p DroppedBranch>,
}

impl<'a> Planner<'_, '_, 'a> {
    fn relocate(&mut self, name: &str, symbol: SymbolId, cx: &Cx<'a, '_>) -> Option<String> {
        if !collides(name, self.outer, self.moved, cx) {
            return Some(name.to_string());
        }
        if is_unbound_name(name) {
            return None;
        }
        if !self.givers.gives(symbol) {
            return self.rename(symbol, name, cx);
        }
        let taken = self
            .moved
            .get(&self.outer)
            .is_some_and(|entries| entries.iter().any(|(_, existing)| existing == name));
        if taken {
            return None;
        }
        shadows_dropped(name, self.outer, (self.dropped, self.drop), cx.scoping())
            .then(|| name.to_string())
    }

    fn rename(&mut self, symbol: SymbolId, name: &str, cx: &Cx<'a, '_>) -> Option<String> {
        let fresh = fresh_name(name, cx.scoping(), self.moved, self.handed_out)?;
        self.handed_out.insert(fresh.clone());
        self.staged.push((symbol, Ident::from_str_in(&fresh, cx)));
        Some(fresh)
    }

    fn plan(
        &mut self,
        statement: &Statement<'a>,
        cx: &Cx<'a, '_>,
    ) -> Option<Vec<(SymbolId, String)>> {
        let Statement::BlockStatement(block) = statement else {
            return (!declares_function(statement)).then(Vec::new);
        };
        if block.body.iter().any(declares_function) {
            return None;
        }
        let scope_id = block.scope_id.get()?;
        if block.body.iter().any(declares_without_a_symbol) {
            return None;
        }
        let mut introduced = Vec::new();
        let forwarded = self.moved.get(&scope_id).cloned().unwrap_or_default();
        for (symbol, name) in declared_in(scope_id, cx.scoping())
            .into_iter()
            .chain(forwarded)
        {
            let final_name = self.relocate(&name, symbol, cx)?;
            introduced.push((symbol, final_name));
        }
        Some(introduced)
    }
}

fn declares_without_a_symbol(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .any(|id| id.symbol_id.get().is_none()),
        Statement::FunctionDeclaration(function) => function
            .id
            .as_ref()
            .is_some_and(|id| id.symbol_id.get().is_none()),
        Statement::ClassDeclaration(class) => class
            .id
            .as_ref()
            .is_some_and(|id| id.symbol_id.get().is_none()),
        _ => false,
    }
}

fn declared_in(scope: ScopeId, scoping: &Scoping) -> Vec<(SymbolId, String)> {
    let mut declared: Vec<(SymbolId, String)> = scoping
        .get_bindings(scope)
        .iter()
        .map(|(name, symbol)| (*symbol, name.as_str().to_string()))
        .collect();
    declared
        .sort_by_cached_key(|(symbol, name)| (scoping.symbol_span(*symbol).start, name.clone()));
    declared
}

fn commit_merge<'a>(statement: Statement<'a>, cx: &Cx<'a, '_>) -> AVec<'a, Statement<'a>> {
    let Statement::BlockStatement(block) = statement else {
        let mut body = AVec::new_in(cx);
        body.push(statement);
        return body;
    };
    block.unbox().body
}

fn refuse<'a>(
    span: Span,
    test: Expression<'a>,
    consequent: Statement<'a>,
    alternate: Option<Statement<'a>>,
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    cx.note(notes::DEAD_CODE_UNMERGEABLE, Some(span));
    stmts.push(Statement::new_if_statement(
        span, test, consequent, alternate, cx,
    ));
    false
}

#[derive(Default)]
struct Hoisted<'a> {
    names: Vec<(Ident<'a>, Option<SymbolId>)>,
    function: bool,
}

impl<'a> Visit<'a> for Hoisted<'a> {
    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            for declarator in &it.declarations {
                for id in declarator.id.get_binding_identifiers() {
                    if !self.names.iter().any(|(name, _)| *name == id.name) {
                        self.names.push((id.name, id.symbol_id.get()));
                    }
                }
            }
        }
        walk::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        self.function |= it.is_declaration();
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn completes_abruptly(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ReturnStatement(_) | Statement::ThrowStatement(_)
    )
}

fn runs_nothing_hoisted(statement: &Statement<'_>) -> bool {
    if matches!(
        statement,
        Statement::VariableDeclaration(_)
            | Statement::FunctionDeclaration(_)
            | Statement::ClassDeclaration(_)
    ) || statement.is_module_declaration()
    {
        return false;
    }
    hoists_nothing(statement)
}

fn hoists_nothing(statement: &Statement<'_>) -> bool {
    let hoisted = hoisted_by(Some(statement));
    !hoisted.function && hoisted.names.is_empty()
}

fn hoisted_by<'a>(statement: Option<&Statement<'a>>) -> Hoisted<'a> {
    let mut hoisted = Hoisted::default();
    if let Some(statement) = statement {
        hoisted.visit_statement(statement);
    }
    hoisted
}

struct DroppedFunctions<'s, 'a> {
    scoping: &'s Scoping,
    names: Vec<(Ident<'a>, Option<SymbolId>)>,
    unbound: bool,
}

impl<'a> Visit<'a> for DroppedFunctions<'_, 'a> {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let Some(id) = it.id.as_ref().filter(|_| it.is_declaration()) else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            self.unbound = true;
            return;
        };
        let hoisted = self
            .scoping
            .scope_flags(self.scoping.symbol_scope_id(symbol))
            .is_var();
        if hoisted && !self.names.iter().any(|(name, _)| *name == id.name) {
            self.names.push((id.name, Some(symbol)));
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn annex_b_vars<'a>(
    dropped: Option<&Statement<'a>>,
    scoping: &Scoping,
) -> Option<Vec<(Ident<'a>, Option<SymbolId>)>> {
    let mut found = DroppedFunctions {
        scoping,
        names: Vec::new(),
        unbound: false,
    };
    if let Some(statement) = dropped {
        found.visit_statement(statement);
    }
    (!found.unbound).then_some(found.names)
}

#[derive(Default)]
struct References {
    ids: FxHashSet<ReferenceId>,
}

impl<'a> Visit<'a> for References {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(id) = it.reference_id.get() {
            self.ids.insert(id);
        }
    }
}

fn references_in(statement: Option<&Statement<'_>>) -> FxHashSet<ReferenceId> {
    let mut references = References::default();
    if let Some(statement) = statement {
        references.visit_statement(statement);
    }
    references.ids
}

fn seen_elsewhere(symbol: SymbolId, dropped: &FxHashSet<ReferenceId>, scoping: &Scoping) -> bool {
    scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
        || scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .any(|reference| !dropped.contains(reference))
}

fn still_needed<'a>(
    names: Vec<(Ident<'a>, Option<SymbolId>)>,
    dropped: Option<&Statement<'a>>,
    scoping: &Scoping,
) -> Vec<(Ident<'a>, Option<SymbolId>)> {
    if scope_hazard(scoping).is_some() || has_with(scoping) {
        return names;
    }
    let references = references_in(dropped);
    let misbound = if names
        .iter()
        .filter_map(|(_, symbol)| *symbol)
        .any(|symbol| scoping.symbol_flags(symbol).is_catch_variable())
    {
        misbound_catch_vars(scoping)
    } else {
        FxHashSet::default()
    };
    names
        .into_iter()
        .filter(|(_, symbol)| {
            symbol.is_none_or(|symbol| {
                misbound.contains(&symbol) || seen_elsewhere(symbol, &references, scoping)
            })
        })
        .collect()
}

fn var_statement<'a>(
    names: &[(Ident<'a>, Option<SymbolId>)],
    span: Span,
    cx: &Cx<'a, '_>,
) -> Statement<'a> {
    let declarators = AVec::from_iter_in(
        names.iter().map(|&(name, symbol)| {
            let pattern = match symbol {
                Some(symbol) => {
                    BindingPattern::new_binding_identifier_with_symbol_id(SPAN, name, symbol, cx)
                }
                None => BindingPattern::new_binding_identifier(SPAN, name, cx),
            };
            VariableDeclarator::new(SPAN, pattern, None, None, false, cx)
        }),
        cx,
    );
    Statement::new_variable_declaration(span, VariableDeclarationKind::Var, declarators, false, cx)
}

fn hoisted_names<'a>(
    declaration: &VariableDeclaration<'a>,
) -> Option<Vec<(Ident<'a>, Option<SymbolId>)>> {
    if declaration.kind != VariableDeclarationKind::Var {
        return None;
    }
    declaration
        .declarations
        .iter()
        .map(|declarator| match (&declarator.id, &declarator.init) {
            (BindingPattern::BindingIdentifier(id), None) => Some((id.name, id.symbol_id.get())),
            _ => None,
        })
        .collect()
}

fn declares_function(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) => true,
        Statement::LabeledStatement(labeled) => declares_function(&labeled.body),
        _ => false,
    }
}

fn branch_declares_function(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::BlockStatement(block) => block.body.iter().any(declares_function),
        other => declares_function(other),
    }
}

fn keep_braced<'a>(
    branch: Statement<'a>,
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let braced = match branch {
        block @ Statement::BlockStatement(_) => block,
        other => cx.block_of(other),
    };
    stmts.push(braced);
    true
}

fn global_read_within(
    scoping: &Scoping,
    name: &str,
    outer: ScopeId,
    dropped: &FxHashSet<ReferenceId>,
) -> bool {
    let Some(references) = scoping.root_unresolved_references().get(name) else {
        return false;
    };
    references
        .iter()
        .filter(|reference| !dropped.contains(*reference))
        .any(|reference| {
            let site = scoping.get_reference(*reference).scope_id();
            scoping.scope_ancestors(site).any(|scope| scope == outer)
        })
}

fn has_with(scoping: &Scoping) -> bool {
    scoping
        .scope_descendants_from_root()
        .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
}

#[derive(Clone, Copy)]
struct Decision {
    value: bool,
    literal: bool,
}

fn starts_with_string_statement(statement: &Statement<'_>) -> bool {
    let first = match statement {
        Statement::BlockStatement(block) => block.body.first(),
        other => Some(other),
    };
    matches!(
        first,
        Some(Statement::ExpressionStatement(node))
            if matches!(node.expression, Expression::StringLiteral(_))
    )
}

fn is_string_statement(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ExpressionStatement(node) if matches!(node.expression, Expression::StringLiteral(_))
    )
}

#[derive(Default)]
pub struct DeadCode<'a> {
    moved: Moved,
    handed_out: FxHashSet<String>,
    renames: Renames<'a>,
    strict: bool,
    with: Option<bool>,
    eval: Option<bool>,
    unsafe_lane: bool,
    armed: usize,
    guarded: Vec<Span>,
    callees: Vec<Span>,
    kept_vars: FxHashSet<Span>,
    fresh_vars: FxHashSet<Span>,
    dropped: FxHashSet<ReferenceId>,
    drop: Option<DroppedBranch>,
    known_bits: crate::known_bits::KnownBits,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DeadCode::default())
}

impl<'a> DeadCode<'a> {
    fn decide(&mut self, expr: &Expression<'_>, cx: &Cx<'a, '_>) -> Option<Decision> {
        let cached = std::cell::Cell::new(self.with);
        let scoping = cx.scoping();
        let is_global_undefined = |id: &IdentifierReference<'_>| {
            let with = cached.get().unwrap_or_else(|| {
                let found = has_with(scoping);
                cached.set(Some(found));
                found
            });
            !with
                && id
                    .reference_id
                    .get()
                    .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
        };
        let literal = literal_truthiness(expr, &is_global_undefined);
        self.with = cached.get();
        match literal {
            Some(value) => Some(Decision {
                value,
                literal: true,
            }),
            None => opaque_test(expr)
                .or_else(|| self.known_bits.decide(expr, cx))
                .map(|value| Decision {
                    value,
                    literal: false,
                }),
        }
    }

    fn push_kept_vars(
        &mut self,
        names: &[(Ident<'a>, Option<SymbolId>)],
        span: Span,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &Cx<'a, '_>,
    ) {
        if names.is_empty() {
            return;
        }
        self.fresh_vars.insert(span);
        stmts.push(var_statement(names, span, cx));
    }

    fn prune_kept_vars(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &Cx<'a, '_>) -> usize {
        let mut pruned = 0usize;
        stmts.retain_mut(|statement| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return true;
            };
            if !self.kept_vars.contains(&declaration.span) {
                return true;
            }
            let Some(names) = hoisted_names(declaration) else {
                return true;
            };
            let needed = still_needed(names, None, cx.scoping());
            if needed.len() == declaration.declarations.len() {
                return true;
            }
            pruned += 1;
            if needed.is_empty() {
                self.kept_vars.remove(&declaration.span);
                return false;
            }
            declaration.declarations.retain(|declarator| {
                declarator
                    .id
                    .get_binding_identifier()
                    .is_some_and(|id| needed.iter().any(|(name, _)| *name == id.name))
            });
            true
        });
        pruned
    }

    fn fold_dead_loop(
        &mut self,
        node: ABox<'a, WhileStatement<'a>>,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> bool {
        let hoisted = hoisted_by(Some(&node.body));
        if hoisted.function && !self.strict {
            cx.note(notes::DEAD_CODE_DISCARDED_FUNCTION, Some(node.span));
            stmts.push(Statement::WhileStatement(node));
            return false;
        }
        let names = still_needed(hoisted.names, Some(&node.body), cx.scoping());
        self.push_kept_vars(&names, node.body.span(), stmts, cx);
        true
    }

    fn fold_soundly(
        &mut self,
        node: ABox<'a, IfStatement<'a>>,
        value: bool,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> bool {
        let dropped = if value {
            node.alternate.as_ref()
        } else {
            Some(&node.consequent)
        };
        let hoisted = hoisted_by(dropped);
        if hoisted.function && !self.strict {
            cx.note(notes::DEAD_CODE_DISCARDED_FUNCTION, Some(node.span));
            stmts.push(Statement::IfStatement(node));
            return false;
        }
        let names = still_needed(hoisted.names, dropped, cx.scoping());
        let kept_span = dropped.map_or(node.span, GetSpan::span);
        self.dropped.extend(references_in(dropped));
        let IfStatement {
            consequent,
            alternate,
            ..
        } = node.unbox();
        self.push_kept_vars(&names, kept_span, stmts, cx);
        let (kept, dropped) = match value {
            true => (Some(consequent), alternate.as_ref()),
            false => (alternate, Some(&consequent)),
        };
        self.drop = DroppedBranch::of(dropped);
        match kept {
            None => {}
            Some(block @ Statement::BlockStatement(_)) => self.keep_block(block, outer, stmts, cx),
            Some(function @ Statement::FunctionDeclaration(_)) => {
                let braced = cx.block_of(function);
                stmts.push(braced);
            }
            Some(other) => stmts.push(other),
        }
        true
    }

    fn keep_block(
        &mut self,
        block: Statement<'a>,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) {
        if !self.merges_soundly(&block, outer, cx) {
            stmts.push(block);
            return;
        }
        if let Err(block) = self.try_merge(block, outer, stmts, cx) {
            stmts.push(block);
        }
    }

    fn merges_soundly(&mut self, block: &Statement<'a>, outer: ScopeId, cx: &Cx<'a, '_>) -> bool {
        let Statement::BlockStatement(block) = block else {
            return true;
        };
        let Some(scope) = block.scope_id.get() else {
            return false;
        };
        let scoping = cx.scoping();
        let own = scoping.get_bindings(scope);
        let forwarded = self.moved.get(&scope);
        if own.is_empty() && forwarded.is_none_or(Vec::is_empty) {
            return true;
        }
        if outer == scoping.root_scope_id()
            || *self.eval.get_or_insert_with(|| may_call_eval(scoping))
            || *self.with.get_or_insert_with(|| has_with(scoping))
        {
            return false;
        }
        own.iter()
            .map(|(name, symbol)| (name.as_str(), *symbol))
            .chain(
                forwarded
                    .into_iter()
                    .flatten()
                    .map(|(symbol, name)| (name.as_str(), *symbol)),
            )
            .all(|(name, symbol)| {
                !scoping.symbol_flags(symbol).is_function()
                    && !global_read_within(scoping, name, outer, &self.dropped)
            })
    }

    fn merge_branch(
        &mut self,
        node: IfStatement<'a>,
        span: Span,
        value: bool,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> bool {
        let IfStatement {
            test,
            consequent,
            alternate,
            ..
        } = node;
        let dropped_branch = if value {
            alternate.as_ref()
        } else {
            Some(&consequent)
        };
        let dropped = references_in(dropped_branch);
        self.drop = DroppedBranch::of(dropped_branch);
        self.dropped.extend(dropped.iter().copied());
        if value {
            match self.merge_or_keep(consequent, outer, stmts, cx) {
                Ok(()) => true,
                Err(consequent) if branch_declares_function(&consequent) => {
                    keep_braced(consequent, stmts, cx)
                }
                Err(consequent) => {
                    self.restore_dropped(&dropped);
                    refuse(span, test, consequent, alternate, stmts, cx)
                }
            }
        } else {
            let Some(branch) = alternate else {
                return true;
            };
            match self.merge_or_keep(branch, outer, stmts, cx) {
                Ok(()) => true,
                Err(branch) if branch_declares_function(&branch) => keep_braced(branch, stmts, cx),
                Err(branch) => {
                    self.restore_dropped(&dropped);
                    refuse(span, test, consequent, Some(branch), stmts, cx)
                }
            }
        }
    }

    fn restore_dropped(&mut self, dropped: &FxHashSet<ReferenceId>) {
        self.dropped
            .retain(|reference| !dropped.contains(reference));
    }

    fn merge_or_keep(
        &mut self,
        branch: Statement<'a>,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> Result<(), Statement<'a>> {
        if self.merges_soundly(&branch, outer, cx) {
            return self.try_merge(branch, outer, stmts, cx);
        }
        keep_braced(branch, stmts, cx);
        Ok(())
    }

    fn try_merge(
        &mut self,
        branch: Statement<'a>,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> Result<(), Statement<'a>> {
        let mut planner = Planner {
            outer,
            moved: &self.moved,
            handed_out: &mut self.handed_out,
            staged: Vec::new(),
            givers: Givers::new(&branch, cx),
            dropped: &self.dropped,
            drop: self.drop.as_ref(),
        };
        let planned = planner.plan(&branch, cx);
        let staged = planner.staged;
        let renamed: Vec<SymbolId> = staged.iter().map(|(symbol, _)| *symbol).collect();
        match planned {
            Some(_) if hoist_needs_braces(&branch, &renamed, cx.scoping()) => {
                keep_braced(branch, stmts, cx);
                Ok(())
            }
            Some(introduced) => {
                self.moved.entry(outer).or_default().extend(introduced);
                for (symbol, name) in staged {
                    self.renames.insert(symbol, name);
                }
                stmts.extend(commit_merge(branch, cx));
                Ok(())
            }
            None => Err(branch),
        }
    }

    fn brace_decided_alternate(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        let mut current = stmt;
        while let Statement::IfStatement(node) = current {
            let decided = match &node.alternate {
                Some(Statement::IfStatement(inner)) => self.decide(&inner.test, cx).is_some(),
                _ => return,
            };
            if decided {
                if let Some(alternate) = node.alternate.take() {
                    node.alternate = Some(cx.block_of(alternate));
                    cx.changed();
                }
                return;
            }
            let Some(next) = node.alternate.as_mut() else {
                return;
            };
            current = next;
        }
    }

    fn decision_of_statement(
        &mut self,
        statement: &Statement<'a>,
        cx: &Cx<'a, '_>,
    ) -> Option<Decision> {
        match statement {
            Statement::IfStatement(node) => self.decide(&node.test, cx),
            Statement::WhileStatement(node) => self
                .decide(&node.test, cx)
                .filter(|decision| decision.literal && !decision.value),
            _ => None,
        }
    }

    fn is_unfoldable_result(
        &mut self,
        chosen: &Expression<'a>,
        (guarded, callee): (bool, bool),
        cx: &Cx<'a, '_>,
    ) -> bool {
        match chosen.without_parentheses() {
            Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_)
            | Expression::ChainExpression(_) => {
                guarded || (callee && !crate::mutants::dead_member_callees_folded())
            }
            Expression::Identifier(id) => {
                guarded
                    || (callee && id.name == "eval")
                    || *self.with.get_or_insert_with(|| has_with(cx.scoping()))
            }
            _ => false,
        }
    }

    fn expression_choice(&mut self, expr: &Expression<'a>, cx: &Cx<'a, '_>) -> Option<bool> {
        let guarded = match self.guarded.iter().rposition(|span| *span == expr.span()) {
            Some(index) => {
                self.guarded.remove(index);
                true
            }
            None => false,
        };
        let callee = match self.callees.iter().rposition(|span| *span == expr.span()) {
            Some(index) => {
                self.callees.remove(index);
                true
            }
            None => false,
        };
        let guarded = (guarded, callee);
        match expr {
            Expression::ConditionalExpression(conditional) => {
                let value = self.decide(&conditional.test, cx)?.value;
                let chosen = if value {
                    &conditional.consequent
                } else {
                    &conditional.alternate
                };
                (!self.is_unfoldable_result(chosen, guarded, cx)).then_some(value)
            }
            Expression::LogicalExpression(logical) => {
                let decision = self.decide(&logical.left, cx)?;
                let left_wins = match logical.operator {
                    LogicalOperator::And => !decision.value,
                    LogicalOperator::Or => decision.value,
                    LogicalOperator::Coalesce => return None,
                };
                (left_wins || !self.is_unfoldable_result(&logical.right, guarded, cx))
                    .then_some(left_wins)
            }
            _ => None,
        }
    }

    fn rewrite_statements(
        &mut self,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> usize {
        let mut hits = 0usize;
        for statement in stmts.iter() {
            hits += usize::from(self.decision_of_statement(statement, cx).is_some());
        }
        self.armed = self.armed.saturating_sub(hits);
        if hits == 0 {
            return 0;
        }
        let outer = cx.current_scope_id();
        let taken: Vec<Statement<'a>> = stmts.drain(..).collect();
        let mut changes = 0usize;
        let mut unreachable = false;
        for statement in taken {
            if unreachable && runs_nothing_hoisted(&statement) {
                changes += 1;
                continue;
            }
            let mut next = Some(statement);
            while let Some(statement) = next.take() {
                let Some(decision) = self.decision_of_statement(&statement, cx) else {
                    stmts.push(statement);
                    break;
                };
                let before = stmts.len();
                let folded = match statement {
                    Statement::WhileStatement(node) => self.fold_dead_loop(node, stmts, cx),
                    Statement::IfStatement(node) => {
                        self.fold_branch(node, decision, outer, stmts, cx)
                    }
                    other => {
                        stmts.push(other);
                        false
                    }
                };
                if folded {
                    changes += 1;
                    unreachable |=
                        stmts.len() > before && stmts.last().is_some_and(completes_abruptly);
                    next = (stmts.len() > before)
                        .then(|| self.next_link(stmts, cx))
                        .flatten();
                }
            }
        }
        changes
    }

    fn next_link(
        &mut self,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &Cx<'a, '_>,
    ) -> Option<Statement<'a>> {
        let Some(Statement::IfStatement(node)) = stmts.last() else {
            return None;
        };
        let decision = self.decide(&node.test, cx)?;
        if cx.opts.unsafe_transforms && !decision.literal {
            return None;
        }
        let dropped = if decision.value {
            node.alternate.as_ref()
        } else {
            Some(&node.consequent)
        };
        let hoisted = hoisted_by(dropped);
        if hoisted.function || !hoisted.names.is_empty() {
            return None;
        }
        stmts.pop()
    }

    fn settle_identity_inits(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        for stmt in stmts.iter_mut() {
            let Statement::VariableDeclaration(declaration) = stmt else {
                continue;
            };
            for declarator in declaration.declarations.iter_mut() {
                let Some(init) = declarator.init.as_mut() else {
                    continue;
                };
                if let Some(value) = self.known_bits.identity(init, cx) {
                    *init = Expression::new_boolean_literal(init.span(), value, cx);
                    cx.changed();
                }
            }
        }
    }

    fn settle_right(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let mut changed = false;
        loop {
            let test = match &*stmt {
                Statement::IfStatement(node) => &node.test,
                Statement::WhileStatement(node) => &node.test,
                _ => return changed,
            };
            let Expression::LogicalExpression(logical) = test else {
                return changed;
            };
            let absorbing = match logical.operator {
                LogicalOperator::And => false,
                LogicalOperator::Or => true,
                LogicalOperator::Coalesce => return changed,
            };
            let Some(decision) = self.decide(&logical.right, cx) else {
                return changed;
            };
            let neutral = decision.value != absorbing;
            let step = if neutral || crate::mutants::settled_rights_dropped_anywhere() {
                self.drop_right(stmt, cx)
            } else {
                self.absorb_right(stmt, absorbing, cx)
            };
            if !step {
                return changed;
            }
            changed = true;
            if !neutral {
                return changed;
            }
        }
    }

    fn drop_right(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Statement::IfStatement(node) = stmt else {
            return false;
        };
        let Expression::LogicalExpression(logical) = &mut node.test else {
            return false;
        };
        let left = logical.left.take_in(cx);
        node.test = left;
        true
    }

    fn absorb_right(
        &mut self,
        stmt: &mut Statement<'a>,
        absorbing: bool,
        cx: &mut Cx<'a, '_>,
    ) -> bool {
        let removable = match &*stmt {
            Statement::WhileStatement(node) if !absorbing => Some(hoists_nothing(&node.body)),
            Statement::IfStatement(node) if !absorbing && node.alternate.is_none() => {
                Some(hoists_nothing(&node.consequent))
            }
            Statement::IfStatement(_) => None,
            _ => return false,
        };
        if removable == Some(false) {
            return false;
        }
        let span = stmt.span();
        let test = match stmt {
            Statement::IfStatement(node) => &mut node.test,
            Statement::WhileStatement(node) => &mut node.test,
            _ => return false,
        };
        let Expression::LogicalExpression(logical) = test else {
            return false;
        };
        let sequence = logical.span;
        let left = logical.left.take_in(cx);
        if removable == Some(true) {
            *stmt = Statement::new_expression_statement(span, left, cx);
            return true;
        }
        let mut expressions = AVec::with_capacity_in(2, cx);
        expressions.push(left);
        expressions.push(Expression::new_boolean_literal(SPAN, absorbing, cx));
        *test = Expression::new_sequence_expression(sequence, expressions, cx);
        true
    }

    fn drop_settled_right(&mut self, test: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Expression::LogicalExpression(logical) = test else {
            return false;
        };
        let neutral = match logical.operator {
            LogicalOperator::And => true,
            LogicalOperator::Or => false,
            LogicalOperator::Coalesce => return false,
        };
        let settled = self.decide(&logical.right, cx).is_some_and(|decision| {
            decision.value == neutral || crate::mutants::settled_rights_dropped_anywhere()
        });
        if !settled {
            return false;
        }
        let left = logical.left.take_in(cx);
        *test = left;
        true
    }

    fn fold_branch(
        &mut self,
        node: ABox<'a, IfStatement<'a>>,
        decision: Decision,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> bool {
        let taken = if decision.value {
            Some(&node.consequent)
        } else {
            node.alternate.as_ref()
        };
        let hazard = taken.is_some_and(starts_with_string_statement)
            && stmts.iter().all(is_string_statement);
        if hazard {
            stmts.push(Statement::IfStatement(node));
            return false;
        }
        let span = node.span;
        let dropped = if decision.value {
            node.alternate.as_ref()
        } else {
            Some(&node.consequent)
        };
        let hoisted = hoisted_by(dropped);
        let annex_b = if self.strict || !hoisted.function {
            Some(Vec::new())
        } else {
            annex_b_vars(dropped, cx.scoping())
        };
        let annex_b = annex_b.map(|functions| {
            let mut names = hoisted.names;
            for (name, symbol) in functions {
                if !names.iter().any(|(known, _)| *known == name) {
                    names.push((name, symbol));
                }
            }
            names
        });
        if let (true, false, Some(names)) = (cx.opts.unsafe_transforms, decision.literal, annex_b) {
            let names = still_needed(names, dropped, cx.scoping());
            let kept_span = dropped.map_or(span, GetSpan::span);
            self.push_kept_vars(&names, kept_span, stmts, cx);
            self.merge_branch(node.unbox(), span, decision.value, outer, stmts, cx)
        } else {
            self.fold_soundly(node, decision.value, outer, stmts, cx)
        }
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::ConditionalExpression,
    ExpressionKind::LogicalExpression,
    ExpressionKind::UnaryExpression,
    ExpressionKind::CallExpression,
    ExpressionKind::TaggedTemplateExpression,
]);

const ACTED_ON_STATEMENTS: StatementVariants =
    StatementVariants::of(&[StatementKind::IfStatement, StatementKind::WhileStatement]);

impl<'a> Pass<'a> for DeadCode<'a> {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::ENTER_EXPRESSION
            | Interest::EXIT_STATEMENTS
            | Interest::EXIT_STATEMENT
            | Interest::EXIT_EXPRESSION
            | Interest::EXIT_PROGRAM)
            .on_expressions(ACTED_ON_EXPRESSIONS)
            .on_statements(ACTED_ON_STATEMENTS)
    }

    fn retired(&self) -> Interest {
        if self.unsafe_lane {
            Interest::EXIT_STATEMENT
        } else {
            Interest::NONE
        }
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if self.settle_right(stmt, cx) {
            cx.changed();
        }
        if self.decision_of_statement(stmt, cx).is_some() {
            self.armed += 1;
        }
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.known_bits.identities_ready() {
            self.settle_identity_inits(stmts, cx);
        }
        if !self.kept_vars.is_empty() {
            let pruned = self.prune_kept_vars(stmts, cx);
            cx.changed_by(pruned);
        }
        for stmt in stmts.iter_mut() {
            self.brace_decided_alternate(stmt, cx);
        }
        if self.unsafe_lane {
            for stmt in stmts.iter_mut() {
                if self.settle_right(stmt, cx) {
                    cx.changed();
                }
            }
        }
        if !self.unsafe_lane && self.armed == 0 {
            return;
        }
        let changes = self.rewrite_statements(stmts, cx);
        cx.changed_by(changes);
    }

    fn enter_expression(&mut self, expr: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        let called = match &*expr {
            Expression::CallExpression(call) => Some(&call.callee),
            Expression::TaggedTemplateExpression(tagged) => Some(&tagged.tag),
            _ => None,
        };
        if let Some(callee) = called.map(Expression::without_parentheses) {
            if matches!(
                callee,
                Expression::ConditionalExpression(_) | Expression::LogicalExpression(_)
            ) {
                self.callees.push(callee.span());
            }
            return;
        }
        let Expression::UnaryExpression(unary) = &*expr else {
            return;
        };
        if !matches!(
            unary.operator,
            UnaryOperator::Typeof | UnaryOperator::Delete
        ) {
            return;
        }
        let argument = unary.argument.without_parentheses();
        if matches!(
            argument,
            Expression::ConditionalExpression(_) | Expression::LogicalExpression(_)
        ) {
            self.guarded.push(argument.span());
        }
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if let Expression::LogicalExpression(outer) = expr {
            let same = matches!(&outer.left, Expression::LogicalExpression(inner) if inner.operator == outer.operator);
            if same && self.drop_settled_right(&mut outer.left, cx) {
                cx.changed();
            }
        }
        let Some(keep_consequent) = self.expression_choice(expr, cx) else {
            return;
        };
        let placeholder = Expression::new_null_literal(SPAN, cx);
        let chosen = match std::mem::replace(expr, placeholder) {
            Expression::ConditionalExpression(conditional) => {
                let conditional = conditional.unbox();
                if keep_consequent {
                    conditional.consequent
                } else {
                    conditional.alternate
                }
            }
            Expression::LogicalExpression(logical) => {
                let logical = logical.unbox();
                if keep_consequent {
                    logical.left
                } else {
                    logical.right
                }
            }
            other => other,
        };
        *expr = cx.unnamed_in_slot(chosen);
        cx.changed();
    }

    fn reset(&mut self) {
        self.moved = FxHashMap::default();
        self.handed_out = FxHashSet::default();
        self.renames = Renames::new();
        self.with = None;
        self.eval = None;
        self.guarded = Vec::new();
        self.callees = Vec::new();
    }

    fn enter_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.strict = program_is_strict(node);
        self.unsafe_lane = cx.opts.unsafe_transforms;
        self.armed = 0;
        self.dropped.clear();
        let with = &mut self.with;
        self.known_bits.learn(node, cx, || {
            *with.get_or_insert_with(|| has_with(cx.scoping()))
        });
        let fresh = std::mem::take(&mut self.fresh_vars);
        self.kept_vars.extend(fresh);
    }

    fn exit_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let renames = std::mem::take(&mut self.renames);
        renames.apply(node, cx.scoping(), &*cx);
    }
}
