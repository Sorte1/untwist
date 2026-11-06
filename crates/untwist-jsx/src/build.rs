use oxc_allocator::{Box as ABox, CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::identifier::{is_identifier_part, is_identifier_start};
use untwist_core::ast_util::prop::identifier_name;

pub(crate) const FRAGMENT: &str = "Fragment";
pub(crate) const CHILDREN: &str = "children";
const KEY: &str = "key";
const NAME_DASH: char = '-';
const CONTAINER_CHARS: [char; 5] = ['"', '\\', '&', '\r', '\n'];
const TEXT_BREAKERS: [char; 7] = ['{', '}', '<', '>', '&', '\r', '\n'];
const METHOD: &str = "a method";
const GETTER: &str = "a getter";
const SETTER: &str = "a setter";
const LONE_SURROGATE_KEY: &str = "a lone-surrogate string key";
const UNSPELLABLE_KEY: &str = "a key JSX cannot spell";
const EMPTY_TAG: &str = "an empty tag name";

pub fn lowercase_identifier<'e>(expr: &'e Expression<'_>) -> Option<&'e IdentifierReference<'e>> {
    let Expression::Identifier(id) = expr else {
        return None;
    };
    id.name
        .as_str()
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        .then_some(&**id)
}

pub fn is_usable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_identifier_start(first) && chars.all(|ch| is_identifier_part(ch) || ch == NAME_DASH)
}

pub enum NamePlan {
    Tag(String),
    Member(Vec<String>),
}

fn member_segments(expr: &Expression<'_>, into: &mut Vec<String>) -> bool {
    if let Some(name) = identifier_name(expr) {
        into.push(name.to_string());
        return true;
    }
    let Expression::StaticMemberExpression(member) = expr else {
        return false;
    };
    if member.optional {
        return false;
    }
    if !member_segments(&member.object, into) {
        return false;
    }
    into.push(member.property.name.to_string());
    true
}

pub fn name_plan(expr: &Expression<'_>) -> Option<NamePlan> {
    if let Expression::StringLiteral(literal) = expr {
        return literal
            .value
            .as_str()
            .map(|name| NamePlan::Tag(name.to_string()));
    }
    if let Some(name) = identifier_name(expr) {
        return Some(NamePlan::Tag(name.to_string()));
    }
    let mut segments = Vec::new();
    (member_segments(expr, &mut segments) && segments.len() > 1)
        .then_some(NamePlan::Member(segments))
}

fn arena_name<'a>(name: &str, b: &impl GetAstBuilder<'a>) -> Ident<'a> {
    Ident::from_str_in(name, b.builder())
}

fn clone_name<'a>(name: &JSXElementName<'a>, b: &impl GetAstBuilder<'a>) -> JSXElementName<'a> {
    name.clone_in(b.builder().allocator())
}

fn member_name<'a>(segments: &[String], b: &impl GetAstBuilder<'a>) -> JSXElementName<'a> {
    let (last, prefix) = match segments.split_last() {
        Some(split) => split,
        None => return JSXElementName::new_identifier(Span::default(), arena_name("", b), b),
    };
    let root = prefix.first().map_or("", String::as_str);
    let mut object = JSXMemberExpressionObject::new_identifier_reference(
        Span::default(),
        arena_name(root, b),
        b,
    );
    for segment in prefix.iter().skip(1) {
        let property = JSXIdentifier::new(Span::default(), arena_name(segment, b), b);
        object =
            JSXMemberExpressionObject::new_member_expression(Span::default(), object, property, b);
    }
    let property = JSXIdentifier::new(Span::default(), arena_name(last, b), b);
    JSXElementName::new_member_expression(Span::default(), object, property, b)
}

pub fn unspellable_tag(plan: &NamePlan) -> Option<&str> {
    let NamePlan::Tag(name) = plan else {
        return None;
    };
    if is_usable_name(name) {
        return None;
    }
    match name.is_empty() {
        true => Some(EMPTY_TAG),
        false => Some(name),
    }
}

pub fn component_looking_string<'e>(
    expr: &'e Expression<'_>,
    conservative: bool,
) -> Option<&'e str> {
    if conservative {
        return None;
    }
    let Expression::StringLiteral(literal) = expr else {
        return None;
    };
    let value = literal.value.as_str()?;
    let intrinsic = value.starts_with(|ch: char| ch.is_ascii_lowercase());
    (!intrinsic).then_some(value)
}

pub fn element_name<'a>(plan: &NamePlan, b: &impl GetAstBuilder<'a>) -> JSXElementName<'a> {
    match plan {
        NamePlan::Tag(name) => {
            if name.starts_with(|ch: char| ch.is_ascii_lowercase()) {
                JSXElementName::new_identifier(Span::default(), arena_name(name, b), b)
            } else {
                JSXElementName::new_identifier_reference(Span::default(), arena_name(name, b), b)
            }
        }
        NamePlan::Member(segments) => member_name(segments, b),
    }
}

fn key_name<'a>(key: &PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

pub fn prop_problem(property: &ObjectProperty<'_>) -> Option<&'static str> {
    if property.method {
        return Some(METHOD);
    }
    match property.kind {
        PropertyKind::Get => return Some(GETTER),
        PropertyKind::Set => return Some(SETTER),
        PropertyKind::Init => {}
    }
    match key_name(&property.key) {
        Some(name) if is_usable_name(name) => None,
        Some(_) => Some(UNSPELLABLE_KEY),
        None => match &property.key {
            PropertyKey::StringLiteral(_) => Some(LONE_SURROGATE_KEY),
            _ => Some(UNSPELLABLE_KEY),
        },
    }
}

pub fn inconvertible_prop(props: Option<&Expression<'_>>) -> Option<&'static str> {
    let Some(Expression::ObjectExpression(object)) = props else {
        return None;
    };
    object
        .properties
        .iter()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => prop_problem(property),
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
}

fn attribute_name<'a>(property: &ObjectProperty<'a>) -> Option<&'a str> {
    if prop_problem(property).is_some() {
        return None;
    }
    key_name(&property.key)
}

