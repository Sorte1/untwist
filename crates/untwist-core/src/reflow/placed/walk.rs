use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js_mut, VisitJsMut};
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::scope::ScopeFlags;

use super::super::guard_key::guard_key;
use super::super::guards::{
    fixed_point_text, guard_candidate, handed_out, holds_a_line_terminator, may_hold_a_guard,
    statement_guard, GuardRecord,
};
use crate::guard_shape::{holder_read_target, may_be_a_guard, read_target};
use crate::print::{function_text, pretty_function_text};

pub(super) const MARKER: &str = "\u{E000}";
const PLACED: Span = Span::new(u32::MAX, u32::MAX);

pub(super) struct Shape {
    pub(super) count: usize,
    pub(super) elided: bool,
    pub(super) wrapped: bool,
    pub(super) objects: Vec<usize>,
}

pub(super) enum Event {
    Enter,
    Exit(usize),
    Candidate,
}

pub(super) struct Target {
    pub(super) text: String,
    pub(super) prefix: &'static str,
    pub(super) printed: String,
}

#[derive(Default)]
pub(super) struct Doubt {
    pub(super) target: bool,
    pub(super) unbroken: bool,
}

struct Pending<'a> {
    target: *const Function<'a>,
    compacted: Option<String>,
}

pub(super) struct Placer<'a> {
    alloc: &'a Allocator,
    pub(super) shapes: Vec<Shape>,
    pub(super) events: Vec<Event>,
    pub(super) targets: Vec<Target>,
    pub(super) record: GuardRecord,
    pub(super) doubt: Doubt,
    candidate: bool,
    method: bool,
    pending: Option<Pending<'a>>,
}

fn placed<'a>(alloc: &'a Allocator, name: &'static str) -> BindingIdentifier<'a> {
    BindingIdentifier::new(PLACED, Ident::from(name), &AstBuilder::new(alloc))
}

fn wrapped(element: &ArrayExpressionElement<'_>) -> bool {
    let Some(expression) = element.as_expression() else {
        return false;
    };
    match expression.without_parentheses() {
        Expression::SequenceExpression(_) => true,
        Expression::FunctionExpression(function) => function.pife,
        Expression::ArrowFunctionExpression(arrow) => arrow.pife,
        _ => false,
    }
}

fn shape(array: &ArrayExpression<'_>) -> Shape {
    Shape {
        count: array.elements.len(),
        elided: array
            .elements
            .iter()
            .any(|element| matches!(element, ArrayExpressionElement::Elision(_))),
        wrapped: array.elements.iter().any(wrapped),
        objects: array
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| is_an_object(element))
            .map(|(index, _)| index)
            .collect(),
    }
}

fn is_an_object(element: &ArrayExpressionElement<'_>) -> bool {
    element.as_expression().is_some_and(|expression| {
        matches!(
            expression.without_parentheses(),
            Expression::ObjectExpression(_)
        )
    })
}

fn printed_by_its_keyword(function: &Function<'_>) -> bool {
    function.id.is_none() && !function.generator && !function.declare
}

impl<'a> Placer<'a> {
    pub(super) fn new(alloc: &'a Allocator) -> Self {
        Placer {
            alloc,
            shapes: Vec::new(),
            events: Vec::new(),
            targets: Vec::new(),
            record: GuardRecord::default(),
            doubt: Doubt::default(),
            candidate: false,
            method: false,
            pending: None,
        }
    }

    fn note_candidate(&mut self, function: &Function<'a>) {
        if !self.candidate && may_be_a_guard(function) && read_target(function).is_some() {
            self.candidate = true;
            self.events.push(Event::Candidate);
        }
    }

    fn settle(&mut self, init: &Expression<'a>) {
        if self.pending.is_some() {
            return;
        }
        let Some(function) = guard_candidate(init) else {
            return;
        };
        if !may_be_a_guard(function) {
            return;
        }
        if let Some(target) = read_target(function) {
            self.hold(target);
        }
    }

    fn settle_handed_out(&mut self, function: &Function<'a>) {
        if self.pending.is_some() {
            return;
        }
        if let Some(target) = handed_out(function) {
            self.hold(target);
        }
    }

