use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_span::Span;

use super::holds::Hold;
use super::sites::Sites;
use super::source::is_ambient_intrinsic;

pub const QUIET_INTRINSICS: [&str; 39] = [
    "Array.isArray",
    "JSON.parse",
    "Math.abs",
    "Math.ceil",
    "Math.floor",
    "Math.max",
    "Math.min",
    "Math.pow",
    "Math.round",
    "Math.sign",
    "Math.sqrt",
    "Math.trunc",
    "Number",
    "Number.isFinite",
    "Number.isInteger",
    "Number.isNaN",
    "Number.isSafeInteger",
    "Object.is",
    "Promise.all",
    "Promise.allSettled",
    "Promise.any",
    "Promise.race",
    "Promise.resolve",
    "String.fromCharCode",
    "String.fromCodePoint",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "Math.cbrt",
    "Math.hypot",
    "Math.log",
    "Math.exp",
    "Math.log2",
    "Math.log10",
];

const PROMISE_LOOKUPS: [&str; 2] = ["then", "resolve"];

pub const QUIET_MODULES: [&str; 4] = ["path", "node:path", "url", "node:url"];

const LOADER: &str = "require";

fn static_key<'a>(member: &MemberExpression<'a>) -> Option<&'a str> {
    match member {
        MemberExpression::StaticMemberExpression(m) => Some(m.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(m) => match m.expression.without_parentheses() {
            Expression::StringLiteral(text) => text.value.as_str(),
            _ => None,
        },
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn quiet_path(base: &str, key: Option<&str>) -> bool {
    QUIET_INTRINSICS.iter().any(|path| match key {
        Some(key) => path
            .split_once('.')
            .is_some_and(|(b, k)| b == base && k == key),
        None => *path == base,
    })
}

fn required_module<'a>(init: &'a Expression<'a>) -> Option<&'a str> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.without_parentheses() else {
        return None;
    };
    if callee.name != LOADER || call.arguments.len() != 1 {
        return None;
    }
    match call.arguments[0].as_expression()?.without_parentheses() {
        Expression::StringLiteral(text) => text.value.as_str(),
        _ => None,
    }
}

pub(super) enum Quiet<'a> {
    Intrinsic(&'a str, Option<&'a str>),
    Outside(Option<&'a str>),
}

impl<'a> Sites<'_, 'a> {
    fn unbound_name(&self, expr: &Expression<'a>) -> Option<&'a str> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        self.fresh.symbol(id).is_none().then_some(id.name.as_str())
    }

    fn quiet_module(&self, base: &Expression<'a>) -> bool {
        let Expression::Identifier(id) = base.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.fresh.symbol(id) else {
            return false;
        };
        !self.fresh.scoping.symbol_is_mutated(symbol)
            && self
                .fresh
                .literal_init(id)
                .and_then(required_module)
                .is_some_and(|module| QUIET_MODULES.contains(&module))
    }

    pub(super) fn quiet_callee(&self, callee: &Expression<'a>) -> Option<Quiet<'a>> {
        let callee = callee.without_parentheses();
        if let Some(name) = self.unbound_name(callee) {
            if name == LOADER {
                return Some(Quiet::Outside(None));
            }
            return (is_ambient_intrinsic(name) && quiet_path(name, None))
                .then_some(Quiet::Intrinsic(name, None));
        }
        let member = callee.as_member_expression()?;
        let key = static_key(member)?;
        if let Some(base) = self.unbound_name(member.object()) {
            return (is_ambient_intrinsic(base) && quiet_path(base, Some(key)))
                .then_some(Quiet::Intrinsic(base, Some(key)));
        }
        self.quiet_module(member.object())
            .then_some(Quiet::Outside(Some(key)))
    }

    pub(super) fn judge_quiet(&mut self, quiet: Quiet<'a>, span: Span) {
        match quiet {
            Quiet::Intrinsic(base, key) => {
                self.out.ambient.push((span, base, true));
                if let Some(key) = key {
                    self.out.observe_keyed.push((span, Cow::Borrowed(key)));
                }
                if base == "Promise" {
                    for lookup in PROMISE_LOOKUPS {
                        self.out.observe_keyed.push((span, Cow::Borrowed(lookup)));
                    }
                }
            }
            Quiet::Outside(key) => {
                self.out.ambient.push((span, LOADER, true));
                if let Some(key) = key {
                    self.out.observe_keyed.push((span, Cow::Borrowed(key)));
                }
                self.out.outside_premise.get_or_insert(span);
            }
        }
    }

    pub(super) fn hands_global(&self, arguments: &[Argument<'a>]) -> bool {
        arguments
            .iter()
            .any(|argument| match argument.as_expression() {
                Some(expr) => {
                    self.maybe_global(expr) || !matches!(self.argument_hold(expr), Hold::No)
                }
                None => true,
            })
    }
}