pub fn attribute_value<'a>(
    expr: Expression<'a>,
    b: &impl GetAstBuilder<'a>,
) -> JSXAttributeValue<'a> {
    if let Expression::StringLiteral(literal) = &expr {
        if let Some(text) = literal
            .value
            .as_str()
            .filter(|text| !text.contains(CONTAINER_CHARS))
        {
            return JSXAttributeValue::new_string_literal(Span::default(), text, None, b);
        }
    }
    JSXAttributeValue::new_expression_container(Span::default(), JSXExpression::from(expr), b)
}

pub fn key_attribute<'a>(
    argument: Argument<'a>,
    b: &impl GetAstBuilder<'a>,
) -> Option<JSXAttributeItem<'a>> {
    match argument {
        Argument::SpreadElement(_) => None,
        other => {
            let name = JSXAttributeName::new_identifier(Span::default(), arena_name(KEY, b), b);
            let value = attribute_value(other.into_expression(), b);
            Some(JSXAttributeItem::new_attribute(
                Span::default(),
                name,
                Some(value),
                b,
            ))
        }
    }
}

fn own_spread_attribute<'a>(
    property: ObjectPropertyKind<'a>,
    b: &impl GetAstBuilder<'a>,
) -> JSXAttributeItem<'a> {
    let properties = AVec::from_iter_in([property], b.builder());
    let object = Expression::new_object_expression(Span::default(), properties, b);
    JSXAttributeItem::new_spread_attribute(Span::default(), object, b)
}

fn attribute<'a>(
    property: ObjectPropertyKind<'a>,
    skip_children: bool,
    b: &impl GetAstBuilder<'a>,
) -> Option<JSXAttributeItem<'a>> {
    let property = match property {
        ObjectPropertyKind::SpreadProperty(spread) => {
            let spread = spread.unbox();
            return Some(JSXAttributeItem::new_spread_attribute(
                Span::default(),
                spread.argument,
                b,
            ));
        }
        ObjectPropertyKind::ObjectProperty(property) => property,
    };
    let Some(name) = attribute_name(&property) else {
        return Some(own_spread_attribute(
            ObjectPropertyKind::ObjectProperty(property),
            b,
        ));
    };
    if skip_children && name == CHILDREN {
        return None;
    }
    let name = JSXAttributeName::new_identifier(Span::default(), arena_name(name, b), b);
    let value = attribute_value(property.unbox().value, b);
    Some(JSXAttributeItem::new_attribute(
        Span::default(),
        name,
        Some(value),
        b,
    ))
}

pub fn attributes<'a>(
    object: ABox<'a, ObjectExpression<'a>>,
    skip_children: bool,
    b: &impl GetAstBuilder<'a>,
) -> AVec<'a, JSXAttributeItem<'a>> {
    let object = object.unbox();
    let mut items = AVec::with_capacity_in(object.properties.len(), b.builder());
    for property in object.properties {
        if let Some(item) = attribute(property, skip_children, b) {
            items.push(item);
        }
    }
    items
}

pub fn child<'a>(
    argument: Argument<'a>,
    conservative: bool,
    b: &impl GetAstBuilder<'a>,
) -> JSXChild<'a> {
    match argument {
        Argument::SpreadElement(spread) => {
            let spread = spread.unbox();
            JSXChild::new_spread(Span::default(), spread.argument, b)
        }
        other => child_of(other.into_expression(), conservative, b),
    }
}

pub fn child_of<'a>(
    expr: Expression<'a>,
    conservative: bool,
    b: &impl GetAstBuilder<'a>,
) -> JSXChild<'a> {
    match &expr {
        Expression::JSXElement(_) => {}
        Expression::JSXFragment(_) if !conservative => {}
        Expression::StringLiteral(literal) => {
            let text = literal
                .value
                .as_str()
                .filter(|text| !text.contains(TEXT_BREAKERS));
            if let Some(text) = text {
                return JSXChild::new_text(Span::default(), text, None, b);
            }
        }
        _ => {
            return JSXChild::new_expression_container(
                Span::default(),
                JSXExpression::from(expr),
                b,
            );
        }
    }
    match expr {
        Expression::JSXElement(element) => JSXChild::Element(element),
        Expression::JSXFragment(fragment) => JSXChild::Fragment(fragment),
        other => JSXChild::new_expression_container(Span::default(), JSXExpression::from(other), b),
    }
}

pub fn element<'a>(
    name: JSXElementName<'a>,
    attributes: AVec<'a, JSXAttributeItem<'a>>,
    children: AVec<'a, JSXChild<'a>>,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Expression<'a> {
    let self_closing = children.is_empty();
    let closing =
        (!self_closing).then(|| JSXClosingElement::boxed(Span::default(), clone_name(&name, b), b));
    let opening = JSXOpeningElement::boxed(Span::default(), name, None, attributes, b);
    Expression::new_jsx_element(span, opening, children, closing, b)
}

pub fn fragment<'a>(
    children: AVec<'a, JSXChild<'a>>,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Expression<'a> {
    let opening = JSXOpeningFragment::new(Span::default(), b);
    let closing = JSXClosingFragment::new(Span::default(), b);
    Expression::new_jsx_fragment(span, opening, children, closing, b)
}
