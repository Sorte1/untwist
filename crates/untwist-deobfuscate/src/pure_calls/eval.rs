use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_eval::fold::{const_from_literal, fold_binary, fold_unary, Const};

use crate::bindings::resolved_symbol;

const MAX_DEPTH: u32 = 128;
const MAX_STEPS: u32 = 10_000;
const MAX_STRING: usize = 1 << 16;
const MAX_INDEX: u64 = u32::MAX as u64 - 1;
const GLOBAL_VALUES: [&str; 3] = ["undefined", "NaN", "Infinity"];
const PROTO_KEY: &str = "__proto__";

fn bare_template(template: &TemplateLiteral<'_>) -> Option<String> {
    if !template.expressions.is_empty() {
        return None;
    }
    let [quasi] = template.quasis.as_slice() else {
        return None;
    };
    if quasi.lone_surrogates {
        return None;
    }
    Some(quasi.value.cooked?.as_str().to_string())
}

pub(super) fn literal_argument(expr: &Expression<'_>) -> Option<Const> {
    match expr {
        Expression::TemplateLiteral(template) => bare_template(template).map(Const::Str),
        other => const_from_literal(other),
    }
}

fn static_key<'k>(entry: &'k ObjectProperty<'_>) -> Option<Cow<'k, str>> {
    if entry.kind != PropertyKind::Init || entry.method || entry.computed || entry.shorthand {
        return None;
    }
    let key = match &entry.key {
        PropertyKey::StaticIdentifier(id) => Cow::Borrowed(id.name.as_str()),
        PropertyKey::StringLiteral(literal) if !literal.lone_surrogates => {
            Cow::Borrowed(literal.value.as_str())
        }
        PropertyKey::NumericLiteral(literal) => {
            Cow::Owned(property_key(&Const::Num(literal.value))?)
        }
        _ => return None,
    };
    (key != PROTO_KEY).then_some(key)
}

pub(super) fn literal_data(expr: &Expression<'_>, nested: bool) -> bool {
    literal_data_at(expr, nested, 0)
}

fn literal_data_at(expr: &Expression<'_>, nested: bool, depth: u32) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let element = |inner: &Expression<'_>| {
        if nested {
            literal_data_at(inner, nested, depth + 1)
        } else {
            literal_argument(inner).is_some()
        }
    };
    match expr {
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|item| item.as_expression().is_some_and(element)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            let ObjectPropertyKind::ObjectProperty(entry) = property else {
                return false;
            };
            static_key(entry).is_some() && element(&entry.value)
        }),
        other => literal_argument(other).is_some(),
    }
}

fn property_key(value: &Const) -> Option<String> {
    match fold_binary(BinaryOperator::Addition, &Const::Str(String::new()), value)? {
        Const::Str(key) => Some(key),
        _ => None,
    }
}

fn array_index(key: &str) -> Option<usize> {
    let canonical = key == "0" || (!key.starts_with('0') && !key.is_empty());
    if !canonical || !key.bytes().all(|byte| byte.is_ascii_digit()) || key.len() > 10 {
        return None;
    }
    let index: u64 = key.parse().ok()?;
    if index > MAX_INDEX {
        return None;
    }
    usize::try_from(index).ok()
}

pub(super) struct Shape<'s> {
    pub(super) scoping: &'s Scoping,
    pub(super) params: &'s [SymbolId],
    pub(super) constants: &'s dyn Fn(SymbolId) -> bool,
    pub(super) consulted: Vec<SymbolId>,
}

