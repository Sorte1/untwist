use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::literal::is_safe_literal;
use untwist_core::pass::Cx;
use untwist_core::semantic::scope_hazard;
use untwist_eval::fold::{fold_to_const, FoldOptions};

use crate::mutants::{
    nested_writes_trusted, write_only_order_trusted, write_only_scope_hazard_ignored,
};

fn is_constant_value(expr: &Expression<'_>) -> bool {
    fold_to_const(expr, &FoldOptions::default()).is_some() || is_safe_literal(expr)
}

fn target_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    scoping
        .get_reference(target.reference_id.get()?)
        .symbol_id()
}

fn written_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<(SymbolId, usize)> {
    let symbol = target_symbol(expr, scoping)?;
    Some((symbol, self_writes(expr, symbol, scoping)?))
}

fn self_writes(expr: &Expression<'_>, symbol: SymbolId, scoping: &Scoping) -> Option<usize> {
    let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
        return None;
    };
    if target_symbol(expr, scoping)? != symbol {
        return nested_writes_trusted().then_some(0);
    }
    if is_constant_value(&assignment.right) {
        return Some(1);
    }
    let nested = match assignment.right.without_parentheses() {
        Expression::AssignmentExpression(_) => self_writes(&assignment.right, symbol, scoping)?,
        _ => numeric_writes(&assignment.right, symbol, scoping)?,
    };
    Some(1 + nested)
}

fn numeric_writes(expr: &Expression<'_>, symbol: SymbolId, scoping: &Scoping) -> Option<usize> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => Some(0),
        Expression::UnaryExpression(unary) => {
            let numeric_operator = matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
            );
            let operand = matches!(
                unary.argument.without_parentheses(),
                Expression::NumericLiteral(_)
            );
            (numeric_operator && operand).then_some(0)
        }
        Expression::BinaryExpression(binary) if is_numeric_operator(binary.operator) => {
            let left = numeric_writes(&binary.left, symbol, scoping)?;
            Some(left + numeric_writes(&binary.right, symbol, scoping)?)
        }
        Expression::AssignmentExpression(assignment) => {
            let value = numeric_writes(&assignment.right, symbol, scoping)?;
            if target_symbol(expr, scoping)? == symbol {
                Some(1 + value)
            } else {
                nested_writes_trusted().then_some(value)
            }
        }
        _ => None,
    }
}

fn is_numeric_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
    )
}

#[derive(Default)]
struct Writes {
    in_sequences: FxHashMap<SymbolId, Vec<Span>>,
    counts: FxHashMap<SymbolId, usize>,
    functions: Vec<Span>,
    owner: FxHashMap<Span, Span>,
}

struct Scan<'s> {
    scoping: &'s Scoping,
    writes: Writes,
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        if let Some((_, leading)) = it.expressions.split_last() {
            for expr in leading {
                if let Some((symbol, count)) = written_symbol(expr, self.scoping) {
                    let span = expr.without_parentheses().span();
                    *self.writes.counts.entry(symbol).or_default() += count;
                    self.writes
                        .in_sequences
                        .entry(symbol)
                        .or_default()
                        .push(span);
                    if let Some(function) = self.writes.functions.last() {
                        self.writes.owner.insert(span, *function);
                    }
                }
            }
        }
        walk_js::walk_sequence_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.writes.functions.push(it.span);
        walk_js::walk_function(self, it, flags);
        self.writes.functions.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.writes.functions.push(it.span);
        walk_js::walk_arrow_function_expression(self, it);
        self.writes.functions.pop();
    }
}

struct Declarations<'s> {
    wanted: &'s FxHashSet<SymbolId>,
    found: FxHashMap<SymbolId, (Span, Option<Span>, bool)>,
    functions: Vec<Span>,
}

impl Declarations<'_> {
    fn note(&mut self, declaration: &VariableDeclaration<'_>, top: bool) {
        let [declarator] = declaration.declarations.as_slice() else {
            return;
        };
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        let Some(symbol) = id
            .symbol_id
            .get()
            .filter(|symbol| self.wanted.contains(symbol))
        else {
            return;
        };
        let plain = declarator.init.is_none()
            && match declaration.kind {
                VariableDeclarationKind::Var => true,
                VariableDeclarationKind::Let => top,
                _ => false,
            };
        if plain {
            self.found.insert(
                symbol,
                (
                    declaration.span,
                    self.functions.last().copied(),
                    declaration.kind == VariableDeclarationKind::Let,
                ),
            );
        }
    }
}

impl<'a> VisitJs<'a> for Declarations<'_> {
    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        for stmt in &it.statements {
            if let Statement::VariableDeclaration(declaration) = stmt {
                self.note(declaration, true);
            }
        }
        walk_js::walk_function_body(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            self.note(it, false);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions.push(it.span);
        walk_js::walk_function(self, it, flags);
        self.functions.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.functions.push(it.span);
        walk_js::walk_arrow_function_expression(self, it);
        self.functions.pop();
    }
}

fn write_only(scoping: &Scoping, symbol: SymbolId, writes: usize) -> bool {
    let flags = scoping.symbol_flags(symbol);
    let references: Vec<_> = scoping.get_resolved_references(symbol).collect();
    flags.intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::BlockScopedVariable)
        && !flags.contains(SymbolFlags::ConstVariable)
        && scoping.symbol_scope_id(symbol) != scoping.root_scope_id()
        && scoping.symbol_redeclarations(symbol).is_empty()
        && references.len() == writes
        && references
            .iter()
            .all(|reference| reference.is_write() && !reference.is_read())
}

pub(crate) struct Plan {
    dropped: FxHashSet<Span>,
    declarations: FxHashSet<Span>,
}

