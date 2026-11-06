use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;
use rustc_hash::FxHashSet;

use super::storage::Storage;

#[derive(Clone, Debug)]
pub enum WrittenKey {
    Name(String),
    Pending(Span),
    Concat(Vec<WrittenKey>),
    Choice {
        key: Span,
        options: Vec<(String, WrittenKey)>,
    },
}

impl WrittenKey {
    pub fn resolved(&self, resolve: &dyn Fn(Span) -> Option<String>) -> Option<String> {
        match self {
            WrittenKey::Name(name) => Some(name.clone()),
            WrittenKey::Pending(span) => resolve(*span),
            WrittenKey::Concat(parts) => parts
                .iter()
                .map(|part| part.resolved(resolve))
                .collect::<Option<Vec<String>>>()
                .map(|parts| parts.concat()),
            WrittenKey::Choice { key, options } => {
                let key = resolve(*key)?;
                options
                    .iter()
                    .find(|(name, _)| *name == key)
                    .and_then(|(_, chosen)| chosen.resolved(resolve))
            }
        }
    }

    pub fn pending_spans(&self, spans: &mut FxHashSet<Span>) {
        match self {
            WrittenKey::Name(_) => {}
            WrittenKey::Pending(span) => {
                spans.insert(*span);
            }
            WrittenKey::Concat(parts) => {
                for part in parts {
                    part.pending_spans(spans);
                }
            }
            WrittenKey::Choice { key, options } => {
                spans.insert(*key);
                for (_, option) in options {
                    option.pending_spans(spans);
                }
            }
        }
    }

    pub(super) fn is_pending(&self) -> bool {
        match self {
            WrittenKey::Name(_) => false,
            WrittenKey::Pending(_) => true,
            WrittenKey::Concat(parts) => parts.iter().any(WrittenKey::is_pending),
            WrittenKey::Choice { .. } => true,
        }
    }
}

pub(super) fn text_template(
    expr: &Expression<'_>,
    storage: Option<&Storage<'_>>,
) -> Option<WrittenKey> {
    match expr.without_parentheses() {
        Expression::StringLiteral(text) => named(text.value.as_str()),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            Some(WrittenKey::Concat(vec![
                text_template(&binary.left, storage)?,
                text_template(&binary.right, storage)?,
            ]))
        }
        Expression::TemplateLiteral(template) => {
            let mut parts = Vec::with_capacity(template.quasis.len() + template.expressions.len());
            for (index, quasi) in template.quasis.iter().enumerate() {
                parts.push(WrittenKey::Name(quasi.value.cooked.as_ref()?.to_string()));
                if let Some(expression) = template.expressions.get(index) {
                    parts.push(text_template(expression, storage)?);
                }
            }
            Some(WrittenKey::Concat(parts))
        }
        Expression::CallExpression(call) => Some(
            storage
                .and_then(|storage| storage.call_template(call))
                .unwrap_or(WrittenKey::Pending(call.span)),
        ),
        Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
            storage.and_then(|storage| storage.read_template(expr))
        }
        _ => None,
    }
}

const KEYED_AT_ONE: &[&str] = &["defineProperty", "set", "deleteProperty"];
const KEYED_AT_ZERO: &[&str] = &["__defineGetter__", "__defineSetter__"];
const LITERAL_KEYS: &[&str] = &["assign", "defineProperties"];

fn named(name: &str) -> Option<WrittenKey> {
    Some(WrittenKey::Name(name.to_string()))
}

fn concatenated(key: &Expression<'_>, parts: &mut Vec<WrittenKey>) -> bool {
    match key.without_parentheses() {
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            concatenated(&binary.left, parts) && concatenated(&binary.right, parts)
        }
        other => match written_key(other) {
            Some(part) => {
                parts.push(part);
                true
            }
            None => false,
        },
    }
}