    fn hold(&mut self, target: &Function<'a>) {
        let compacted = self.compacts(target);
        self.pending = Some(Pending {
            target: std::ptr::from_ref(target),
            compacted,
        });
    }

    fn compacts(&mut self, target: &Function<'a>) -> Option<String> {
        let printed = function_text(target, false);
        if !holds_a_line_terminator(&printed) {
            self.doubt.unbroken = true;
            return None;
        }
        let compact = printed.replace('\n', "");
        let key = guard_key(&compact);
        let fixed = fixed_point_text(target, &compact, true);
        match fixed {
            Some(_) => self.record.note_compacted(key),
            None => self.record.note_kept(key),
        }
        fixed
    }

    fn pending_target(&self, function: &Function<'a>) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| std::ptr::eq(pending.target, function))
    }

    fn name(&mut self, function: &mut Function<'a>, target: bool) {
        let method = std::mem::take(&mut self.method);
        let compacted = target
            && self
                .pending
                .as_ref()
                .is_some_and(|pending| pending.compacted.is_some());
        if method || !printed_by_its_keyword(function) {
            self.doubt.target |= compacted;
            return;
        }
        function.id = Some(placed(self.alloc, if compacted { MARKER } else { "" }));
    }

    fn finish_target(&mut self, function: &Function<'a>) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        if let Some(text) = pending.compacted {
            let prefix = if function.r#async {
                "async function "
            } else {
                "function "
            };
            self.targets.push(Target {
                text,
                prefix,
                printed: pretty_function_text(function),
            });
        }
    }
}

impl<'a> VisitJsMut<'a> for Placer<'a> {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        self.settle_handed_out(it);
        let target = self.pending_target(it);
        self.note_candidate(it);
        self.name(it, target);
        walk_js_mut::walk_function(self, it, flags);
        if target {
            self.finish_target(it);
        }
    }

    fn visit_object_property(&mut self, it: &mut ObjectProperty<'a>) {
        let method = it.method || it.kind != PropertyKind::Init;
        self.visit_property_key(&mut it.key);
        self.method = method && matches!(it.value, Expression::FunctionExpression(_));
        self.visit_expression(&mut it.value);
    }

    fn visit_method_definition(&mut self, it: &mut MethodDefinition<'a>) {
        self.visit_decorators(&mut it.decorators);
        self.visit_property_key(&mut it.key);
        self.method = true;
        self.visit_function(&mut it.value, ScopeFlags::Function);
    }

    fn visit_array_expression(&mut self, it: &mut ArrayExpression<'a>) {
        if it.elements.len() <= 2 {
            walk_js_mut::walk_array_expression(self, it);
            return;
        }
        let index = self.shapes.len();
        self.shapes.push(shape(it));
        self.events.push(Event::Enter);
        walk_js_mut::walk_array_expression(self, it);
        self.events.push(Event::Exit(index));
    }

    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        if may_hold_a_guard(it.init.as_ref()) {
            if let Some(init) = &it.init {
                self.settle(init);
            }
        }
        walk_js_mut::walk_variable_declarator(self, it);
    }

    fn visit_expression_statement(&mut self, it: &mut ExpressionStatement<'a>) {
        if let Some(init) = statement_guard(it) {
            self.settle(init);
        }
        walk_js_mut::walk_expression_statement(self, it);
    }

    fn visit_object_expression(&mut self, it: &mut ObjectExpression<'a>) {
        if self.pending.is_none() {
            if let Some(target) = holder_read_target(it) {
                if !self.candidate {
                    self.candidate = true;
                    self.events.push(Event::Candidate);
                }
                let compacted = self.compacts(target);
                self.pending = Some(Pending {
                    target: std::ptr::from_ref(target),
                    compacted,
                });
            }
        }
        walk_js_mut::walk_object_expression(self, it);
    }
}

struct Restorer;

impl<'a> VisitJsMut<'a> for Restorer {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        if it.id.as_ref().is_some_and(|id| id.span == PLACED) {
            it.id = None;
        }
        walk_js_mut::walk_function(self, it, flags);
    }
}

pub(super) fn restore(program: &mut Program<'_>) {
    Restorer.visit_program(program);
}
