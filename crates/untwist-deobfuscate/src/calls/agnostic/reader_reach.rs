use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};

use super::namespace_uses::{handed_back_name, NAMESPACES, NAMESPACE_KEY};
use super::reach::VALUE_DEPTH;
use super::roles::MUTATORS;
use super::sites::Sites;
use super::AKey;

const FIXED_NAMES: [&str; 2] = ["length", "lastIndex"];

const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];

const CONSTRUCTING: [&str; 2] = ["of", "from"];

fn fixed_writer(name: &str) -> bool {
    MUTATORS.contains(&name) || CONSTRUCTING.contains(&name)
}

const CONTAINER_READERS: [&str; 4] = [
    "getOwnPropertyDescriptors",
    "values",
    "entries",
    "fromEntries",
];

fn literal_name<'k>(key: Option<&'k Expression<'_>>) -> Option<&'k str> {
    match key.map(|key| key.without_parentheses()) {
        Some(Expression::StringLiteral(text)) => text.value.as_str(),
        _ => None,
    }
}

fn inert_literal_key(key: Option<&Expression<'_>>) -> bool {
    match key.map(|key| key.without_parentheses()) {
        Some(Expression::StringLiteral(text)) => text
            .value
            .as_str()
            .is_some_and(|name| !handed_back_name(name)),
        Some(Expression::NumericLiteral(_)) => true,
        _ => false,
    }
}

fn names(keys: &[AKey<'_>], list: &[&str]) -> bool {
    keys.iter()
        .any(|key| matches!(key, AKey::Named(name) if list.contains(&name.as_ref())))
}

fn unresolved(keys: &[AKey<'_>]) -> bool {
    keys.iter()
        .any(|key| matches!(key, AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown))
}

impl<'a> Sites<'_, 'a> {
    pub(super) fn reach_reader_call(
        &mut self,
        call: &CallExpression<'a>,
        member: &MemberExpression<'a>,
        keys: &[AKey<'a>],
    ) {
        if !self.watching(call.span) || self.flow.sink_of(call.span).is_none() {
            return;
        }
        let open = unresolved(keys);
        let keyed = open
            || names(keys, &["getOwnPropertyDescriptor"])
            || (names(keys, &["get"]) && self.keyed_get(call, member));
        if keyed && self.hands_back_by_key(call) {
            return self.hit_reflective(call.span);
        }
        let containers = open || names(keys, &CONTAINER_READERS);
        if containers && self.reads_a_namespace(call.arguments.first()) {
            return self.hit_reflective(call.span);
        }
        if (open || names(keys, &["assign"]))
            && call
                .arguments
                .iter()
                .skip(1)
                .any(|source| self.reads_a_namespace(Some(source)))
        {
            self.hit_reflective(call.span);
        }
    }

    pub(super) fn fixed_watched(&self) -> bool {
        FIXED_NAMES.iter().any(|name| self.watched_name(name))
    }

    pub(super) fn reach_fixed_writer_read(&mut self, keys: &[AKey<'a>], span: Span) {
        let named = keys
            .iter()
            .any(|key| matches!(key, AKey::Named(name) if fixed_writer(name)));
        if named && self.watching(span) && self.fixed_watched() {
            self.hit_reflective(span);
        }
    }

    pub(super) fn reach_fixed_writes(
        &mut self,
        call: &CallExpression<'a>,
        member: &MemberExpression<'a>,
        keys: &[AKey<'a>],
    ) {
        if !self.fixed_watched() {
            return;
        }
        let forwarded = names(keys, &FORWARDERS);
        let receivers = if forwarded && self.namespace_owner(member.object(), VALUE_DEPTH) {
            2
        } else {
            1
        };
        let hit = if forwarded {
            call.arguments
                .iter()
                .take(receivers)
                .any(|argument| self.reads_a_namespace(Some(argument)))
        } else {
            (names(keys, &MUTATORS) || names(keys, &CONSTRUCTING))
                && self.may_be_namespace(member.object())
        };
        if hit {
            self.hit_reflective(call.span);
        }
    }

    pub(super) fn reach_spread_namespace(&mut self, spread: &SpreadElement<'a>) {
        let span = spread.span();
        if self.watching(span) && self.may_be_namespace(&spread.argument) {
            self.hit_reflective(span);
        }
    }

    pub(super) fn destructures_a_namespace<'k>(
        &self,
        base: &Expression<'a>,
        rest: bool,
        mut computed: impl Iterator<Item = Option<&'k Expression<'a>>>,
    ) -> bool
    where
        'a: 'k,
    {
        (rest || computed.any(|key| !inert_literal_key(key))) && self.may_be_namespace(base)
    }

    fn hands_back_by_key(&self, call: &CallExpression<'a>) -> bool {
        if call.arguments.iter().take(2).any(Argument::is_spread) {
            return true;
        }
        let base = call.arguments.first().and_then(Argument::as_expression);
        let key = call.arguments.get(1).and_then(Argument::as_expression);
        if literal_name(key) == Some(NAMESPACE_KEY) {
            return true;
        }
        if self.fixed_watched() && literal_name(key).is_some_and(fixed_writer) {
            return true;
        }
        base.is_some_and(|base| self.may_be_namespace(base)) && !inert_literal_key(key)
    }

    fn keyed_get(&self, call: &CallExpression<'a>, member: &MemberExpression<'a>) -> bool {
        let object = member.object();
        self.namespace_owner(object, VALUE_DEPTH)
            || (call.arguments.len() > 1 && !self.proven_non_global(object))
    }

    pub(super) fn reach_loose(&mut self) {
        let bindings = self.fresh.bindings;
        let spans = match (&bindings.loose.untrusted_symbol, self.symbol_safe) {
            (Some(untrusted), false) => untrusted,
            _ => &bindings.loose.spans,
        };
        let loose = spans.iter().copied();
        let effects = bindings
            .loose
            .blocks
            .iter()
            .filter(|(_, names)| names.iter().any(|name| self.watched_name(name)))
            .map(|(span, _)| *span);
        if let Some(span) = loose.chain(effects).find(|span| self.watching(*span)) {
            self.mark_source(
                span,
                "a namespace or the global object outside the safe forms",
            );
            self.hit(span);
        }
    }

    fn reads_a_namespace(&self, argument: Option<&Argument<'a>>) -> bool {
        match argument {
            None => false,
            Some(argument) => argument
                .as_expression()
                .is_none_or(|expr| self.may_be_namespace(expr)),
        }
    }

    fn may_be_namespace(&self, expr: &Expression<'a>) -> bool {
        !self.proven_non_global(expr)
    }

    fn namespace_owner(&self, object: &Expression<'a>, depth: u8) -> bool {
        if depth == 0 {
            return false;
        }
        match object.without_parentheses() {
            Expression::Identifier(id) if self.fresh.symbol(id).is_none() => {
                NAMESPACES.contains(&id.name.as_str())
            }
            Expression::Identifier(id) => self
                .fresh
                .literal_init(id)
                .is_some_and(|init| self.namespace_owner(init, depth - 1)),
            Expression::StaticMemberExpression(member) => {
                NAMESPACES.contains(&member.property.name.as_str())
            }
            _ => false,
        }
    }
}
