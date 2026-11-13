use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{GetSpan, Span};
use untwist_core::ast_util::edit::ExprEdits;

pub(super) struct Swapped<'a> {
    planned: Span,
    replacement: Span,
    original: Option<Expression<'a>>,
}

struct SwapIn<'e, 'a> {
    edits: &'e mut ExprEdits<'a>,
    taken: Vec<Swapped<'a>>,
}

impl<'a> VisitMut<'a> for SwapIn<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let planned = it.span();
        let Some(replacement) = self.edits.take(planned) else {
            walk_mut::walk_expression(self, it);
            return;
        };
        let at = replacement.span();
        let original = std::mem::replace(it, replacement);
        self.taken.push(Swapped {
            planned,
            replacement: at,
            original: Some(original),
        });
    }
}

pub(super) fn swap_in<'a>(
    arguments: &mut AVec<'a, Argument<'a>>,
    edits: &mut ExprEdits<'a>,
) -> Vec<Swapped<'a>> {
    if edits.is_empty() {
        return Vec::new();
    }
    let mut swap = SwapIn {
        edits,
        taken: Vec::new(),
    };
    for argument in arguments.iter_mut() {
        swap.visit_argument(argument);
    }
    swap.taken
}

struct SwapBack<'e, 'a> {
    edits: &'e mut ExprEdits<'a>,
    taken: Vec<Swapped<'a>>,
    absorbed: bool,
}

impl<'a> VisitMut<'a> for SwapBack<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let at = it.span();
        let Some(swapped) = self
            .taken
            .iter_mut()
            .find(|swapped| swapped.replacement == at && swapped.original.is_some())
        else {
            walk_mut::walk_expression(self, it);
            return;
        };
        let Some(original) = swapped.original.take() else {
            return;
        };
        let replacement = std::mem::replace(it, original);
        if !self.absorbed {
            self.edits.put_back(swapped.planned, replacement);
        }
    }
}

pub(super) fn swap_back<'a>(
    arguments: &mut AVec<'a, Argument<'a>>,
    taken: Vec<Swapped<'a>>,
    edits: &mut ExprEdits<'a>,
    absorbed: bool,
) {
    if taken.is_empty() {
        return;
    }
    let mut swap = SwapBack {
        edits,
        taken,
        absorbed,
    };
    for argument in arguments.iter_mut() {
        swap.visit_argument(argument);
    }
}
