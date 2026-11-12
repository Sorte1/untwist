use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashMap;

use crate::probe::{expression_contains, holds_own_scope};

use super::{decoder_call, is_splittable_branch};

pub(crate) struct TernaryParts<'k, 'a> {
    pub tests: Vec<&'k Expression<'a>>,
    pub leaves: Vec<&'k Expression<'a>>,
}

pub(crate) fn ternary_argument<'k, 'a>(
    call: &'k CallExpression<'a>,
) -> Option<&'k ConditionalExpression<'a>> {
    match call.arguments.as_slice() {
        [Argument::ConditionalExpression(conditional)] => Some(conditional),
        _ => None,
    }
}

pub(crate) fn ternary_parts<'k, 'a>(call: &'k CallExpression<'a>) -> Option<TernaryParts<'k, 'a>> {
    let conditional = ternary_argument(call)?;
    if !is_splittable_branch(&conditional.consequent)
        || !is_splittable_branch(&conditional.alternate)
    {
        return None;
    }
    let mut parts = TernaryParts {
        tests: Vec::new(),
        leaves: Vec::new(),
    };
    parts.tests.push(&conditional.test);
    for branch in [&conditional.consequent, &conditional.alternate] {
        split_branch(branch, &mut parts);
    }
    let scoped = parts
        .tests
        .iter()
        .any(|test| expression_contains(test, holds_own_scope));
    (!scoped).then_some(parts)
}

fn split_branch<'k, 'a>(expr: &'k Expression<'a>, parts: &mut TernaryParts<'k, 'a>) {
    match expr.without_parentheses() {
        Expression::ConditionalExpression(conditional) => {
            parts.tests.push(&conditional.test);
            split_branch(&conditional.consequent, parts);
            split_branch(&conditional.alternate, parts);
        }
        leaf => parts.leaves.push(leaf),
    }
}

pub(crate) struct TernaryEdits<'a> {
    pub built: FxHashMap<Span, Expression<'a>>,
    pub leaves: Vec<Span>,
}

struct Groups<'s, 'a> {
    alloc: &'a Allocator,
    edits: &'s mut TernaryEdits<'a>,
}

impl<'a> Groups<'_, 'a> {
    fn group(&mut self, it: &Expression<'a>, call: &CallExpression<'a>) {
        let Some(parts) = ternary_parts(call) else {
            return;
        };
        let spans: Vec<Span> = parts.leaves.iter().map(|leaf| leaf.span()).collect();
        if !spans.iter().any(|span| self.edits.built.contains_key(span)) {
            return;
        }
        if !spans.iter().all(|span| self.edits.built.contains_key(span)) {
            for span in &spans {
                self.edits.built.remove(span);
            }
            return;
        }
        let Some(argument) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let mut replacement = argument.clone_in(self.alloc);
        let mut swap = Swap {
            built: &mut self.edits.built,
            swapped: Vec::new(),
        };
        swap.visit_expression(&mut replacement);
        let swapped = swap.swapped;
        self.edits.leaves.extend(swapped);
        self.edits.built.insert(it.span(), replacement);
    }
}

impl<'a> VisitJs<'a> for Groups<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        walk_js::walk_expression(self, it);
        if let Some((_, call)) = decoder_call(it) {
            self.group(it, call);
        }
    }
}

struct Swap<'s, 'a> {
    built: &'s mut FxHashMap<Span, Expression<'a>>,
    swapped: Vec<Span>,
}

impl<'a> VisitMut<'a> for Swap<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(value) = self.built.remove(&it.span()) {
            self.swapped.push(it.span());
            *it = value;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

pub(crate) fn group_ternaries<'a>(
    program: &Program<'a>,
    built: FxHashMap<Span, Expression<'a>>,
    alloc: &'a Allocator,
) -> TernaryEdits<'a> {
    let mut edits = TernaryEdits {
        built,
        leaves: Vec::new(),
    };
    if !edits.built.is_empty() {
        Groups {
            alloc,
            edits: &mut edits,
        }
        .visit_program(program);
    }
    edits
}
