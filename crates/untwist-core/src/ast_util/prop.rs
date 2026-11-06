use std::borrow::Cow;

use oxc_ast::ast::*;

pub fn number_key(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e21 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

pub fn prop_name<'k>(key: &'k PropertyKey<'_>) -> Option<Cow<'k, str>> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(Cow::Borrowed(id.name.as_str())),
        PropertyKey::StringLiteral(literal) => literal.value.as_str().map(Cow::Borrowed),
        PropertyKey::NumericLiteral(literal) => Some(Cow::Owned(number_key(literal.value))),
        _ => None,
    }
}

pub fn identifier_name<'k>(expr: &'k Expression<'_>) -> Option<&'k str> {
    match expr {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

#[inline(always)]
pub fn member_parts<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, Cow<'k, str>)> {
    match expr {
        Expression::StaticMemberExpression(member) if !member.optional => {
            Some((&member.object, Cow::Borrowed(member.property.name.as_str())))
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            Some((&member.object, literal_key(&member.expression)?))
        }
        _ => None,
    }
}

#[inline(always)]
pub fn literal_key<'k>(expr: &'k Expression<'_>) -> Option<Cow<'k, str>> {
    match expr {
        Expression::StringLiteral(literal) => literal.value.as_str().map(Cow::Borrowed),
        Expression::NumericLiteral(literal) => Some(Cow::Owned(number_key(literal.value))),
        _ => None,
    }
}

pub fn literal_member_parts<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, Cow<'k, str>)> {
    let object = match expr {
        Expression::StaticMemberExpression(member) => &member.object,
        Expression::ComputedMemberExpression(member) => &member.object,
        _ => return None,
    };
    if !matches!(object, Expression::ObjectExpression(_)) {
        return None;
    }
    member_parts(expr)
}

pub fn computed_index<'k, 'a>(expr: &'k Expression<'a>) -> Option<(&'k Expression<'a>, f64)> {
    let Expression::ComputedMemberExpression(member) = expr else {
        return None;
    };
    if member.optional {
        return None;
    }
    let Expression::NumericLiteral(literal) = &member.expression else {
        return None;
    };
    Some((&member.object, literal.value))
}
