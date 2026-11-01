use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_span::SPAN;

fn is_shorthand(key: &PropertyKey<'_>, value: &BindingPattern<'_>) -> bool {
    match (key, value) {
        (PropertyKey::StaticIdentifier(key), BindingPattern::BindingIdentifier(id)) => {
            key.name == id.name
        }
        _ => false,
    }
}

fn is_static_key(key: &PropertyKey<'_>) -> bool {
    matches!(
        key,
        PropertyKey::StaticIdentifier(_)
            | PropertyKey::StringLiteral(_)
            | PropertyKey::NumericLiteral(_)
    )
}

pub fn object_pattern<'a>(
    members: impl IntoIterator<Item = (PropertyKey<'a>, BindingPattern<'a>)>,
    builder: &impl GetAstBuilder<'a>,
) -> BindingPattern<'a> {
    let members = members.into_iter();
    let mut properties = AVec::with_capacity_in(members.size_hint().0, builder.builder());
    for (key, value) in members {
        let shorthand = is_shorthand(&key, &value);
        let computed = !is_static_key(&key);
        properties.push(BindingProperty::new(
            SPAN, key, value, shorthand, computed, builder,
        ));
    }
    BindingPattern::new_object_pattern(SPAN, properties, None, builder)
}

pub fn array_pattern<'a>(
    slots: impl IntoIterator<Item = Option<BindingPattern<'a>>>,
    builder: &impl GetAstBuilder<'a>,
) -> BindingPattern<'a> {
    let slots = slots.into_iter();
    let mut elements = AVec::with_capacity_in(slots.size_hint().0, builder.builder());
    elements.extend(slots);
    BindingPattern::new_array_pattern(SPAN, elements, None, builder)
}
