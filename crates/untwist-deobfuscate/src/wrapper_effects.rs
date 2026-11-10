use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::Span;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::inline::{inline_function_call, param_names, returned_expression};
use untwist_core::pass::Cx;

use crate::wrapper_args::{is_arithmetic, keeps_its_operand, SiteView};
use crate::wrapper_calls::is_substitutable_argument;
use crate::wrapper_chain::{Chain, Fits};

fn is_pure_operator(operator: UnaryOperator) -> bool {
    matches!(
        operator,
        UnaryOperator::UnaryNegation
            | UnaryOperator::UnaryPlus
            | UnaryOperator::LogicalNot
            | UnaryOperator::BitwiseNot
    )
}

fn local_writes<'a>(expr: &Expression<'a>, writes: &mut Vec<&'a str>) -> bool {
    if is_substitutable_argument(expr) {
        return true;
    }
    match expr.without_parentheses() {
        Expression::AssignmentExpression(assignment) => {
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
                return false;
            };
            if assignment.operator != AssignmentOperator::Assign {
                return false;
            }
            writes.push(target.name.as_str());
            local_writes(&assignment.right, writes)
        }
        Expression::UnaryExpression(unary) if is_pure_operator(unary.operator) => {
            local_writes(&unary.argument, writes)
        }
        Expression::BinaryExpression(binary) => {
            local_writes(&binary.left, writes) && local_writes(&binary.right, writes)
        }
        _ => false,
    }
}

fn outer_assignments<'p>(expr: &'p Expression<'p>, out: &mut Vec<&'p Expression<'p>>) {
    match expr.without_parentheses() {
        Expression::AssignmentExpression(_) => out.push(expr),
        Expression::UnaryExpression(unary) => outer_assignments(&unary.argument, out),
        Expression::BinaryExpression(binary) => {
            outer_assignments(&binary.left, out);
            outer_assignments(&binary.right, out);
        }
        _ => {}
    }
}

#[derive(Default)]
struct Names<'a> {
    found: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Names<'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found.insert(it.name.as_str());
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if !matches!(it, SimpleAssignmentTarget::AssignmentTargetIdentifier(_)) {
            walk_js::walk_simple_assignment_target(self, it);
        }
    }
}

struct Settled<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
}

impl<'a> VisitMut<'a> for Settled<'_, 'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        let Expression::AssignmentExpression(assignment) = it else {
            return;
        };
        if assignment.operator != AssignmentOperator::Assign {
            return;
        }
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            return;
        };
        *it = Expression::new_identifier(assignment.span, target.name, self.cx);
    }
}

fn names_in<'a>(expr: &Expression<'a>) -> FxHashSet<&'a str> {
    let mut names = Names::default();
    names.visit_expression(expr);
    names.found
}

struct Effect<'p> {
    index: usize,
    parts: Vec<&'p Expression<'p>>,
    writes: Vec<&'p str>,
}

fn effects<'p>(arguments: &'p AVec<'p, Argument<'p>>, checked: bool) -> Option<Vec<Effect<'p>>> {
    let mut found = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let expression = argument.as_expression()?;
        if is_substitutable_argument(expression) {
            continue;
        }
        let mut writes = Vec::new();
        let mut parts = Vec::new();
        if local_writes(expression, &mut writes) {
            outer_assignments(expression, &mut parts);
        } else if checked {
            return None;
        } else {
            parts.push(expression);
        }
        found.push(Effect {
            index,
            parts,
            writes,
        });
    }
    Some(found)
}

