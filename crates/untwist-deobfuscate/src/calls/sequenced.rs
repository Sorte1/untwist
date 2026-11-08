use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::matcher::is_literal_argument;
use untwist_core::ast_util::scope::binding_symbol;

use crate::mutants::external_write_order_trusted;

struct Frame {
    id: u32,
    boundary: u32,
    current: usize,
    writes: u32,
    assigned: Option<Span>,
}

struct Candidate {
    value: Span,
    frame: u32,
    element: usize,
}

struct OrderScan<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    boundaries: Vec<u32>,
    next_id: u32,
    frames: Vec<Frame>,
    declared: Option<u32>,
    under_with: bool,
    aliased: bool,
    writers: Vec<u32>,
    candidates: FxHashMap<Span, Candidate>,
}

impl OrderScan<'_> {
    fn boundary(&self) -> u32 {
        self.boundaries.last().copied().unwrap_or(0)
    }

    fn fresh_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn enter_boundary(&mut self) {
        let id = self.fresh_id();
        self.boundaries.push(id);
    }

    fn assigned_literal(&self, expr: &Expression<'_>) -> Option<Span> {
        let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
            return None;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            return None;
        };
        let resolved = target
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        (assignment.operator == AssignmentOperator::Assign
            && resolved == Some(self.symbol)
            && is_literal_argument(&assignment.right))
        .then(|| assignment.right.span())
    }

    fn push_frame(&mut self) {
        let id = self.fresh_id();
        let boundary = self.boundary();
        self.frames.push(Frame {
            id,
            boundary,
            current: 0,
            writes: 0,
            assigned: None,
        });
    }

    fn mark_current(&mut self, index: usize) {
        if let Some(frame) = self.frames.last_mut() {
            frame.current = index;
            frame.writes = 0;
        }
    }

    fn unconditional_literal(&self, expr: &Expression<'_>) -> Option<Span> {
        match expr.without_parentheses() {
            Expression::AssignmentExpression(_) => self.assigned_literal(expr),
            Expression::BinaryExpression(binary) => self
                .unconditional_literal(&binary.right)
                .or_else(|| self.unconditional_literal(&binary.left)),
            Expression::UnaryExpression(unary) if unary.operator != UnaryOperator::Delete => {
                self.unconditional_literal(&unary.argument)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .iter()
                .rev()
                .find_map(|inner| self.unconditional_literal(inner)),
            Expression::LogicalExpression(logical) if external_write_order_trusted() => {
                self.unconditional_literal(&logical.right)
            }
            _ => None,
        }
    }

    fn mark_assigned(&mut self, expr: &Expression<'_>) {
        let Some(value) = self.unconditional_literal(expr) else {
            return;
        };
        if let Some(frame) = self.frames.last_mut() {
            if frame.writes == 1 || external_write_order_trusted() {
                frame.assigned = Some(value);
            }
        }
    }

    fn visit_ordered(&mut self, elements: &[&Expression<'_>], visit: impl Fn(&mut Self, usize)) {
        self.push_frame();
        for (index, element) in elements.iter().enumerate() {
            self.mark_current(index);
            visit(self, index);
            self.mark_assigned(element);
        }
        self.frames.pop();
    }

    fn note_write(&mut self) {
        self.writers.push(self.boundary());
        for frame in &mut self.frames {
            frame.writes += 1;
        }
        if external_write_order_trusted() {
            return;
        }
        let open: Vec<(u32, usize)> = self
            .frames
            .iter()
            .map(|frame| (frame.id, frame.current))
            .collect();
        for frame in &mut self.frames {
            frame.assigned = None;
        }
        self.candidates
            .retain(|_, candidate| !open.contains(&(candidate.frame, candidate.element)));
    }

    fn note_read(&mut self, span: Span) {
        let boundary = self.boundary();
        let found = self
            .frames
            .iter()
            .rev()
            .take_while(|frame| frame.boundary == boundary)
            .find_map(|frame| {
                frame.assigned.map(|value| Candidate {
                    value,
                    frame: frame.id,
                    element: frame.current,
                })
            });
        if let Some(candidate) = found {
            self.candidates.insert(span, candidate);
        }
    }
}

impl<'a> OrderScan<'_> {
    fn visit_arguments_after(&mut self, callee: &Expression<'a>, arguments: &[Argument<'a>]) {
        let spread = arguments
            .iter()
            .any(|argument| argument.as_expression().is_none());
        if spread {
            self.visit_expression(callee);
            for argument in arguments {
                self.visit_argument(argument);
            }
            return;
        }
        let elements: Vec<&Expression<'a>> = std::iter::once(callee)
            .chain(arguments.iter().filter_map(Argument::as_expression))
            .collect();
        self.visit_ordered(&elements, |scan, index| {
            scan.visit_expression(elements[index])
        });
    }
}

