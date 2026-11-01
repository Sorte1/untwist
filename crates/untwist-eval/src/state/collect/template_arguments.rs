use oxc_ast::ast::{Expression, Function, MemberExpression};
use oxc_span::{GetSpan, Span};

use super::judge::{is_bare_arguments, Walker};
use super::scope::FrameKind;
use super::TriggerKind;

fn literal_index(expr: &Expression<'_>) -> Option<usize> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(literal)
            if literal.value >= 0.0 && literal.value.fract() == 0.0 =>
        {
            Some(literal.value as usize)
        }
        _ => None,
    }
}

pub(super) struct ArgumentScope {
    first: usize,
    count: usize,
    arguments: Option<usize>,
    strict: bool,
    outer: bool,
    reads: bool,
}

impl<'a> Walker<'a, '_> {
    pub(super) fn enter_arguments_scope(&mut self, function: &Function<'a>, first: usize) {
        let outer = self.strict;
        self.strict = outer || function.has_use_strict_directive();
        let binding = self
            .frames
            .last()
            .and_then(|frame| frame.names.get("arguments").copied());
        let declared = binding.is_some_and(|binding| self.bindings[binding].declarations != 0);
        if declared && !self.strict {
            self.push_trigger(Span::default(), TriggerKind::Unrecognized);
        }
        let arguments = binding.filter(|_| !declared);
        self.argument_scopes.push(ArgumentScope {
            first,
            count: function.params.items.len(),
            arguments,
            strict: self.strict,
            outer,
            reads: false,
        });
    }

    pub(super) fn leave_arguments_scope(&mut self) {
        let Some(scope) = self.argument_scopes.pop() else {
            return;
        };
        self.strict = scope.outer;
        if scope.strict || !scope.reads {
            return;
        }
        let end = (scope.first + scope.count).min(self.bindings.len());
        let written = self.bindings[scope.first.min(end)..end]
            .iter()
            .any(|binding| binding.writes > 0 || binding.declarations > 1);
        if written {
            self.push_trigger(Span::default(), TriggerKind::Unrecognized);
        }
    }

    pub(super) fn note_arguments_read(&mut self, binding: usize, span: Span) {
        if self.arguments_exempt == Some(span) {
            return;
        }
        let scope = self
            .argument_scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.arguments == Some(binding));
        if let Some(scope) = scope {
            scope.reads = true;
        }
    }

    pub(super) fn note_inner_parameters(&mut self, function: &Function<'a>, first: usize) {
        if self.outer_arity.is_some() {
            self.inner_params = (first, function.params.items.len());
        }
    }

    pub(super) fn note_index_read(&mut self, member: &MemberExpression<'a>) {
        let (Some(arity), MemberExpression::ComputedMemberExpression(computed)) =
            (self.outer_arity, member)
        else {
            return;
        };
        let in_range = literal_index(&computed.expression).is_some_and(|index| index < arity);
        if in_range && is_bare_arguments(&computed.object) {
            self.index_read = Some(computed.object.without_parentheses().span());
        }
    }

    pub(super) fn check_template_arguments(&mut self, binding: usize, span: Span) {
        if self.outer_arity.is_none()
            || self.binding_names.get(binding).copied() != Some("arguments")
            || self.frame_kind_of(binding) != FrameKind::Call
            || self.arguments_exempt == Some(span)
        {
            return;
        }
        if self.index_read == Some(span) {
            self.inner_reads_arguments = true;
            return;
        }
        self.push_trigger(span, TriggerKind::Unrecognized);
    }

    pub(super) fn judge_parameter_writes(&mut self) {
        let (first, count) = self.inner_params;
        let end = (first + count).min(self.bindings.len());
        let written = self.bindings[first.min(end)..end]
            .iter()
            .any(|binding| binding.writes > 0);
        if self.inner_reads_arguments && written {
            self.push_trigger(Span::default(), TriggerKind::Unrecognized);
        }
    }
}