pub(crate) fn plan(program: &Program<'_>, scoping: &Scoping) -> Option<Plan> {
    if scope_hazard(scoping).is_some() && !write_only_scope_hazard_ignored() {
        return None;
    }
    let mut scan = Scan {
        scoping,
        writes: Writes::default(),
    };
    scan.visit_program(program);
    let writes = scan.writes;
    let wanted: FxHashSet<SymbolId> = writes
        .in_sequences
        .iter()
        .filter(|(symbol, _)| write_only(scoping, **symbol, writes.counts[*symbol]))
        .map(|(symbol, _)| *symbol)
        .collect();
    if wanted.is_empty() {
        return None;
    }
    let mut declarations = Declarations {
        wanted: &wanted,
        found: FxHashMap::default(),
        functions: Vec::new(),
    };
    declarations.visit_program(program);
    let found = declarations.found;
    let mut plan = Plan {
        dropped: FxHashSet::default(),
        declarations: FxHashSet::default(),
    };
    for symbol in wanted {
        let Some((declared, function, lexical)) = found.get(&symbol).copied() else {
            continue;
        };
        let spans = &writes.in_sequences[&symbol];
        let ordered = !lexical
            || write_only_order_trusted()
            || spans.iter().all(|span| {
                span.start > declared.end && writes.owner.get(span).copied() == function
            });
        if !ordered {
            continue;
        }
        plan.dropped.extend(spans.iter().copied());
        plan.declarations.insert(declared);
    }
    (!plan.dropped.is_empty()).then_some(plan)
}

#[derive(Clone, Copy)]
enum Position {
    Callee,
    Delete,
    Typeof,
}

impl Position {
    fn reads_a_reference(self, tail: &Expression<'_>, scoping: &Scoping) -> bool {
        match (self, tail.without_parentheses()) {
            (Position::Typeof, Expression::Identifier(id)) => id
                .reference_id
                .get()
                .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none()),
            (Position::Typeof, _) => false,
            (_, tail) => matches!(
                tail,
                Expression::Identifier(_)
                    | Expression::StaticMemberExpression(_)
                    | Expression::ComputedMemberExpression(_)
                    | Expression::PrivateFieldExpression(_)
                    | Expression::ChainExpression(_)
            ),
        }
    }
}

fn operand_position(parent: &Expression<'_>) -> Option<Position> {
    match parent {
        Expression::CallExpression(_) | Expression::TaggedTemplateExpression(_) => {
            Some(Position::Callee)
        }
        Expression::ChainExpression(chain) => {
            matches!(chain.expression, ChainElement::CallExpression(_)).then_some(Position::Callee)
        }
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::Delete => Some(Position::Delete),
            UnaryOperator::Typeof => Some(Position::Typeof),
            _ => None,
        },
        _ => None,
    }
}

struct Apply<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    plan: Plan,
    position: Option<Position>,
    applied: usize,
}

impl<'a> VisitMut<'a> for Apply<'_, 'a, '_> {
    fn visit_sequence_expression(&mut self, it: &mut SequenceExpression<'a>) {
        let last = it.expressions.len().saturating_sub(1);
        let before = it.expressions.len();
        let dropped = &self.plan.dropped;
        let mut index = 0;
        it.expressions.retain(|expr| {
            let keep = index == last || !dropped.contains(&expr.without_parentheses().span());
            index += 1;
            keep
        });
        self.applied += before - it.expressions.len();
        walk_mut::walk_sequence_expression(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let position = self.position.take();
        self.position = match &*it {
            Expression::ParenthesizedExpression(_) => position,
            other => operand_position(other),
        };
        walk_mut::walk_expression(self, it);
        self.position = None;
        let Expression::SequenceExpression(sequence) = it else {
            return;
        };
        if sequence.expressions.len() != 1 {
            return;
        }
        let scoping = self.cx.scoping();
        let pinned = position
            .zip(sequence.expressions.first())
            .is_some_and(|(position, tail)| position.reads_a_reference(tail, scoping));
        if pinned {
            let zero =
                Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, self.cx);
            sequence.expressions.insert(0, zero);
        } else if let Some(only) = sequence.expressions.pop() {
            *it = only;
        }
    }

    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        let before = it.len();
        let declarations = &self.plan.declarations;
        if it.iter().any(|stmt| declarations.contains(&stmt.span())) {
            let taken = it.take_in(self.cx);
            for stmt in taken {
                if !declarations.contains(&stmt.span()) {
                    it.push(stmt);
                }
            }
        }
        self.applied += before - it.len();
        walk_mut::walk_statements(self, it);
    }
}

pub(crate) fn apply<'a>(program: &mut Program<'a>, plan: Plan, cx: &Cx<'a, '_>) -> usize {
    let mut apply = Apply {
        cx,
        plan,
        position: None,
        applied: 0,
    };
    apply.visit_program(program);
    apply.applied
}

thread_local! {
    static SWEEP_PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

thread_local! {
    static SEQUENCED_VALUE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn forget_write_only_flags() {
    SEQUENCED_VALUE.with(|seen| seen.set(false));
    SWEEP_PENDING.with(|pending| pending.set(false));
}

pub(crate) fn note_sequenced_value() {
    SEQUENCED_VALUE.with(|seen| seen.set(true));
}

pub(crate) fn take_sequenced_value() -> bool {
    SEQUENCED_VALUE.with(|seen| seen.replace(false))
}

pub(crate) fn request_sweep() {
    SWEEP_PENDING.with(|pending| pending.set(true));
}

pub(crate) fn sweep<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    if !SWEEP_PENDING.with(|pending| pending.replace(false)) {
        return 0;
    }
    plan(program, cx.scoping()).map_or(0, |plan| apply(program, plan, cx))
}
