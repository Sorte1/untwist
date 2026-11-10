use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::global::{references_are_global, references_are_global_or};
use untwist_core::ast_util::inline::{inline_function_call, returned_expression};
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::member_parts;
use untwist_core::pass::Cx;
use untwist_eval::fold::{fold_to_const, Const, FoldOptions};

use std::borrow::Cow;

use super::CffValue;

fn truthy(value: &Const) -> bool {
    match value {
        Const::Bool(flag) => *flag,
        Const::Num(number) => *number != 0.0 && !number.is_nan(),
        Const::Str(text) => !text.is_empty(),
        Const::Null | Const::Undefined => false,
    }
}

fn encloses(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn predicate_receiver(test: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let (callee, _) = call_parts(test)?;
    let (receiver, _) = member_parts(callee)?;
    let Expression::Identifier(id) = receiver else {
        return None;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn dispatch_predicate<'k, 'p>(
    test: &'k Expression<'p>,
    symbol: SymbolId,
    scoping: &Scoping,
    props: &'k [(Cow<'k, str>, CffValue<'k, 'p>)],
) -> Option<(&'k Function<'p>, &'k AVec<'p, Argument<'p>>)> {
    if predicate_receiver(test, scoping)? != symbol {
        return None;
    }
    let (callee, arguments) = call_parts(test)?;
    let (_, name) = member_parts(callee)?;
    let value = props
        .iter()
        .find(|(key, _)| key.as_ref() == name.as_ref())
        .map(|(_, value)| value)?;
    let CffValue::Function(function) = value else {
        return None;
    };
    Some((function, arguments))
}

fn decide<'p>(
    test: &Expression<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    cx: &Cx<'_, '_>,
) -> Option<bool> {
    let (function, arguments) = dispatch_predicate(test, symbol, cx.scoping(), props)?;
    let inlined = inline_function_call(function, arguments, cx)?;
    let value = fold_to_const(&inlined, &FoldOptions::default())?;
    reads_only_globals(function, arguments, cx.scoping()).then(|| truthy(&value))
}

fn reads_only_globals(
    function: &Function<'_>,
    arguments: &[Argument<'_>],
    scoping: &Scoping,
) -> bool {
    let own = function.scope_id.get();
    let is_own = |symbol| Some(scoping.symbol_scope_id(symbol)) == own;
    let body = returned_expression(function)
        .is_none_or(|expr| references_are_global_or(expr, scoping, is_own));
    body && arguments.iter().all(|argument| {
        argument
            .as_expression()
            .is_some_and(|expr| references_are_global(expr, scoping))
    })
}

pub(super) struct Branch<'p> {
    test: &'p Expression<'p>,
    consequent: Span,
    alternate: Option<Span>,
}

#[derive(Default)]
pub(super) struct Branches<'p> {
    by_receiver: FxHashMap<SymbolId, Vec<Branch<'p>>>,
    statements: Vec<Span>,
}

struct BranchScan<'s, 'p> {
    scoping: &'s Scoping,
    found: Branches<'p>,
}

impl<'p> BranchScan<'_, 'p> {
    fn branch(&mut self, test: &'p Expression<'p>, consequent: Span, alternate: Option<Span>) {
        let Some(symbol) = predicate_receiver(test, self.scoping) else {
            return;
        };
        self.found
            .by_receiver
            .entry(symbol)
            .or_default()
            .push(Branch {
                test,
                consequent,
                alternate,
            });
    }
}

impl<'p> VisitJs<'p> for BranchScan<'_, 'p> {
    fn visit_if_statement(&mut self, it: &IfStatement<'p>) {
        let it: &'p IfStatement<'p> = self.alloc(it);
        self.branch(
            &it.test,
            it.consequent.span(),
            it.alternate.as_ref().map(GetSpan::span),
        );
        walk_js::walk_if_statement(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'p>) {
        let it: &'p ConditionalExpression<'p> = self.alloc(it);
        self.branch(&it.test, it.consequent.span(), Some(it.alternate.span()));
        walk_js::walk_conditional_expression(self, it);
    }

    fn visit_statements(&mut self, it: &AVec<'p, Statement<'p>>) {
        self.found
            .statements
            .extend(it.iter().filter_map(|statement| match statement {
                Statement::ExpressionStatement(expression) => Some(expression.span),
                _ => None,
            }));
        walk_js::walk_statements(self, it);
    }
}

pub(super) fn predicate_branches<'p>(program: &'p Program<'p>, scoping: &Scoping) -> Branches<'p> {
    let mut scan = BranchScan {
        scoping,
        found: Branches::default(),
    };
    scan.visit_program(program);
    scan.found
}

pub(super) fn dead_branch_spans<'p>(
    branches: &Branches<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    cx: &Cx<'_, '_>,
) -> Vec<Span> {
    let Some(tested) = branches.by_receiver.get(&symbol) else {
        return Vec::new();
    };
    tested
        .iter()
        .filter_map(|branch| {
            let taken = decide(branch.test, symbol, props, cx)?;
            let dead = if taken {
                branch.alternate
            } else {
                Some(branch.consequent)
            };
            dead.filter(|span| span.start < span.end)
        })
        .collect()
}

pub(super) struct DeadWrites {
    pub statements: Vec<Span>,
    pub kept: Vec<Span>,
}

pub(super) fn dead_write_statements(
    branches: &Branches<'_>,
    mutations: &[Span],
    dead: &[Span],
) -> DeadWrites {
    let mut found: Vec<Option<Span>> = vec![None; mutations.len()];
    let inside_dead = |span: &&Span| dead.iter().any(|range| encloses(*range, **span));
    for span in branches.statements.iter().filter(inside_dead) {
        for (slot, mutation) in found.iter_mut().zip(mutations) {
            if encloses(*span, *mutation) {
                *slot = Some(*span);
            }
        }
    }
    let mut statements = Vec::new();
    let mut kept = Vec::new();
    for (mutation, found) in mutations.iter().zip(&found) {
        match found {
            Some(span) => statements.push(*span),
            None => kept.push(*mutation),
        }
    }
    statements.sort_unstable_by_key(|span| (span.start, span.end));
    statements.dedup();
    DeadWrites { statements, kept }
}