pub(super) fn written_key(key: &Expression<'_>) -> Option<WrittenKey> {
    match key.without_parentheses() {
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            let mut parts = Vec::new();
            concatenated(key, &mut parts).then_some(WrittenKey::Concat(parts))
        }
        Expression::StringLiteral(name) => named(name.value.as_str()),
        Expression::TemplateLiteral(template) => template_key(template),
        Expression::ArrayExpression(array) => joined_key(&array.elements, ","),
        Expression::CallExpression(call) => {
            Some(spelled_call_key(call).unwrap_or(WrittenKey::Pending(call.span)))
        }
        _ => None,
    }
}

fn template_key(template: &TemplateLiteral<'_>) -> Option<WrittenKey> {
    let mut parts = Vec::with_capacity(template.quasis.len() + template.expressions.len());
    for (index, quasi) in template.quasis.iter().enumerate() {
        parts.push(WrittenKey::Name(quasi.value.cooked.as_ref()?.to_string()));
        if let Some(expression) = template.expressions.get(index) {
            parts.push(written_key(expression)?);
        }
    }
    Some(WrittenKey::Concat(parts))
}

fn joined_key(elements: &[ArrayExpressionElement<'_>], separator: &str) -> Option<WrittenKey> {
    let mut parts = Vec::with_capacity(elements.len() * 2);
    for (index, element) in elements.iter().enumerate() {
        if index > 0 {
            parts.push(WrittenKey::Name(separator.to_string()));
        }
        parts.push(written_key(element.as_expression()?)?);
    }
    Some(WrittenKey::Concat(parts))
}

fn spelled_call_key(call: &CallExpression<'_>) -> Option<WrittenKey> {
    let member = call.callee.without_parentheses().as_member_expression()?;
    let receiver = member.object().without_parentheses();
    match member.static_property_name()? {
        "concat" if !matches!(receiver, Expression::ArrayExpression(_)) => {
            let mut parts = vec![written_key(receiver)?];
            for argument in &call.arguments {
                parts.push(written_key(argument.as_expression()?)?);
            }
            Some(WrittenKey::Concat(parts))
        }
        "join" => {
            let Expression::ArrayExpression(array) = receiver else {
                return None;
            };
            let separator = match call.arguments.first().map(Argument::as_expression) {
                None => ",",
                Some(Some(Expression::StringLiteral(text))) => text.value.as_str(),
                Some(_) => return None,
            };
            joined_key(&array.elements, separator)
        }
        _ => None,
    }
}

pub(super) fn member_written_key(member: &MemberExpression<'_>) -> Option<WrittenKey> {
    match member {
        MemberExpression::StaticMemberExpression(member) => named(member.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(member) => written_key(&member.expression),
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn literal_keys(object: &ObjectExpression<'_>, out: &mut Vec<WrittenKey>) {
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let key = match &property.key {
            PropertyKey::StaticIdentifier(name) => named(name.name.as_str()),
            PropertyKey::StringLiteral(name) => named(name.value.as_str()),
            other => other.as_expression().and_then(written_key),
        };
        out.extend(key);
    }
}

pub(super) fn definer_written_keys(call: &CallExpression<'_>, out: &mut Vec<WrittenKey>) {
    let Some(member) = call.callee.without_parentheses().as_member_expression() else {
        return;
    };
    let Some(method) = super::writes::member_method_name(member) else {
        return;
    };
    let method = method.as_str();
    let at = |index: usize| call.arguments.get(index).and_then(Argument::as_expression);
    if KEYED_AT_ONE.contains(&method) {
        out.extend(at(1).and_then(written_key));
    } else if KEYED_AT_ZERO.contains(&method) {
        out.extend(at(0).and_then(written_key));
    } else if LITERAL_KEYS.contains(&method) {
        for argument in call.arguments.iter().skip(1) {
            if let Some(Expression::ObjectExpression(object)) = argument
                .as_expression()
                .map(Expression::without_parentheses)
            {
                literal_keys(object, out);
            }
        }
    }
}
