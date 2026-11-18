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

#[derive(Default)]
struct Plan {
    by_value: FxHashSet<Span>,
    unstable: FxHashSet<Span>,
}

struct Write<'p> {
    index: usize,
    name: &'p str,
    span: Span,
    value: Option<bool>,
}

struct Read<'p> {
    index: usize,
    name: &'p str,
    span: Span,
}

struct Access<'p> {
    index: usize,
    writes: Vec<Write<'p>>,
    reads: Vec<Read<'p>>,
}

impl<'p> Access<'p> {
    fn write(&mut self, name: &'p str, span: Span, value: Option<bool>) {
        let index = self.index;
        self.writes.push(Write {
            index,
            name,
            span,
            value,
        });
    }
}

impl<'p> VisitJs<'p> for Access<'p> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'p>) {
        let index = self.index;
        self.reads.push(Read {
            index,
            name: it.name.as_str(),
            span: it.span,
        });
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'p>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
            let value = (it.operator == AssignmentOperator::Assign).then(|| is_constant(&it.right));
            self.write(target.name.as_str(), it.span, value);
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'p>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &it.argument {
            self.write(target.name.as_str(), it.span, None);
        }
        walk_js::walk_update_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'p>) {
        if !matches!(it, SimpleAssignmentTarget::AssignmentTargetIdentifier(_)) {
            walk_js::walk_simple_assignment_target(self, it);
        }
    }
}

fn is_constant(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            is_pure_operator(unary.operator) && is_constant(&unary.argument)
        }
        Expression::BinaryExpression(binary) => {
            is_constant(&binary.left) && is_constant(&binary.right)
        }
        _ => false,
    }
}

fn written_later(writes: &[Write<'_>], name: &str, index: usize, at: Span) -> bool {
    writes.iter().any(|write| {
        write.name == name
            && write.span != at
            && (write.index > index || (write.index == index && !write.span.contains_inclusive(at)))
    })
}

fn plan<'p>(arguments: &'p AVec<'p, Argument<'p>>) -> Plan {
    let mut access = Access {
        index: 0,
        writes: Vec::new(),
        reads: Vec::new(),
    };
    for (index, argument) in arguments.iter().enumerate() {
        access.index = index;
        access.visit_argument(argument);
    }
    let mut plan = Plan::default();
    for write in &access.writes {
        let Some(constant) = write.value else {
            continue;
        };
        if !written_later(&access.writes, write.name, write.index, write.span) {
            continue;
        }
        if constant {
            plan.by_value.insert(write.span);
        } else {
            plan.unstable.insert(write.span);
        }
    }
    for read in &access.reads {
        if written_later(&access.writes, read.name, read.index, read.span) {
            plan.unstable.insert(read.span);
        }
    }
    plan
}

struct Settled<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    plan: &'c Plan,
    refused: bool,
}

impl<'a> VisitMut<'a> for Settled<'_, 'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let Expression::AssignmentExpression(assignment) = it else {
            walk_mut::walk_expression(self, it);
            return;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            walk_mut::walk_expression(self, it);
            return;
        };
        if assignment.operator != AssignmentOperator::Assign {
            walk_mut::walk_expression(self, it);
            return;
        }
        if self.plan.unstable.contains(&assignment.span) {
            self.refused = true;
            return;
        }
        *it = match self.plan.by_value.contains(&assignment.span) {
            true => assignment.right.clone_in(self.cx.allocator()),
            false => Expression::new_identifier(assignment.span, target.name, self.cx),
        };
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if self.plan.unstable.contains(&it.span) {
            self.refused = true;
        }
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
    settled_after(&parts, inlined, (span, &plan(arguments)), cx)
}

fn settled_after<'a>(
    parts: &[&Expression<'_>],
    mut inlined: Expression<'a>,
    (span, plan): (Span, &Plan),
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let mut settled = Settled {
        cx,
        plan,
        refused: false,
    };
    settled.visit_expression(&mut inlined);
    if settled.refused {
        return None;
    }
    let allocator = cx.allocator();
    let mut expressions = AVec::with_capacity_in(parts.len() + 1, cx);
    for part in parts {
        expressions.push(part.clone_in(allocator));
    }
    expressions.push(inlined);
    Some(Expression::new_sequence_expression(span, expressions, cx))
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
    let fits: Fits<'_> = &|uses, _| !uses.is_blocked();
    let inlined = chain.inline_through((wrapper, arguments, false), view, fits, cx)?;
    settled_after(&writes.parts, inlined, (span, &Plan::default()), cx)
}