impl Shape<'_> {
    pub(super) fn pure(&mut self, expr: &Expression<'_>) -> bool {
        self.pure_at(expr, 0)
    }

    fn identifier(&mut self, id: &IdentifierReference<'_>) -> bool {
        let Some(symbol) = resolved_symbol(id, self.scoping) else {
            return GLOBAL_VALUES.contains(&id.name.as_str());
        };
        if self.params.contains(&symbol) {
            return true;
        }
        if !(self.constants)(symbol) {
            return false;
        }
        if !self.consulted.contains(&symbol) {
            self.consulted.push(symbol);
        }
        true
    }

    fn pure_at(&mut self, expr: &Expression<'_>, depth: u32) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let next = depth + 1;
        match expr {
            Expression::Identifier(id) => self.identifier(id),
            Expression::ParenthesizedExpression(inner) => self.pure_at(&inner.expression, next),
            Expression::TemplateLiteral(template) => {
                template
                    .quasis
                    .iter()
                    .all(|quasi| !quasi.lone_surrogates && quasi.value.cooked.is_some())
                    && template
                        .expressions
                        .iter()
                        .all(|part| self.pure_at(part, next))
            }
            Expression::UnaryExpression(unary) => {
                unary.operator != UnaryOperator::Delete && self.pure_at(&unary.argument, next)
            }
            Expression::BinaryExpression(binary) => {
                !matches!(
                    binary.operator,
                    BinaryOperator::In | BinaryOperator::Instanceof
                ) && self.pure_at(&binary.left, next)
                    && self.pure_at(&binary.right, next)
            }
            Expression::LogicalExpression(logical) => {
                self.pure_at(&logical.left, next) && self.pure_at(&logical.right, next)
            }
            Expression::ConditionalExpression(conditional) => {
                self.pure_at(&conditional.test, next)
                    && self.pure_at(&conditional.consequent, next)
                    && self.pure_at(&conditional.alternate, next)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .iter()
                .all(|part| self.pure_at(part, next)),
            Expression::StaticMemberExpression(member) => {
                !member.optional && self.pure_at(&member.object, next)
            }
            Expression::ComputedMemberExpression(member) => {
                !member.optional
                    && self.pure_at(&member.object, next)
                    && self.pure_at(&member.expression, next)
            }
            Expression::ArrayExpression(_) | Expression::ObjectExpression(_) => {
                literal_data(expr, true)
            }
            other => literal_argument(other).is_some(),
        }
    }
}

enum Out<'p> {
    Prim(Const),
    Array(&'p ArrayExpression<'p>),
    Object(&'p ObjectExpression<'p>),
}

impl Out<'_> {
    fn prim(&self) -> Option<&Const> {
        match self {
            Out::Prim(value) => Some(value),
            _ => None,
        }
    }

    fn truthy(&self) -> Option<bool> {
        match self {
            Out::Prim(value) => match fold_unary(UnaryOperator::LogicalNot, value)? {
                Const::Bool(negated) => Some(!negated),
                _ => None,
            },
            _ => Some(true),
        }
    }
}

pub(super) struct Machine<'s, 'p> {
    scoping: &'s Scoping,
    args: FxHashMap<SymbolId, Const>,
    params: &'s [SymbolId],
    constants: &'s dyn Fn(SymbolId) -> Option<&'p Expression<'p>>,
    steps: u32,
}

impl<'s, 'p> Machine<'s, 'p> {
    pub(super) fn new(
        scoping: &'s Scoping,
        params: &'s [SymbolId],
        args: FxHashMap<SymbolId, Const>,
        constants: &'s dyn Fn(SymbolId) -> Option<&'p Expression<'p>>,
    ) -> Self {
        Self {
            scoping,
            args,
            params,
            constants,
            steps: 0,
        }
    }

    pub(super) fn result(&mut self, body: &'p Expression<'p>) -> Option<Const> {
        let value = match self.eval(body, 0)? {
            Out::Prim(value) => value,
            _ => return None,
        };
        match &value {
            Const::Undefined => None,
            Const::Num(number) if !number.is_finite() => None,
            Const::Str(text) if text.len() > MAX_STRING => None,
            _ => Some(value),
        }
    }