fn reordering_is_unobservable(
    arguments: &AVec<'_, Argument<'_>>,
    used: &FxHashSet<usize>,
    effects: &[Effect<'_>],
) -> bool {
    used.iter().all(|index| {
        let Some(expression) = arguments.get(*index).and_then(Argument::as_expression) else {
            return true;
        };
        let reads = names_in(expression);
        effects
            .iter()
            .filter(|effect| effect.index >= *index)
            .all(|effect| effect.writes.iter().all(|name| !reads.contains(name)))
    })
}

fn used_parameters(names: &[&str], returned: &Expression<'_>) -> FxHashSet<usize> {
    let read = names_in(returned);
    names
        .iter()
        .enumerate()
        .filter(|(_, name)| read.contains(*name))
        .map(|(index, _)| index)
        .collect()
}

pub fn inline_keeping_effects<'p, 'a>(
    function: &Function<'p>,
    arguments: &'p AVec<'p, Argument<'p>>,
    span: Span,
    checked: bool,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let effects = effects(arguments, checked)?;
    if effects.is_empty() {
        return None;
    }
    let names = param_names(function)?;
    let used = used_parameters(&names, returned_expression(function)?);
    if checked && !reordering_is_unobservable(arguments, &used, &effects) {
        return None;
    }
    let inlined = inline_function_call(function, arguments, cx)?;
    let parts: Vec<&Expression<'p>> = effects
        .iter()
        .flat_map(|effect| effect.parts.iter().copied())
        .collect();
    Some(settled_after(&parts, inlined, span, cx))
}

fn settled_after<'a>(
    parts: &[&Expression<'_>],
    mut inlined: Expression<'a>,
    span: Span,
    cx: &Cx<'a, '_>,
) -> Expression<'a> {
    Settled { cx }.visit_expression(&mut inlined);
    let allocator = cx.allocator();
    let mut expressions = AVec::with_capacity_in(parts.len() + 1, cx);
    for part in parts {
        expressions.push(part.clone_in(allocator));
    }
    expressions.push(inlined);
    Expression::new_sequence_expression(span, expressions, cx)
}

struct LocalWrites<'v, 'p> {
    view: &'v SiteView<'v>,
    written: Vec<&'p str>,
    parts: Vec<&'p Expression<'p>>,
}

impl<'p> LocalWrites<'_, 'p> {
    fn accepts(&mut self, expr: &'p Expression<'p>, outermost: bool) -> bool {
        match expr.without_parentheses() {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => true,
            Expression::Identifier(id) => {
                self.written.contains(&id.name.as_str()) || self.view.is_plain_undefined(id)
            }
            Expression::UnaryExpression(unary) => {
                keeps_its_operand(unary.operator) && self.accepts(&unary.argument, outermost)
            }
            Expression::BinaryExpression(binary) => {
                is_arithmetic(binary.operator)
                    && self.accepts(&binary.left, outermost)
                    && self.accepts(&binary.right, outermost)
            }
            Expression::AssignmentExpression(assignment) => {
                if outermost {
                    self.parts.push(expr);
                }
                self.accepts_write(assignment)
            }
            _ => false,
        }
    }

    fn accepts_write(&mut self, assignment: &'p AssignmentExpression<'p>) -> bool {
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            return false;
        };
        if assignment.operator != AssignmentOperator::Assign || !self.view.is_plain_local(target) {
            return false;
        }
        let name = target.name.as_str();
        if !self.accepts(&assignment.right, false) || self.written.contains(&name) {
            return false;
        }
        self.written.push(name);
        true
    }
}

pub fn inline_local_writes<'p, 'a>(
    chain: &Chain<'p>,
    view: &SiteView<'_>,
    wrapper: SymbolId,
    arguments: &'p AVec<'p, Argument<'p>>,
    span: Span,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let mut writes = LocalWrites {
        view,
        written: Vec::new(),
        parts: Vec::new(),
    };
    for argument in arguments {
        if !writes.accepts(argument.as_expression()?, true) {
            return None;
        }
    }
    if writes.parts.is_empty() {
        return None;
    }
    let fits: Fits<'_> = &|uses, _, _| !uses.is_blocked();
    let inlined = chain.inline_through(wrapper, arguments, fits, cx)?;
    Some(settled_after(&writes.parts, inlined, span, cx))
}
