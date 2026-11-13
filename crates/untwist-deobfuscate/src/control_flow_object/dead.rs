use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::global::{references_are_global, references_are_global_or};
use untwist_core::ast_util::inline::{param_names, returned_expression};
use untwist_core::ast_util::prop::member_parts;
use untwist_core::pass::Cx;
use untwist_eval::fold::{fold_binary, fold_to_const, fold_unary, Const, FoldOptions};

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

const MAX_TEST_DEPTH: usize = 16;

const FOLDED_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];

fn receiver_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let (receiver, _) = member_parts(expr)?;
    let Expression::Identifier(id) = receiver else {
        return None;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn tested_receivers(
    test: &Expression<'_>,
    scoping: &Scoping,
    depth: usize,
    found: &mut Vec<SymbolId>,
) {
    if depth > MAX_TEST_DEPTH {
        return;
    }
    let inner = test.without_parentheses();
    if let Some(symbol) = receiver_symbol(inner, scoping) {
        found.push(symbol);
        return;
    }
    match inner {
        Expression::CallExpression(call) if !call.optional => {
            tested_receivers(&call.callee, scoping, depth + 1, found);
            for argument in &call.arguments {
                if let Some(argument) = argument.as_expression() {
                    tested_receivers(argument, scoping, depth + 1, found);
                }
            }
        }
        Expression::BinaryExpression(binary) => {
            tested_receivers(&binary.left, scoping, depth + 1, found);
            tested_receivers(&binary.right, scoping, depth + 1, found);
        }
        Expression::UnaryExpression(unary) => {
            tested_receivers(&unary.argument, scoping, depth + 1, found);
        }
        _ => {}
    }
}

struct Known<'k, 'p> {
    symbol: SymbolId,
    props: &'k [(Cow<'k, str>, CffValue<'k, 'p>)],
    scoping: &'k Scoping,
}

impl<'p> Known<'_, 'p> {
    fn prop(&self, member: &Expression<'_>) -> Option<&CffValue<'_, 'p>> {
        if receiver_symbol(member, self.scoping)? != self.symbol {
            return None;
        }
        let (_, name) = member_parts(member)?;
        self.props
            .iter()
            .find(|(key, _)| key.as_ref() == name.as_ref())
            .map(|(_, value)| value)
    }

    fn value(&self, expr: &Expression<'_>, depth: usize) -> Option<Const> {
        if depth > MAX_TEST_DEPTH {
            return None;
        }
        let inner = expr.without_parentheses();
        if let Some(value) = self.prop(inner) {
            return match value {
                CffValue::Text(literal) => literal
                    .value
                    .as_str()
                    .map(|text| Const::Str(text.to_string())),
                CffValue::Function(_) => None,
            };
        }
        match inner {
            Expression::CallExpression(call) if !call.optional => self.call(call, depth),
            Expression::BinaryExpression(binary) => fold_binary(
                binary.operator,
                &self.value(&binary.left, depth + 1)?,
                &self.value(&binary.right, depth + 1)?,
            ),
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                fold_unary(unary.operator, &self.value(&unary.argument, depth + 1)?)
            }
            _ if references_are_global(inner, self.scoping) => {
                fold_to_const(inner, &FoldOptions::default())
            }
            _ => None,
        }
    }

    fn call(&self, call: &CallExpression<'_>, depth: usize) -> Option<Const> {
        let CffValue::Function(function) = self.prop(&call.callee)? else {
            return None;
        };
        let names = param_names(function)?;
        if names.len() != call.arguments.len()
            || names.iter().any(|name| FOLDED_GLOBALS.contains(name))
            || !reads_only_own_or_globals(function, self.scoping)
        {
            return None;
        }
        let mut options = FoldOptions::default();
        for (name, argument) in names.iter().zip(&call.arguments) {
            let value = self.value(argument.as_expression()?, depth + 1)?;
            options.bindings.insert((*name).to_string(), value);
        }
        fold_to_const(returned_expression(function)?, &options)
    }
}

fn decide<'p>(
    test: &Expression<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    cx: &Cx<'_, '_>,
) -> Option<bool> {
    let known = Known {
        symbol,
        props,
        scoping: cx.scoping(),
    };
    known.value(test, 0).map(|value| truthy(&value))
}

fn reads_only_own_or_globals(function: &Function<'_>, scoping: &Scoping) -> bool {
    let own = function.scope_id.get();
    let is_own = |symbol| Some(scoping.symbol_scope_id(symbol)) == own;
    returned_expression(function)
        .is_some_and(|expr| references_are_global_or(expr, scoping, is_own))
}

