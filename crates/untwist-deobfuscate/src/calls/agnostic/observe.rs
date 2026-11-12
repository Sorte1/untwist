use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;

use super::forward::{forwarding_call, passing_call};
use super::fresh::array_element;
use super::holds::Hold;
use super::reach::VALUE_DEPTH;
use super::roles::KEY_DEPTH;
use super::sites::Sites;
use super::source::is_ambient_intrinsic;
use super::AKey;
use crate::value_flow::{flow_rule_disabled, OBSERVER_LIST};
use untwist_core::host_globals::is_host_global;

const ALIAS_DEPTH: u8 = 4;
const FORWARD_DEPTH: u8 = 8;

fn element_string<'a>(array: &'a ArrayExpression<'a>, index: f64) -> Option<&'a str> {
    match array_element(array, index)?.without_parentheses() {
        Expression::StringLiteral(text) => text.value.as_str(),
        _ => None,
    }
}

const RECEIVER_OBSERVERS: [&str; 4] = [
    "hasOwnProperty",
    "propertyIsEnumerable",
    "__lookupGetter__",
    "__lookupSetter__",
];

impl<'a> Sites<'_, 'a> {
    fn observing(&self, span: Span) -> bool {
        self.watch.observe
            && !self.watch.props.is_empty()
            && self.out.observed.is_none()
            && self.out.reach.is_none()
            && !self.in_machinery(span)
    }

    fn observed_key(&self, expr: &Expression<'a>) -> bool {
        self.key_set(expr).iter().any(|key| match key {
            AKey::Named(name) => self.watch.has_prop(name),
            AKey::Index => false,
            AKey::Filtered(filter) => self.filter_hits(*filter),
            AKey::Pending(_) | AKey::Unknown => true,
        })
    }

    fn observe_base(&mut self, base: &Expression<'a>, span: Span) {
        if let Some(name) = self.ambient_base(base) {
            self.out.ambient.push((span, name, true));
        } else if self.maybe_global(base) {
            self.out.observed.get_or_insert(span);
        }
    }

    pub(super) fn observe_enumerated(&mut self, base: &Expression<'a>, span: Span) {
        if self.observing(span) {
            self.observe_base(base, span);
        }
    }

    fn effective_callee<'k>(&self, callee: &'k Expression<'a>) -> &'k Expression<'a> {
        let mut current = callee.without_parentheses();
        while let Expression::SequenceExpression(seq) = current {
            match seq.expressions.last() {
                Some(last) => current = last.without_parentheses(),
                None => break,
            }
        }
        current
    }

    fn root_identifier<'k>(&self, expr: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => Some(id),
            other => self.root_identifier(other.as_member_expression()?.object()),
        }
    }

    fn name_assigned(&self, name: &str) -> bool {
        let scoping = self.fresh.scoping;
        self.assigned_globals
            .get_or_init(|| {
                scoping
                    .root_unresolved_references()
                    .iter()
                    .filter(|(_, references)| {
                        references
                            .iter()
                            .any(|reference| scoping.get_reference(*reference).is_write())
                    })
                    .map(|(name, _)| name.as_str())
                    .collect()
            })
            .contains(name)
    }

    fn throws_reference(&mut self, expr: &Expression<'a>, span: Span) -> bool {
        let Some(id) = self.root_identifier(expr) else {
            return false;
        };
        let name = id.name.as_str();
        let thrown = self.fresh.symbol(id).is_none()
            && name != "arguments"
            && !is_host_global(name)
            && !is_ambient_intrinsic(name)
            && !self.name_assigned(name);
        if thrown {
            self.out.observe_keyed.push((span, Cow::Borrowed(name)));
        }
        thrown
    }

    fn evaluation_throws(
        &mut self,
        eff: &Expression<'a>,
        arguments: &[Argument<'a>],
        span: Span,
    ) -> bool {
        self.throws_reference(eff, span)
            || arguments
                .iter()
                .filter_map(Argument::as_expression)
                .any(|argument| self.throws_reference(argument, span))
    }

    pub(super) fn maybe_global(&self, base: &Expression<'a>) -> bool {
        self.ambient_base(base).is_none()
            && !self.non_global(base, VALUE_DEPTH)
            && !self.watched_element(base)
            && self.text_value(base, KEY_DEPTH).is_none()
    }

    fn observe_argument(&mut self, expr: &Expression<'a>, span: Span) {
        if self.ambient_base(expr).is_some() || self.maybe_global(expr) {
            self.observe_base(expr, span);
            return;
        }
        match self.argument_hold(expr) {
            Hold::No => {}
            Hold::Stored => {
                self.out.observe_stored.get_or_insert(span);
            }
            Hold::Yes => {
                self.out.observed.get_or_insert(span);
            }
        }
    }

    pub(super) fn watched_text(&self, expr: &Expression<'a>) -> Option<&'a str> {
        let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
            return None;
        };
        let Expression::NumericLiteral(index) = member.expression.without_parentheses() else {
            return None;
        };
        if !self.watched_element(expr) {
            return None;
        }
        let Expression::Identifier(id) = member.object.without_parentheses() else {
            return None;
        };
        let mut init = self.fresh.literal_init(id)?;
        for _ in 0..ALIAS_DEPTH {
            match init.without_parentheses() {
                Expression::Identifier(next) => init = self.fresh.literal_init(next)?,
                Expression::ArrayExpression(array) => return element_string(array, index.value),
                _ => return None,
            }
        }
        None
    }

    pub(super) fn watched_element(&self, expr: &Expression<'a>) -> bool {
        let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(id) = member.object.without_parentheses() else {
            return false;
        };
        let scoping = self.fresh.scoping;
        matches!(
            member.expression.without_parentheses(),
            Expression::NumericLiteral(_)
        ) && self.watch.has_name(id.name.as_str())
            && self.fresh.literal_init(id).is_some()
            && self
                .fresh
                .symbol(id)
                .is_some_and(|symbol| scoping.symbol_scope_id(symbol) == scoping.root_scope_id())
    }

    pub(super) fn observe_in(&mut self, binary: &BinaryExpression<'a>) {
        if binary.operator == BinaryOperator::In
            && self.observing(binary.span)
            && self.observed_key(&binary.left)
        {
            self.observe_base(&binary.right, binary.span);
        }
    }

    pub(super) fn observe_arguments(
        &mut self,
        span: Span,
        callee: &Expression<'a>,
        arguments: &[Argument<'a>],
    ) {
        if !self.observing(span) || self.forward_calls.contains(&span) {
            return;
        }
        if let Some(func) = self.fresh.sealed_function(callee) {
            if self.forwarder_call(func, FORWARD_DEPTH).is_some() {
                self.observe_forwarded(span, arguments);
                return;
            }
        }
        if self.fresh.is_user_function(callee)
            || self.holds.calls_only_user_functions(callee, arguments)
        {
            return;
        }
        if let Some(key) = self.applies_user_function(callee) {
            self.out.observe_keyed.push((span, Cow::Borrowed(key)));
            return;
        }
        if flow_rule_disabled(OBSERVER_LIST) || self.quiet_collection_call(callee) {
            return;
        }
        if let Some(quiet) = self.quiet_callee(callee) {
            if self.hands_global(arguments) {
                self.judge_quiet(quiet, span);
                return;
            }
        }
        let eff = self.effective_callee(callee);
        if self.evaluation_throws(eff, arguments, span) {
            return;
        }
        for (index, argument) in arguments.iter().enumerate() {
            match argument.as_expression() {
                Some(expr) => {
                    if !self.arg_consumed_safely(callee, arguments, index) {
                        self.observe_argument(expr, span);
                    }
                }
                None => {
                    self.out.observed.get_or_insert(span);
                }
            }
        }
        if let Some(member) = callee.without_parentheses().as_member_expression() {
            self.observe_receiver(member, arguments, span);
        }
    }

    pub(super) fn note_forwarders(&mut self, id: &BindingPattern<'a>, init: &Expression<'a>) {
        if !self.watch.observe {
            return;
        }
        let (BindingPattern::BindingIdentifier(id), Expression::ObjectExpression(object)) =
            (id, init.without_parentheses())
        else {
            return;
        };
        if !id
            .symbol_id
            .get()
            .is_some_and(|symbol| self.fresh.is_sealed(symbol))
        {
            return;
        }
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(p) = property else {
                continue;
            };
            if let Expression::FunctionExpression(func) = p.value.without_parentheses() {
                if let Some(call) = self.forwarder_call(func, FORWARD_DEPTH) {
                    self.forward_calls.insert(call);
                }
            }
        }
    }

    fn forwarder_call(&self, func: &Function<'a>, depth: u8) -> Option<Span> {
        if let Some(call) = forwarding_call(self.fresh.scoping, func) {
            return Some(call);
        }
        if depth == 0 {
            return None;
        }
        let call = passing_call(self.fresh.scoping, func)?;
        let inner = self.fresh.sealed_function(&call.callee)?;
        self.forwarder_call(inner, depth - 1).map(|_| call.span)
    }

    fn observe_forwarded(&mut self, span: Span, arguments: &[Argument<'a>]) {
        let Some((first, rest)) = arguments.split_first() else {
            return;
        };
        match first.as_expression() {
            Some(target) => self.observe_arguments(span, target, rest),
            None => {
                self.out.observed.get_or_insert(span);
            }
        }
    }

    fn observe_receiver(
        &mut self,
        member: &MemberExpression<'a>,
        arguments: &[Argument<'a>],
        span: Span,
    ) {
        let observer = self.member_keys(member).iter().any(|key| match key {
            AKey::Named(name) => RECEIVER_OBSERVERS.contains(&name.as_ref()),
            AKey::Index => false,
            AKey::Filtered(filter) => RECEIVER_OBSERVERS
                .iter()
                .any(|name| self.filter_accepts(*filter, name)),
            AKey::Pending(_) | AKey::Unknown => true,
        });
        let hits = observer
            && arguments
                .first()
                .and_then(Argument::as_expression)
                .is_none_or(|key| self.observed_key(key));
        if hits {
            self.observe_base(member.object(), span);
        }
    }

    pub(super) fn observe_template(&mut self, tagged: &TaggedTemplateExpression<'a>) {
        if !self.observing(tagged.span) || self.fresh.is_user_function(&tagged.tag) {
            return;
        }
        for expr in &tagged.quasi.expressions {
            self.observe_argument(expr, tagged.span);
        }
    }
}
