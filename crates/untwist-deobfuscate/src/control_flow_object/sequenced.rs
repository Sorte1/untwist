use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;
use untwist_core::semantic::write_reference_count;

use super::hoisted::{empty_lexical_or_var, pure_object};
use crate::mutants::sequence_object_reads_trusted;

struct Candidate {
    assignment: Span,
    symbol: SymbolId,
    later: Vec<Span>,
}

fn assigned_object(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let Expression::ObjectExpression(object) = &assignment.right else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign
        || object.properties.is_empty()
        || !pure_object(object)
    {
        return None;
    }
    scoping
        .get_reference(target.reference_id.get()?)
        .symbol_id()
}

struct Candidates<'s> {
    scoping: &'s Scoping,
    found: Vec<Candidate>,
}

impl<'a> VisitJs<'a> for Candidates<'_> {
    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        for (index, expr) in it.expressions.iter().enumerate() {
            let Some(symbol) = assigned_object(expr, self.scoping) else {
                continue;
            };
            self.found.push(Candidate {
                assignment: expr.without_parentheses().span(),
                symbol,
                later: it.expressions[index + 1..]
                    .iter()
                    .map(GetSpan::span)
                    .collect(),
            });
        }
        walk_js::walk_sequence_expression(self, it);
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

struct Uses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    spans: Vec<Span>,
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol == Some(self.symbol) {
            self.spans.push(it.span);
        }
    }
}

fn expressions<'k, 'a>(stmt: &'k Statement<'a>) -> Vec<&'k Expression<'a>> {
    match stmt {
        Statement::ExpressionStatement(node) => vec![&node.expression],
        Statement::ReturnStatement(node) => node.argument.iter().collect(),
        Statement::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .filter_map(|declarator| declarator.init.as_ref())
            .collect(),
        _ => Vec::new(),
    }
}

fn candidates(stmt: &Statement<'_>, scoping: &Scoping) -> Vec<Candidate> {
    let mut scan = Candidates {
        scoping,
        found: Vec::new(),
    };
    for expr in expressions(stmt) {
        scan.visit_expression(expr);
    }
    scan.found
}

fn sealed(stmt: &Statement<'_>, candidate: &Candidate, scoping: &Scoping) -> bool {
    if write_reference_count(scoping, candidate.symbol) != 1
        || !scoping.symbol_redeclarations(candidate.symbol).is_empty()
    {
        return false;
    }
    let mut uses = Uses {
        scoping,
        symbol: candidate.symbol,
        spans: Vec::new(),
    };
    uses.visit_statement(stmt);
    let total = scoping.get_resolved_reference_ids(candidate.symbol).len();
    let within = |outer: &Span, span: &Span| outer.start <= span.start && span.end <= outer.end;
    let written = uses
        .spans
        .iter()
        .filter(|span| within(&candidate.assignment, span))
        .count();
    let later = uses
        .spans
        .iter()
        .filter(|span| candidate.later.iter().any(|outer| within(outer, span)))
        .count();
    let reads_after = later + written == uses.spans.len() || sequence_object_reads_trusted();
    uses.spans.len() == total && written == 1 && later > 0 && reads_after
}

struct Take<'c, 'a, 'b> {
    assignment: Span,
    object: Option<Expression<'a>>,
    cx: &'c Cx<'a, 'b>,
}

impl<'a> VisitMut<'a> for Take<'_, 'a, '_> {
    fn visit_sequence_expression(&mut self, it: &mut SequenceExpression<'a>) {
        if self.object.is_none() {
            let at = it
                .expressions
                .iter()
                .position(|expr| expr.without_parentheses().span() == self.assignment);
            if let Some(at) = at.filter(|at| at + 1 < it.expressions.len()) {
                let mut removed = it.expressions.remove(at);
                if let Expression::AssignmentExpression(assignment) =
                    removed.without_parentheses_mut()
                {
                    self.object = Some(assignment.right.take_in(self.cx));
                }
                return;
            }
        }
        walk_mut::walk_sequence_expression(self, it);
    }
}

pub(super) fn declare_sequenced_objects<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    if !stmts
        .iter()
        .any(|stmt| empty_lexical_or_var(stmt).is_some())
    {
        return 0;
    }
    for at in 0..stmts.len() {
        for candidate in candidates(&stmts[at], cx.scoping()) {
            let Some(declared) = (0..at)
                .find(|index| empty_lexical_or_var(&stmts[*index]) == Some(candidate.symbol))
            else {
                continue;
            };
            if !sealed(&stmts[at], &candidate, cx.scoping()) {
                continue;
            }
            let mut take = Take {
                assignment: candidate.assignment,
                object: None,
                cx: &*cx,
            };
            take.visit_statement(&mut stmts[at]);
            let Some(object) = take.object else {
                continue;
            };
            if let Statement::VariableDeclaration(declaration) = &mut stmts[declared] {
                if let Some(declarator) = declaration.declarations.first_mut() {
                    declarator.init = Some(object);
                    return 1;
                }
            }
        }
    }
    0
}
