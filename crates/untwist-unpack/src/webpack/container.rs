use oxc_ast::ast::*;
use untwist_core::ast_util::prop::prop_name;

use crate::matcher::{anonymous_factory, Factory};

pub fn is_module_container(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            other => other
                .as_expression()
                .is_some_and(|expr| anonymous_factory(expr).is_some()),
        }),
        Expression::ObjectExpression(object) => object.properties.iter().all(is_container_property),
        _ => false,
    }
}

pub fn container_is_empty(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ArrayExpression(array) => array.elements.is_empty(),
        Expression::ObjectExpression(object) => object.properties.is_empty(),
        _ => true,
    }
}

fn is_container_property(property: &ObjectPropertyKind<'_>) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return false;
    };
    if !has_container_shape(property) {
        return false;
    }
    if property.method {
        return matches!(
            &property.value,
            Expression::FunctionExpression(function) if is_valid_method(function)
        );
    }
    if anonymous_factory(&property.value).is_some() {
        return true;
    }
    is_public_path_property(property)
}

fn has_container_shape(property: &ObjectProperty<'_>) -> bool {
    !property.computed && property.kind == PropertyKind::Init && prop_name(&property.key).is_some()
}

fn is_valid_method(function: &Function<'_>) -> bool {
    !function.generator && !function.r#async && function.body.is_some()
}

fn is_public_path_property(property: &ObjectProperty<'_>) -> bool {
    matches!(&property.key, PropertyKey::StaticIdentifier(id) if id.name == "c")
        && matches!(property.value, Expression::StringLiteral(_))
}

pub fn module_factories<'k, 'a>(container: &'k Expression<'a>) -> Vec<(String, Factory<'k, 'a>)> {
    match container {
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .enumerate()
            .filter_map(|(index, element)| {
                let expr = element.as_expression()?;
                let factory = anonymous_factory(expr)?;
                Some((index.to_string(), factory))
            })
            .collect(),
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .filter_map(object_property_factory)
            .collect(),
        _ => Vec::new(),
    }
}

fn object_property_factory<'k, 'a>(
    property: &'k ObjectPropertyKind<'a>,
) -> Option<(String, Factory<'k, 'a>)> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    if !has_container_shape(property) {
        return None;
    }
    let name = prop_name(&property.key)?.into_owned();
    let factory = if property.method {
        match &property.value {
            Expression::FunctionExpression(function) if is_valid_method(function) => Factory {
                params: &function.params,
                body: function.body.as_deref()?,
            },
            _ => return None,
        }
    } else {
        anonymous_factory(&property.value)?
    };
    Some((name, factory))
}