    fn identifier(&mut self, id: &IdentifierReference<'_>, depth: u32) -> Option<Out<'p>> {
        let Some(symbol) = resolved_symbol(id, self.scoping) else {
            return match id.name.as_str() {
                "undefined" => Some(Out::Prim(Const::Undefined)),
                "NaN" => Some(Out::Prim(Const::Num(f64::NAN))),
                "Infinity" => Some(Out::Prim(Const::Num(f64::INFINITY))),
                _ => None,
            };
        };
        if self.params.contains(&symbol) {
            let value = self.args.get(&symbol).cloned().unwrap_or(Const::Undefined);
            return Some(Out::Prim(value));
        }
        let init = (self.constants)(symbol)?;
        self.eval(init, depth + 1)
    }

    fn eval(&mut self, expr: &'p Expression<'p>, depth: u32) -> Option<Out<'p>> {
        self.steps += 1;
        if depth > MAX_DEPTH || self.steps > MAX_STEPS {
            return None;
        }
        let next = depth + 1;
        match expr {
            Expression::Identifier(id) => self.identifier(id, depth),
            Expression::ParenthesizedExpression(inner) => self.eval(&inner.expression, next),
            Expression::ArrayExpression(array) => Some(Out::Array(array)),
            Expression::ObjectExpression(object) => Some(Out::Object(object)),
            Expression::TemplateLiteral(template) => self.template(template, next),
            Expression::UnaryExpression(unary) => self.unary(unary, next),
            Expression::BinaryExpression(binary) => {
                let left = self.eval(&binary.left, next)?;
                let right = self.eval(&binary.right, next)?;
                let value = fold_binary(binary.operator, left.prim()?, right.prim()?)?;
                Some(Out::Prim(value))
            }
            Expression::LogicalExpression(logical) => self.logical(logical, next),
            Expression::ConditionalExpression(conditional) => {
                let test = self.eval(&conditional.test, next)?.truthy()?;
                let branch = if test {
                    &conditional.consequent
                } else {
                    &conditional.alternate
                };
                self.eval(branch, next)
            }
            Expression::SequenceExpression(sequence) => {
                let mut last = None;
                for part in &sequence.expressions {
                    last = Some(self.eval(part, next)?);
                }
                last
            }
            Expression::StaticMemberExpression(member) if !member.optional => {
                let object = self.eval(&member.object, next)?;
                self.get(object, member.property.name.as_str(), next)
            }
            Expression::ComputedMemberExpression(member) if !member.optional => {
                let object = self.eval(&member.object, next)?;
                let key = self.eval(&member.expression, next)?;
                let key = property_key(key.prim()?)?;
                self.get(object, &key, next)
            }
            other => literal_argument(other).map(Out::Prim),
        }
    }

    fn template(&mut self, template: &'p TemplateLiteral<'p>, depth: u32) -> Option<Out<'p>> {
        let mut text = Const::Str(String::new());
        for (index, quasi) in template.quasis.iter().enumerate() {
            if quasi.lone_surrogates {
                return None;
            }
            let cooked = Const::Str(quasi.value.cooked?.as_str().to_string());
            text = fold_binary(BinaryOperator::Addition, &text, &cooked)?;
            if let Some(part) = template.expressions.get(index) {
                let value = self.eval(part, depth)?;
                text = fold_binary(BinaryOperator::Addition, &text, value.prim()?)?;
            }
        }
        Some(Out::Prim(text))
    }

    fn unary(&mut self, unary: &'p UnaryExpression<'p>, depth: u32) -> Option<Out<'p>> {
        let argument = self.eval(&unary.argument, depth)?;
        let value = match (unary.operator, &argument) {
            (UnaryOperator::Delete, _) => return None,
            (UnaryOperator::Void, _) => Const::Undefined,
            (UnaryOperator::Typeof, Out::Array(_) | Out::Object(_)) => {
                Const::Str("object".to_string())
            }
            (UnaryOperator::LogicalNot, _) => Const::Bool(!argument.truthy()?),
            (operator, _) => fold_unary(operator, argument.prim()?)?,
        };
        Some(Out::Prim(value))
    }

    fn logical(&mut self, logical: &'p LogicalExpression<'p>, depth: u32) -> Option<Out<'p>> {
        let left = self.eval(&logical.left, depth)?;
        let take_right = match logical.operator {
            LogicalOperator::And => left.truthy()?,
            LogicalOperator::Or => !left.truthy()?,
            LogicalOperator::Coalesce => {
                matches!(left.prim(), Some(Const::Null | Const::Undefined))
            }
        };
        if take_right {
            return self.eval(&logical.right, depth);
        }
        Some(left)
    }

    fn get(&mut self, object: Out<'p>, key: &str, depth: u32) -> Option<Out<'p>> {
        match object {
            Out::Array(array) => {
                if key == "length" {
                    return Some(Out::Prim(Const::Num(array.elements.len() as f64)));
                }
                let element = array.elements.get(array_index(key)?)?;
                self.eval(element.as_expression()?, depth)
            }
            Out::Object(object) => {
                let found = object.properties.iter().rev().find_map(|property| {
                    let ObjectPropertyKind::ObjectProperty(entry) = property else {
                        return None;
                    };
                    (static_key(entry)? == key).then_some(&entry.value)
                })?;
                self.eval(found, depth)
            }
            Out::Prim(Const::Str(text)) => string_member(&text, key).map(Out::Prim),
            Out::Prim(_) => None,
        }
    }
}

fn string_member(text: &str, key: &str) -> Option<Const> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if key == "length" {
        return Some(Const::Num(units.len() as f64));
    }
    let unit = *units.get(array_index(key)?)?;
    let character = char::from_u32(u32::from(unit))?;
    Some(Const::Str(character.to_string()))
}
