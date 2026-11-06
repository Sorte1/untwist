use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::{Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::rename::{is_name_taken, Renames, UNIQUE_NAME_SUFFIX_CAP};
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Pass, StatementKind, StatementVariants,
};
use untwist_core::semantic::{program_is_strict, scope_hazard};
use untwist_eval::fold::{fold_to_const, Const, FoldOptions};

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
    scoping: &Scoping,
    moved: &Moved,
    handed_out: &FxHashSet<String>,
) -> bool {
    !is_name_taken(scoping, name)
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
    if is_available(&first, scoping, moved, handed_out) {
        return Some(first);
    }
    (2..=UNIQUE_NAME_SUFFIX_CAP)
        .map(|suffix| format!("_{base}{suffix}"))
        .find(|candidate| is_available(candidate, scoping, moved, handed_out))
}

fn relocate<'a>(
    name: &str,
    symbol: SymbolId,
    outer: ScopeId,
    moved: &Moved,
    handed_out: &mut FxHashSet<String>,
    staged: &mut Vec<(SymbolId, Ident<'a>)>,
    cx: &Cx<'a, '_>,
) -> Option<String> {
    if !collides(name, outer, moved, cx) {
        return Some(name.to_string());
    }
    let fresh = fresh_name(name, cx.scoping(), moved, handed_out)?;
    handed_out.insert(fresh.clone());
    staged.push((symbol, Ident::from_str_in(&fresh, cx)));
    Some(fresh)
}

fn plan_merge<'a>(
    statement: &Statement<'a>,
    outer: ScopeId,
    moved: &Moved,
    handed_out: &mut FxHashSet<String>,
    staged: &mut Vec<(SymbolId, Ident<'a>)>,
    cx: &Cx<'a, '_>,
) -> Option<Vec<(SymbolId, String)>> {
    let Statement::BlockStatement(block) = statement else {
        return Some(Vec::new());
    };
    let scope_id = block.scope_id.get()?;
    let mut introduced = Vec::new();
    for (symbol, name) in declared_in(scope_id, cx.scoping()) {
        let final_name = relocate(&name, symbol, outer, moved, handed_out, staged, cx)?;
        introduced.push((symbol, final_name));
    }
    if let Some(relocated) = moved.get(&scope_id) {
        for (symbol, name) in relocated.clone() {
            let final_name = relocate(&name, symbol, outer, moved, handed_out, staged, cx)?;
            introduced.push((symbol, final_name));
        }
    }
    Some(introduced)
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

fn hoisted_by<'a>(statement: Option<&Statement<'a>>) -> Hoisted<'a> {
    let mut hoisted = Hoisted::default();
    if let Some(statement) = statement {
        hoisted.visit_statement(statement);
    }
    hoisted
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
    names
        .into_iter()
        .filter(|(_, symbol)| {
            symbol.is_none_or(|symbol| seen_elsewhere(symbol, &references, scoping))
        })
        .collect()
}

fn var_statement<'a>(names: &[(Ident<'a>, Option<SymbolId>)], cx: &Cx<'a, '_>) -> Statement<'a> {
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
    Statement::new_variable_declaration(SPAN, VariableDeclarationKind::Var, declarators, false, cx)
}

fn has_with(scoping: &Scoping) -> bool {
    scoping
        .scope_descendants_from_root()
        .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
}

#[derive(Default)]
pub struct DeadCode<'a> {
    moved: Moved,
    handed_out: FxHashSet<String>,
    renames: Renames<'a>,
    strict: bool,
    with: Option<bool>,
    unsafe_lane: bool,
    armed: usize,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DeadCode::default())
}

impl<'a> DeadCode<'a> {
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
        let IfStatement {
            consequent,
            alternate,
            ..
        } = node.unbox();
        if !names.is_empty() {
            stmts.push(var_statement(&names, cx));
        }
        match if value { Some(consequent) } else { alternate } {
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
        let unresolved = scoping.root_unresolved_references();
        if outer == scoping.root_scope_id()
            || unresolved.contains_key("eval")
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
                !scoping.symbol_flags(symbol).is_function() && !unresolved.contains_key(name)
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
        if value {
            match self.try_merge(consequent, outer, stmts, cx) {
                Ok(()) => true,
                Err(consequent) => refuse(span, test, consequent, alternate, stmts, cx),
            }
        } else {
            let Some(branch) = alternate else {
                return true;
            };
            match self.try_merge(branch, outer, stmts, cx) {
                Ok(()) => true,
                Err(branch) => refuse(span, test, consequent, Some(branch), stmts, cx),
            }
        }
    }

    fn try_merge(
        &mut self,
        branch: Statement<'a>,
        outer: ScopeId,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> Result<(), Statement<'a>> {
        let mut staged = Vec::new();
        let planned = plan_merge(
            &branch,
            outer,
            &self.moved,
            &mut self.handed_out,
            &mut staged,
            cx,
        );
        match planned {
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

    fn rewrite_statements(
        &mut self,
        stmts: &mut AVec<'a, Statement<'a>>,
        cx: &mut Cx<'a, '_>,
    ) -> usize {
        let hits = stmts
            .iter()
            .filter(|stmt| {
                matches!(stmt, Statement::IfStatement(node) if opaque_test(&node.test).is_some())
            })
            .count();
        self.armed = self.armed.saturating_sub(hits);
        if hits == 0 {
            return 0;
        }
        let outer = cx.current_scope_id();
        let taken: Vec<Statement<'a>> = stmts.drain(..).collect();
        let mut changes = 0usize;
        for statement in taken {
            let Statement::IfStatement(node) = statement else {
                stmts.push(statement);
                continue;
            };
            let Some(value) = opaque_test(&node.test) else {
                stmts.push(Statement::IfStatement(node));
                continue;
            };
            let span = node.span;
            let folded = if cx.opts.unsafe_transforms {
                self.merge_branch(node.unbox(), span, value, outer, stmts, cx)
            } else {
                self.fold_soundly(node, value, outer, stmts, cx)
            };
            if folded {
                changes += 1;
            }
        }
        changes
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::ConditionalExpression]);

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[StatementKind::IfStatement]);

impl<'a> Pass<'a> for DeadCode<'a> {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
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

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        if matches!(stmt, Statement::IfStatement(node) if opaque_test(&node.test).is_some()) {
            self.armed += 1;
        }
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if !self.unsafe_lane && self.armed == 0 {
            return;
        }
        let changes = self.rewrite_statements(stmts, cx);
        cx.changed_by(changes);
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::ConditionalExpression(conditional) = &*expr else {
            return;
        };
        let Some(value) = opaque_test(&conditional.test) else {
            return;
        };
        let placeholder = Expression::new_null_literal(SPAN, cx);
        let conditional = match std::mem::replace(expr, placeholder) {
            Expression::ConditionalExpression(conditional) => conditional.unbox(),
            other => {
                *expr = other;
                return;
            }
        };
        *expr = if value {
            conditional.consequent
        } else {
            conditional.alternate
        };
        cx.changed();
    }

    fn reset(&mut self) {
        self.moved = FxHashMap::default();
        self.handed_out = FxHashSet::default();
        self.renames = Renames::new();
        self.with = None;
    }

    fn enter_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.strict = program_is_strict(node);
        self.unsafe_lane = cx.opts.unsafe_transforms;
        self.armed = 0;
    }

    fn exit_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let renames = std::mem::take(&mut self.renames);
        renames.apply(node, cx.scoping());
    }
}