#[derive(Default)]
struct ArgumentsRead {
    found: bool,
}

impl<'a> VisitJs<'a> for ArgumentsRead {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

impl OrderScan<'_> {
    fn binds(&self, params: &FormalParameters<'_>) -> bool {
        params
            .items
            .iter()
            .any(|param| binding_symbol(&param.pattern) == Some(self.symbol))
    }

    fn note_parameter(&mut self, params: &FormalParameters<'_>, aliased: bool) {
        if !self.binds(params) {
            return;
        }
        if aliased && !external_write_order_trusted() {
            self.aliased = true;
            return;
        }
        self.declared = Some(self.boundary());
    }
}

impl<'a> VisitJs<'a> for OrderScan<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.enter_boundary();
        let mut reads = ArgumentsRead::default();
        if self.binds(&it.params) {
            reads.visit_function(it, flags);
        }
        self.note_parameter(&it.params, reads.found);
        walk_js::walk_function(self, it, flags);
        self.boundaries.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.enter_boundary();
        self.note_parameter(&it.params, false);
        walk_js::walk_arrow_function_expression(self, it);
        self.boundaries.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.enter_boundary();
        walk_js::walk_class(self, it);
        self.boundaries.pop();
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if binding_symbol(&it.id) == Some(self.symbol) {
            self.declared = Some(self.boundary());
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        self.push_frame();
        for (index, statement) in it.iter().enumerate() {
            self.mark_current(index);
            self.visit_statement(statement);
            if let Statement::ExpressionStatement(expression) = statement {
                self.mark_assigned(&expression.expression);
            }
        }
        self.frames.pop();
    }

    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        let elements: Vec<&Expression<'a>> = it.expressions.iter().collect();
        self.visit_ordered(&elements, |scan, index| {
            scan.visit_expression(elements[index])
        });
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.visit_arguments_after(&it.callee, &it.arguments);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.visit_arguments_after(&it.callee, &it.arguments);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        if reference.symbol_id() != Some(self.symbol) {
            return;
        }
        if self
            .scoping
            .scope_ancestors(reference.scope_id())
            .any(|ancestor| self.scoping.scope_flags(ancestor).is_with())
        {
            self.under_with = true;
        }
        if reference.is_write() {
            self.note_write();
        } else {
            self.note_read(it.span);
        }
    }
}

fn var_scope(scoping: &Scoping, scope: ScopeId) -> Option<ScopeId> {
    scoping
        .scope_ancestors(scope)
        .find(|ancestor| scoping.scope_flags(*ancestor).is_var())
}

fn under(scoping: &Scoping, scope: ScopeId, root: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| ancestor == root)
}

fn local_to_a_function(scoping: &Scoping, symbol: SymbolId) -> bool {
    let Some(home) = var_scope(scoping, scoping.symbol_scope_id(symbol)) else {
        return false;
    };
    if scoping.scope_flags(home).is_top() {
        return false;
    }
    !(0..scoping.scopes_len())
        .map(ScopeId::new)
        .filter(|scope| scoping.scope_flags(*scope).contains_direct_eval())
        .any(|scope| under(scoping, scope, home))
}

pub(super) fn ordered_values(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
) -> FxHashMap<Span, Span> {
    if !local_to_a_function(scoping, symbol) {
        return FxHashMap::default();
    }
    let mut scan = OrderScan {
        scoping,
        symbol,
        boundaries: Vec::new(),
        next_id: 0,
        frames: Vec::new(),
        declared: None,
        under_with: false,
        aliased: false,
        writers: Vec::new(),
        candidates: FxHashMap::default(),
    };
    scan.visit_program(program);
    let Some(declared) = scan.declared.filter(|_| !scan.under_with && !scan.aliased) else {
        return FxHashMap::default();
    };
    let closed =
        external_write_order_trusted() || scan.writers.iter().all(|boundary| *boundary == declared);
    if !closed {
        return FxHashMap::default();
    }
    scan.candidates
        .into_iter()
        .map(|(reference, candidate)| (reference, candidate.value))
        .collect()
}
