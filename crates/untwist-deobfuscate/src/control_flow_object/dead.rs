use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
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

fn dispatch_predicate<'k, 'p>(
    test: &'k Expression<'p>,
    symbol: SymbolId,
    scoping: &Scoping,
    props: &'k [(Cow<'k, str>, CffValue<'k, 'p>)],
) -> Option<(&'k Function<'p>, &'k AVec<'p, Argument<'p>>)> {
    let (callee, arguments) = call_parts(test)?;
    let (receiver, name) = member_parts(callee)?;
    let Expression::Identifier(id) = receiver else {
        return None;
    };
    let resolved = id
        .reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())?;
    if resolved != symbol {
        return None;
    }
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

struct DeadScan<'k, 'p, 'a, 'c> {
    symbol: SymbolId,
    props: &'k [(Cow<'k, str>, CffValue<'k, 'p>)],
    cx: &'k Cx<'a, 'c>,
    spans: Vec<Span>,
}

impl<'p> DeadScan<'_, 'p, '_, '_> {
    fn branch(&mut self, test: &Expression<'p>, consequent: Span, alternate: Option<Span>) {
        let Some(taken) = decide(test, self.symbol, self.props, self.cx) else {
            return;
        };
        let dead = if taken { alternate } else { Some(consequent) };
        if let Some(span) = dead.filter(|span| span.start < span.end) {
            self.spans.push(span);
        }
    }
}

impl<'p> VisitJs<'p> for DeadScan<'_, 'p, '_, '_> {
    fn visit_if_statement(&mut self, it: &IfStatement<'p>) {
        self.branch(
            &it.test,
            it.consequent.span(),
            it.alternate.as_ref().map(GetSpan::span),
        );
        walk_js::walk_if_statement(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'p>) {
        self.branch(&it.test, it.consequent.span(), Some(it.alternate.span()));
        walk_js::walk_conditional_expression(self, it);
    }
}

pub(super) fn dead_branch_spans<'k, 'p>(
    program: &'p Program<'p>,
    symbol: SymbolId,
    props: &'k [(Cow<'k, str>, CffValue<'k, 'p>)],
    cx: &'k Cx<'_, '_>,
) -> Vec<Span> {
    let mut scan = DeadScan {
        symbol,
        props,
        cx,
        spans: Vec::new(),
    };
    scan.visit_program(program);
    scan.spans
}

struct WriteStatements<'k> {
    mutations: &'k [Span],
    dead: &'k [Span],
    found: Vec<Option<Span>>,
}

impl<'p> VisitJs<'p> for WriteStatements<'_> {
    fn visit_statements(&mut self, it: &AVec<'p, Statement<'p>>) {
        for statement in it {
            let Statement::ExpressionStatement(expression) = statement else {
                continue;
            };
            let span = expression.span;
            if !self.dead.iter().any(|range| encloses(*range, span)) {
                continue;
            }
            for (index, mutation) in self.mutations.iter().enumerate() {
                if encloses(span, *mutation) {
                    self.found[index] = Some(span);
                }
            }
        }
        walk_js::walk_statements(self, it);
    }
}

pub(super) struct DeadWrites {
    pub statements: Vec<Span>,
    pub kept: Vec<Span>,
}

pub(super) fn dead_write_statements(
    program: &Program<'_>,
    mutations: &[Span],
    dead: &[Span],
) -> DeadWrites {
    let mut scan = WriteStatements {
        mutations,
        dead,
        found: vec![None; mutations.len()],
    };
    scan.visit_program(program);
    let mut statements = Vec::new();
    let mut kept = Vec::new();
    for (mutation, found) in mutations.iter().zip(&scan.found) {
        match found {
            Some(span) => statements.push(*span),
            None => kept.push(*mutation),
        }
    }
    statements.sort_unstable_by_key(|span| (span.start, span.end));
    statements.dedup();
    DeadWrites { statements, kept }
}
