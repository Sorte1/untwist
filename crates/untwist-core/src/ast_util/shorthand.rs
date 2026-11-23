use oxc_allocator::{Allocator, Box as ArenaBox, TakeIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_str::Ident;

const PROTO_KEY: &str = "__proto__";

pub fn shorthand_target_name<'a>(it: &AssignmentTargetProperty<'a>) -> Option<Ident<'a>> {
    match it {
        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(shorthand) => {
            Some(shorthand.binding.name)
        }
        AssignmentTargetProperty::AssignmentTargetPropertyProperty(_) => None,
    }
}

pub fn walk_target_keeping_key<'a, V: VisitMut<'a>>(
    visitor: &mut V,
    it: &mut AssignmentTargetProperty<'a>,
    alloc: &'a Allocator,
) {
    let key = shorthand_target_name(it);
    walk_mut::walk_assignment_target_property(visitor, it);
    if let Some(key) = key {
        spell_out_renamed_target(it, key, alloc);
    }
}

pub fn walk_object_property_keeping_key<'a, V: VisitMut<'a>>(
    visitor: &mut V,
    it: &mut ObjectProperty<'a>,
    alloc: &'a Allocator,
) {
    walk_mut::walk_object_property(visitor, it);
    keep_object_shorthand_key(it, alloc);
}

pub fn walk_binding_property_keeping_key<'a, V: VisitMut<'a>>(
    visitor: &mut V,
    it: &mut BindingProperty<'a>,
) {
    walk_mut::walk_binding_property(visitor, it);
    keep_binding_shorthand_key(it);
}

pub fn spell_out_renamed_target<'a>(
    it: &mut AssignmentTargetProperty<'a>,
    key: Ident<'a>,
    alloc: &'a Allocator,
) {
    let renamed = matches!(
        it,
        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(shorthand)
            if shorthand.binding.name != key
    );
    if !renamed {
        return;
    }
    let builder = AstBuilder::new(alloc);
    let AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(taken) = it.take_in(&builder)
    else {
        return;
    };
    let taken = taken.unbox();
    let span = taken.span;
    let name = PropertyKey::StaticIdentifier(ArenaBox::new_in(
        IdentifierName::new(taken.binding.span, key, &builder),
        &builder,
    ));
    let target =
        AssignmentTarget::AssignmentTargetIdentifier(ArenaBox::new_in(taken.binding, &builder));
    let binding = match taken.init {
        Some(init) => AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(ArenaBox::new_in(
            AssignmentTargetWithDefault::new(span, target, init, &builder),
            &builder,
        )),
        None => AssignmentTargetMaybeDefault::from(target),
    };
    *it = AssignmentTargetProperty::new_assignment_target_property_property(
        span, name, binding, false, &builder,
    );
}

pub fn keep_object_shorthand_key<'a>(it: &mut ObjectProperty<'a>, alloc: &'a Allocator) {
    if !it.shorthand {
        return;
    }
    let (PropertyKey::StaticIdentifier(key), Expression::Identifier(value)) = (&it.key, &it.value)
    else {
        return;
    };
    if key.name == value.name {
        return;
    }
    it.shorthand = false;
    if key.name == PROTO_KEY {
        let builder = AstBuilder::new(alloc);
        it.key = PropertyKey::new_string_literal(key.span, PROTO_KEY, None, &builder);
        it.computed = true;
    }
}

pub fn keep_binding_shorthand_key(it: &mut BindingProperty<'_>) {
    if !it.shorthand {
        return;
    }
    let PropertyKey::StaticIdentifier(key) = &it.key else {
        return;
    };
    let value = match &it.value {
        BindingPattern::BindingIdentifier(id) => Some(id.name),
        BindingPattern::AssignmentPattern(pattern) => match &pattern.left {
            BindingPattern::BindingIdentifier(id) => Some(id.name),
            _ => None,
        },
        _ => None,
    };
    if value.is_some_and(|value| value != key.name) {
        it.shorthand = false;
    }
}