#[derive(Clone, Copy)]
pub(super) struct DeadPart {
    node: usize,
    pub span: Span,
}

impl DeadPart {
    fn of_statement(stmt: &Statement<'_>) -> Self {
        Self {
            node: std::ptr::from_ref(stmt) as usize,
            span: covering_span(stmt),
        }
    }

    fn of_expression(expr: &Expression<'_>) -> Self {
        Self {
            node: std::ptr::from_ref(expr) as usize,
            span: expr.span(),
        }
    }
}

pub(super) struct Branch<'p> {
    test: &'p Expression<'p>,
    consequent: DeadPart,
    alternate: Option<DeadPart>,
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
    fn branch(
        &mut self,
        test: &'p Expression<'p>,
        consequent: DeadPart,
        alternate: Option<DeadPart>,
    ) {
        let mut symbols = Vec::new();
        tested_receivers(test, self.scoping, 0, &mut symbols);
        symbols.sort_unstable();
        symbols.dedup();
        for symbol in symbols {
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
}

impl<'p> VisitJs<'p> for BranchScan<'_, 'p> {
    fn visit_if_statement(&mut self, it: &IfStatement<'p>) {
        let it: &'p IfStatement<'p> = self.alloc(it);
        self.branch(
            &it.test,
            DeadPart::of_statement(&it.consequent),
            it.alternate.as_ref().map(DeadPart::of_statement),
        );
        walk_js::walk_if_statement(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'p>) {
        let it: &'p ConditionalExpression<'p> = self.alloc(it);
        self.branch(
            &it.test,
            DeadPart::of_expression(&it.consequent),
            Some(DeadPart::of_expression(&it.alternate)),
        );
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

fn covering_span(stmt: &Statement<'_>) -> Span {
    let span = stmt.span();
    if !span.is_empty() {
        return span;
    }
    let Statement::BlockStatement(block) = stmt else {
        return span;
    };
    let mut inner = block.body.iter().map(covering_span);
    let Some(first) = inner.next() else {
        return span;
    };
    if first.is_empty() {
        return span;
    }
    let mut covered = first;
    for next in inner {
        if next.is_empty() || next.start < covered.end {
            return span;
        }
        covered.end = next.end;
    }
    covered
}

pub(super) fn predicate_branches<'p>(program: &'p Program<'p>, scoping: &Scoping) -> Branches<'p> {
    let mut scan = BranchScan {
        scoping,
        found: Branches::default(),
    };
    scan.visit_program(program);
    scan.found
}

pub(super) fn dead_branch_parts<'p>(
    branches: &Branches<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    cx: &Cx<'_, '_>,
) -> Vec<DeadPart> {
    let Some(tested) = branches.by_receiver.get(&symbol) else {
        return Vec::new();
    };
    tested
        .iter()
        .filter_map(|branch| {
            let taken = decide(branch.test, symbol, props, cx)?;
            if taken {
                branch.alternate
            } else {
                Some(branch.consequent)
            }
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

struct DeadUses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    dead: FxHashSet<usize>,
    depth: usize,
    inside: FxHashSet<Span>,
    outside: FxHashSet<Span>,
}

impl DeadUses<'_> {
    fn record(&mut self, span: Span) {
        if self.depth > 0 {
            self.inside.insert(span);
        } else {
            self.outside.insert(span);
        }
    }

    fn resolves(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            == Some(self.symbol)
    }

    fn entering(&mut self, node: usize) -> bool {
        let dead = self.dead.contains(&node);
        if dead {
            self.depth += 1;
        }
        dead
    }
}

impl<'a> VisitJs<'a> for DeadUses<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        walk_js::walk_statement(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        if receiver_symbol(it, self.scoping) == Some(self.symbol) {
            self.record(it.span());
        }
        walk_js::walk_expression(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = it.as_member_expression() {
            if let Expression::Identifier(object) = member.object() {
                if self.resolves(object) {
                    self.record(member.span());
                }
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.resolves(it) {
            self.record(it.span);
        }
    }
}

pub(super) fn mutations_only_in_dead_parts(
    program: &Program<'_>,
    symbol: SymbolId,
    dead: &[DeadPart],
    mutations: &[Span],
    scoping: &Scoping,
) -> bool {
    let mut uses = DeadUses {
        scoping,
        symbol,
        dead: dead.iter().map(|part| part.node).collect(),
        depth: 0,
        inside: FxHashSet::default(),
        outside: FxHashSet::default(),
    };
    uses.visit_program(program);
    mutations
        .iter()
        .all(|span| uses.inside.contains(span) && !uses.outside.contains(span))
}
