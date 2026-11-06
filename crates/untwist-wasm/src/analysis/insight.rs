use oxc_ast::ast::{
    Class, ClassElement, Expression, FormalParameters, MethodDefinitionKind, ObjectExpression,
    ObjectPropertyKind, UnaryOperator,
};
use oxc_span::{GetSpan, Span};

use super::index::{Detail, Form, Names};

const VALUE_CHARS: usize = 200;
const PARAM_CHARS: usize = 40;
const PARAM_LIMIT: usize = 16;

fn source(text: &str, span: Span) -> &str {
    text.get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}

fn cut(text: &str, limit: usize) -> (String, bool) {
    match text.char_indices().nth(limit) {
        Some((byte, _)) => (text.get(..byte).unwrap_or_default().to_string(), true),
        None => (text.to_string(), false),
    }
}

fn quoted(value: &str) -> Box<str> {
    let (shown, cut_short) = cut(value, VALUE_CHARS);
    let mut json = serde_json::to_string(&shown).unwrap_or_default();
    if cut_short {
        json.insert(json.len().saturating_sub(1), '…');
    }
    json.into()
}

fn spaced(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short(text: &str, limit: usize) -> Box<str> {
    let (shown, cut_short) = cut(&spaced(text), limit);
    if cut_short {
        format!("{shown}…").into()
    } else {
        shown.into()
    }
}

pub fn params_of(params: &FormalParameters<'_>, text: &str) -> Box<[Box<str>]> {
    params
        .items
        .iter()
        .map(GetSpan::span)
        .chain(params.rest.iter().map(|rest| rest.span))
        .take(PARAM_LIMIT)
        .map(|span| short(source(text, span), PARAM_CHARS))
        .collect()
}

pub fn constructor_params(class: &Class<'_>, text: &str) -> Box<[Box<str>]> {
    class
        .body
        .body
        .iter()
        .find_map(|element| match element {
            ClassElement::MethodDefinition(method)
                if method.kind == MethodDefinitionKind::Constructor =>
            {
                Some(params_of(&method.value.params, text))
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn literal_form(init: &Expression<'_>, names: &mut Names) -> Form {
    match init.without_parentheses() {
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => Form::String,
        Expression::NumericLiteral(_) | Expression::BigIntLiteral(_) => Form::Number,
        Expression::BooleanLiteral(_) => Form::Boolean,
        Expression::RegExpLiteral(_) => Form::Regexp,
        Expression::ArrayExpression(_) => Form::Array,
        Expression::ObjectExpression(_) => Form::Object,
        Expression::UnaryExpression(unary) => match (unary.operator, &unary.argument) {
            (UnaryOperator::LogicalNot, Expression::NumericLiteral(_)) => Form::Boolean,
            (UnaryOperator::UnaryNegation, Expression::NumericLiteral(_)) => Form::Number,
            _ => Form::Unknown,
        },
        Expression::NewExpression(new) => match &new.callee {
            Expression::Identifier(callee) => Form::Instance(names.intern(&callee.name)),
            _ => Form::Unknown,
        },
        _ => Form::Unknown,
    }
}

fn string_value(value: &str, span: Span, text: &str) -> (Box<str>, bool) {
    let raw = source(text, span);
    let inner = raw.get(1..raw.len().saturating_sub(1)).unwrap_or_default();
    (quoted(value), inner != value)
}

fn literal_value(init: &Expression<'_>, text: &str) -> Option<(Box<str>, bool)> {
    let shown = |span: Span| (short(source(text, span), VALUE_CHARS), false);
    match init.without_parentheses() {
        Expression::StringLiteral(string) if !string.value.has_lone_surrogate() => {
            Some(string_value(string.value.as_str()?, string.span, text))
        }
        Expression::TemplateLiteral(template) => template
            .single_quasi()
            .and_then(|cooked| cooked.as_str())
            .map(|cooked| string_value(cooked, template.span, text)),
        Expression::NumericLiteral(number) => Some(shown(number.span)),
        Expression::BigIntLiteral(big) => Some(shown(big.span)),
        Expression::RegExpLiteral(regexp) => Some(shown(regexp.span)),
        Expression::BooleanLiteral(boolean) => Some((boolean.value.to_string().into(), false)),
        Expression::NullLiteral(_) => Some(("null".into(), false)),
        Expression::Identifier(id) if id.name == "undefined" => Some(("undefined".into(), false)),
        Expression::UnaryExpression(unary) => match (unary.operator, &unary.argument) {
            (UnaryOperator::LogicalNot, Expression::NumericLiteral(number)) => {
                Some(((number.value == 0.0).to_string().into(), false))
            }
            (UnaryOperator::UnaryNegation, Expression::NumericLiteral(_)) => {
                Some(shown(unary.span))
            }
            (UnaryOperator::Void, Expression::NumericLiteral(_)) => {
                Some(("undefined".into(), false))
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn constant_detail(init: &Expression<'_>, text: &str, names: &mut Names) -> Option<Detail> {
    let form = literal_form(init, names);
    let value = literal_value(init, text);
    if form == Form::Unknown && value.is_none() {
        return None;
    }
    let (value, decoded) = value.map_or((None, false), |(value, decoded)| (Some(value), decoded));
    Some(Detail {
        form,
        value,
        decoded,
        params: None,
    })
}

pub fn function_init<'b, 'a>(init: &'b Expression<'a>) -> Option<&'b FormalParameters<'a>> {
    match init.without_parentheses() {
        Expression::FunctionExpression(function) => Some(&function.params),
        Expression::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        _ => None,
    }
}

pub fn class_init<'b, 'a>(init: &'b Expression<'a>) -> Option<&'b Class<'a>> {
    match init.without_parentheses() {
        Expression::ClassExpression(class) => Some(class),
        _ => None,
    }
}

pub fn object_keys<'b>(object: &'b ObjectExpression<'_>) -> impl Iterator<Item = String> + 'b {
    object
        .properties
        .iter()
        .filter_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => property
                .key
                .static_name()
                .and_then(|name| name.into_cow_str())
                .map(|name| name.to_string()),
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
}

pub fn object_init<'b, 'a>(init: &'b Expression<'a>) -> Option<&'b ObjectExpression<'a>> {
    match init.without_parentheses() {
        Expression::ObjectExpression(object) => Some(object),
        _ => None,
    }
}
