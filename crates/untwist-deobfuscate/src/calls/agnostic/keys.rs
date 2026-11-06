use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use std::borrow::Cow;
use untwist_core::ast_util::js_text::js_text;

use super::roles::*;
use super::sites::*;
use super::AKey;
use crate::proof::{key_of, PatchKey};

#[derive(Clone, Debug)]
pub(super) enum Keys<'a> {
    One(AKey<'a>),
    Many(Vec<AKey<'a>>),
}

impl<'a> Keys<'a> {
    pub(super) fn as_slice(&self) -> &[AKey<'a>] {
        self
    }
}

impl<'a> std::ops::Deref for Keys<'a> {
    type Target = [AKey<'a>];

    fn deref(&self) -> &[AKey<'a>] {
        match self {
            Keys::One(key) => std::slice::from_ref(key),
            Keys::Many(keys) => keys,
        }
    }
}

impl<'a> IntoIterator for Keys<'a> {
    type Item = AKey<'a>;
    type IntoIter = std::iter::Chain<std::option::IntoIter<AKey<'a>>, std::vec::IntoIter<AKey<'a>>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Keys::One(key) => Some(key).into_iter().chain(Vec::new()),
            Keys::Many(keys) => None.into_iter().chain(keys),
        }
    }
}

fn template_key<'a>(template: &TemplateLiteral<'a>) -> AKey<'a> {
    template
        .quasis
        .first()
        .and_then(|quasi| quasi.value.cooked)
        .map_or(AKey::Unknown, |cooked| AKey::Named(js_text(cooked)))
}

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn resolved(&self, span: Span) -> Option<AKey<'a>> {
        self.resolve
            .and_then(|map| map.get(&span))
            .and_then(|text| text.as_string())
            .map(|text| AKey::Named(Cow::Owned(text)))
    }

    pub(super) fn key_class(&self, expr: &Expression<'a>, depth: u8) -> AKey<'a> {
        if depth == 0 {
            return AKey::Unknown;
        }
        let next = depth - 1;
        let inner = expr.without_parentheses();
        if self.fresh.is_numeric(inner) {
            return AKey::Index;
        }
        match inner {
            Expression::BooleanLiteral(_) | Expression::NullLiteral(_) => AKey::Index,
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::LogicalNot | UnaryOperator::Void
                ) =>
            {
                AKey::Index
            }
            Expression::BinaryExpression(binary) if binary.operator != BinaryOperator::Addition => {
                AKey::Index
            }
            Expression::BinaryExpression(binary) => self.concat_key(binary, next),
            Expression::ArrayExpression(array) => self.array_key(array, next),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                template_key(template)
            }
            Expression::ComputedMemberExpression(_) if self.watched_text(inner).is_some() => self
                .watched_text(inner)
                .map_or(AKey::Unknown, |text| AKey::Named(Cow::Borrowed(text))),
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                match self.sealed_read_at(inner, next) {
                    Some(value) => self.key_class(value, next),
                    None if self.symbol_member_unsafe(inner) => AKey::Unknown,
                    None => self.patch_key(key_of(inner)),
                }
            }
            Expression::Identifier(id) => self.identifier_key(id, next),
            Expression::CallExpression(call) => self.call_key(call, inner, next),
            other => self.patch_key(key_of(other)),
        }
    }

    fn symbol_member_unsafe(&self, expr: &Expression<'a>) -> bool {
        let Some(member) = expr.as_member_expression() else {
            return false;
        };
        let Expression::Identifier(id) = member.object().without_parentheses() else {
            return false;
        };
        if id.name != "Symbol" {
            return false;
        }
        self.fresh.symbol(id).is_some() || !self.symbol_safe
    }

    fn patch_key(&self, key: PatchKey<'a>) -> AKey<'a> {
        match key {
            PatchKey::Named(name) => AKey::Named(Cow::Borrowed(name)),
            PatchKey::Pending(span) => self.resolved(span).unwrap_or(AKey::Pending(span)),
            PatchKey::Dynamic | PatchKey::Whole => AKey::Unknown,
        }
    }

    fn concat_key(&self, binary: &BinaryExpression<'a>, depth: u8) -> AKey<'a> {
        match (
            self.key_class(&binary.left, depth),
            self.key_class(&binary.right, depth),
        ) {
            (AKey::Named(left), AKey::Named(right)) => {
                AKey::Named(Cow::Owned(format!("{left}{right}")))
            }
            _ => AKey::Unknown,
        }
    }

    fn array_key(&self, array: &ArrayExpression<'a>, depth: u8) -> AKey<'a> {
        match array.elements.as_slice() {
            [] => AKey::Named(Cow::Borrowed("")),
            [single] => single
                .as_expression()
                .map_or(AKey::Unknown, |element| self.key_class(element, depth)),
            _ => AKey::Index,
        }
    }

    fn identifier_key(&self, id: &IdentifierReference<'a>, depth: u8) -> AKey<'a> {
        if let Some(arg) = self.substituted(id) {
            return self.key_class(arg, depth);
        }
        if let Some(key) = self.filtered_key(id) {
            return key;
        }
        if id.name == "undefined" && self.fresh.symbol(id).is_none() {
            return AKey::Named(Cow::Borrowed("undefined"));
        }
        match self.fresh.literal_init(id) {
            Some(init) => self.key_class(init, depth),
            None => AKey::Unknown,
        }
    }

    fn filtered_key(&self, id: &IdentifierReference<'a>) -> Option<AKey<'a>> {
        let filters = &self.fresh.bindings.filters;
        if filters.is_empty() {
            return None;
        }
        filters.key(self.fresh.symbol(id)?)
    }

    pub(super) fn key_origin(&self, key: &AKey<'a>) -> Origin {
        match key {
            AKey::Filtered(filter) => self
                .fresh
                .bindings
                .filters
                .origin(self.fresh.scoping, *filter),
            other => origin_of(other),
        }
    }

    pub(super) fn filter_hits(&self, filter: Span) -> bool {
        let scoping = self.fresh.scoping;
        let watched = self
            .watch
            .props
            .iter()
            .chain(self.watch.names)
            .any(|name| self.filter_accepts(filter, name));
        let source = self.fresh.bindings.filters.source();
        watched
            || scoping
                .iter_bindings_in(scoping.root_scope_id())
                .filter(|symbol| self.in_machinery(scoping.symbol_span(*symbol)))
                .any(|symbol| {
                    self.filter_accepts(filter, scoping.symbol_name(symbol))
                        || crate::stamp::original_name(source, scoping, symbol)
                            .is_none_or(|name| self.filter_accepts(filter, name))
                })
    }

    pub(super) fn filter_accepts(&self, filter: Span, name: &str) -> bool {
        self.fresh
            .bindings
            .filters
            .accepts(self.fresh.scoping, filter, name)
    }

    fn call_key(&self, call: &CallExpression<'a>, expr: &Expression<'a>, depth: u8) -> AKey<'a> {
        if let Some(key) = self.resolved(call.span) {
            return key;
        }
        if let Some(key) = self.proxy_key(call, depth) {
            return key;
        }
        if self.creates_symbol(call) {
            return AKey::Index;
        }
        self.patch_key(key_of(expr))
    }

    fn creates_symbol(&self, call: &CallExpression<'a>) -> bool {
        if call.optional || !self.symbol_safe {
            return false;
        }
        let Expression::Identifier(root) = call.callee.without_parentheses() else {
            return false;
        };
        root.name == "Symbol" && self.fresh.symbol(root).is_none()
    }

    pub(super) fn str_value(&self, expr: &Expression<'a>, depth: u8) -> Option<String> {
        let next = depth.checked_sub(1)?;
        match expr.without_parentheses() {
            Expression::StringLiteral(text) => Some(js_text(text.value).into_owned()),
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => t
                .quasis
                .first()?
                .value
                .cooked
                .map(|cooked| js_text(cooked).into_owned()),
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                Some(format!(
                    "{}{}",
                    self.str_value(&b.left, next)?,
                    self.str_value(&b.right, next)?
                ))
            }
            Expression::Identifier(id) => {
                if let Some(arg) = self.substituted(id) {
                    return self.str_value(arg, next);
                }
                let symbol = self.fresh.symbol(id)?;
                let flags = self.fresh.scoping.symbol_flags(symbol);
                if !flags.contains(oxc_syntax::symbol::SymbolFlags::BlockScopedVariable) {
                    return None;
                }
                self.str_value(self.fresh.literal_init(id)?, next)
            }
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                self.str_value(self.sealed_str_read(expr, next)?, next)
            }
            Expression::CallExpression(call) => {
                if let Some(AKey::Named(text)) = self.resolved(call.span) {
                    return Some(text.into_owned());
                }
                let call: &'a CallExpression<'a> = self.alloc(call);
                let (function, body) = self.proxy_str(call, next)?;
                self.with_params(function, call, || self.str_value(body, next))
                    .flatten()
            }
            _ => None,
        }
    }

    pub(super) fn sealed_str_read(
        &self,
        expr: &Expression<'a>,
        depth: u8,
    ) -> Option<&'a Expression<'a>> {
        let member = expr.without_parentheses().as_member_expression()?;
        let Expression::Identifier(id) = member.object().without_parentheses() else {
            return None;
        };
        let key = match member {
            MemberExpression::StaticMemberExpression(m) => m.property.name.to_string(),
            MemberExpression::ComputedMemberExpression(m) => {
                self.str_value(&m.expression, depth)?
            }
            MemberExpression::PrivateFieldExpression(_) => return None,
        };
        self.fresh.sealed_field(id, &key)
    }

    pub(super) fn static_truth(&self, test: &Expression<'a>) -> Option<bool> {
        self.static_truth_at(test, KEY_DEPTH)
    }

    fn static_truth_at(&self, test: &Expression<'a>, depth: u8) -> Option<bool> {
        let next = depth.checked_sub(1)?;
        match test.without_parentheses() {
            Expression::BinaryExpression(binary) => {
                let (Some(left), Some(right)) = (
                    self.str_value(&binary.left, next),
                    self.str_value(&binary.right, next),
                ) else {
                    return None;
                };

                match binary.operator {
                    BinaryOperator::StrictEquality | BinaryOperator::Equality => {
                        Some(left == right)
                    }
                    BinaryOperator::StrictInequality | BinaryOperator::Inequality => {
                        Some(left != right)
                    }
                    _ => None,
                }
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                match unary.argument.without_parentheses() {
                    Expression::ArrayExpression(_) | Expression::ObjectExpression(_) => Some(false),
                    other => self.static_truth_at(other, next).map(|truth| !truth),
                }
            }
            Expression::CallExpression(call) => {
                let call: &'a CallExpression<'a> = self.alloc(call);
                let (function, body) = self.proxy_str(call, next)?;
                self.with_params(function, call, || self.static_truth_at(body, next))
                    .flatten()
            }
            _ => None,
        }
    }

    pub(super) fn sealed_read_at(
        &self,
        expr: &Expression<'a>,
        depth: u8,
    ) -> Option<&'a Expression<'a>> {
        let member = expr.without_parentheses().as_member_expression()?;
        let Expression::Identifier(id) = member.object().without_parentheses() else {
            return None;
        };
        let keys = self.member_keys_at(member, depth);
        let [AKey::Named(name)] = keys.as_slice() else {
            return None;
        };
        self.fresh.sealed_field(id, name)
    }

    pub(super) fn key_set(&self, expr: &Expression<'a>) -> Keys<'a> {
        self.key_set_at(expr, KEY_DEPTH)
    }

    fn key_set_at(&self, expr: &Expression<'a>, depth: u8) -> Keys<'a> {
        let inner = expr.without_parentheses();
        let element_read = match inner {
            Expression::Identifier(id) => self.fresh.literal_init(id),
            _ => None,
        }
        .map_or(inner, Expression::without_parentheses);
        if let Expression::ComputedMemberExpression(member) = element_read {
            if let Some(elements) = self.literal_array(&member.object) {
                if self.fresh.is_numeric(&member.expression) {
                    let mut keys: Vec<AKey<'a>> = elements
                        .iter()
                        .map(|element| {
                            element
                                .as_expression()
                                .map_or(AKey::Unknown, |inner| self.key_class(inner, depth))
                        })
                        .collect();
                    keys.push(AKey::Named(Cow::Borrowed("undefined")));
                    keys.sort();
                    keys.dedup();
                    return Keys::Many(keys);
                }
            }
        }
        Keys::One(self.key_class(expr, depth))
    }

    pub(super) fn literal_array(
        &self,
        expr: &Expression<'a>,
    ) -> Option<&'a oxc_allocator::Vec<'a, ArrayExpressionElement<'a>>> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        match self.fresh.literal_init(id)?.without_parentheses() {
            Expression::ArrayExpression(array) => Some(&array.elements),
            _ => None,
        }
    }

    pub(super) fn member_keys(&self, member: &MemberExpression<'a>) -> Keys<'a> {
        self.member_keys_at(member, KEY_DEPTH)
    }

    fn member_keys_at(&self, member: &MemberExpression<'a>, depth: u8) -> Keys<'a> {
        match member {
            MemberExpression::StaticMemberExpression(m) => {
                Keys::One(AKey::Named(Cow::Borrowed(m.property.name.as_str())))
            }
            MemberExpression::ComputedMemberExpression(m) => self.key_set_at(&m.expression, depth),
            MemberExpression::PrivateFieldExpression(_) => Keys::Many(Vec::new()),
        }
    }

    pub(super) fn property_key(&self, key: &PropertyKey<'a>) -> AKey<'a> {
        match key {
            PropertyKey::StaticIdentifier(name) => AKey::Named(Cow::Borrowed(name.name.as_str())),
            PropertyKey::PrivateIdentifier(_) => AKey::Index,
            other => other
                .as_expression()
                .map_or(AKey::Unknown, |expr| self.key_class(expr, KEY_DEPTH)),
        }
    }
}
