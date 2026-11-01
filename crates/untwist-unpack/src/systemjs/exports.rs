use oxc_ast::ast::*;

use super::layout::property_name;

pub const ES_MODULE: &str = "__esModule";

pub enum Value<'k, 'a> {
    Write(&'k IdentifierReference<'a>),
    Read(&'k IdentifierReference<'a>),
    Fresh,
    Invalid,
}

pub fn value<'k, 'a>(expression: &'k Expression<'a>) -> Value<'k, 'a> {
    match expression {
        Expression::AssignmentExpression(assign) => match &assign.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => Value::Write(id),
            _ => Value::Fresh,
        },
        Expression::UpdateExpression(update) => match (&update.argument, update.prefix) {
            (SimpleAssignmentTarget::AssignmentTargetIdentifier(id), true) => Value::Write(id),
            (SimpleAssignmentTarget::AssignmentTargetIdentifier(_), false) => Value::Invalid,
            _ => Value::Fresh,
        },
        Expression::Identifier(id) => Value::Read(id),
        _ => Value::Fresh,
    }
}

pub fn is_export_name(name: &str) -> bool {
    let mut chars = name.chars();
    let head = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$');
    head && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') && name != ES_MODULE
}

pub enum Call<'k, 'a> {
    Single(&'a str, &'k Expression<'a>),
    Object(Vec<(&'a str, &'k Expression<'a>)>),
}

pub fn is_export_call(call: &CallExpression<'_>, export: Option<&str>) -> bool {
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    export == Some(callee.name.as_str())
}

pub fn export_call<'k, 'a>(call: &'k CallExpression<'a>) -> Option<Call<'k, 'a>> {
    if call.optional {
        return None;
    }
    match call.arguments.as_slice() {
        [Argument::StringLiteral(name), value] if !name.lone_surrogates => {
            let name = name.value.as_str();
            let value = value.as_expression()?;
            is_export_name(name).then_some(Call::Single(name, value))
        }
        [Argument::ObjectExpression(object)] => object
            .properties
            .iter()
            .map(|property| {
                let (name, value) = property_name(property)?;
                is_export_name(name).then_some((name, value))
            })
            .collect::<Option<Vec<_>>>()
            .map(Call::Object),
        _ => None,
    }
}

pub fn declarator_export<'k, 'a>(
    declarator: &'k VariableDeclarator<'a>,
    export: Option<&str>,
) -> Option<(&'k BindingIdentifier<'a>, &'a str)> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let Some(Expression::CallExpression(call)) = &declarator.init else {
        return None;
    };
    if !is_export_call(call, export) {
        return None;
    }
    match export_call(call)? {
        Call::Single(name, value) if matches!(self::value(value), Value::Fresh) => Some((id, name)),
        _ => None,
    }
}

pub fn context_import<'a>(
    call: &CallExpression<'a>,
    context: Option<&str>,
) -> Option<Option<&'a str>> {
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    let Expression::Identifier(object) = &member.object else {
        return None;
    };
    if context != Some(object.name.as_str()) {
        return None;
    }
    let specifier = match (call.arguments.as_slice(), call.optional || member.optional) {
        ([Argument::StringLiteral(literal)], false)
            if member.property.name == "import" && !literal.lone_surrogates =>
        {
            Some(literal.value.as_str())
        }
        _ => None,
    };
    Some(specifier)
}
