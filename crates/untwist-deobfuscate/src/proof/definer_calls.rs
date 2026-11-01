use oxc_ast::ast::*;

use super::writes::{definer_args_site, is_getter_definer, is_owned_definer, PatchSite};

fn is_named(expr: &Expression<'_>, names: &[&str]) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if names.contains(&id.name.as_str()))
}

fn definer_value<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    let member = expr.without_parentheses().as_member_expression()?;
    let method = member.static_property_name()?;
    let owned = is_owned_definer(method) && is_named(member.object(), &["Object", "Reflect"]);
    (owned || is_getter_definer(method)).then_some(method)
}

fn plain<'k, 'a>(arguments: &'k [Argument<'a>]) -> Option<Vec<&'k Expression<'a>>> {
    arguments.iter().map(Argument::as_expression).collect()
}

fn listed<'k, 'a>(argument: Option<&'k Argument<'a>>) -> Option<Vec<&'k Expression<'a>>> {
    let Expression::ArrayExpression(array) = argument?.as_expression()?.without_parentheses()
    else {
        return None;
    };
    array
        .elements
        .iter()
        .map(ArrayExpressionElement::as_expression)
        .collect()
}

fn received<'k, 'a>(
    method: &str,
    receiver: Option<&'k Expression<'a>>,
    mut arguments: Vec<&'k Expression<'a>>,
) -> Option<Vec<&'k Expression<'a>>> {
    if is_getter_definer(method) {
        arguments.insert(0, receiver?);
    }
    Some(arguments)
}

pub(super) fn applied_definer_patch<'a>(
    member: &MemberExpression<'a>,
    key: &str,
    call: &CallExpression<'a>,
) -> Option<PatchSite<'a>> {
    let reflected = key == "apply" && is_named(member.object(), &["Reflect"]);
    let (method, receiver, arguments) = if reflected {
        let method = definer_value(call.arguments.first()?.as_expression()?)?;
        let receiver = call.arguments.get(1).and_then(Argument::as_expression);
        (method, receiver, listed(call.arguments.get(2))?)
    } else {
        let method = definer_value(member.object())?;
        let (receiver, rest) = call.arguments.split_first()?;
        let arguments = match key {
            "call" => plain(rest)?,
            _ => listed(rest.first())?,
        };
        (method, receiver.as_expression(), arguments)
    };
    let arguments = received(method, receiver, arguments)?;
    definer_args_site(method, &arguments, call.span)
}

pub(super) fn bound_definer_patch<'a>(
    callee: &Expression<'a>,
    call: &CallExpression<'a>,
) -> Option<PatchSite<'a>> {
    let Expression::CallExpression(bind) = callee else {
        return None;
    };
    let member = bind.callee.without_parentheses().as_member_expression()?;
    if member.static_property_name()? != "bind" {
        return None;
    }
    let method = definer_value(member.object())?;
    let (receiver, bound) = bind.arguments.split_first()?;
    let mut arguments = plain(bound)?;
    arguments.extend(plain(&call.arguments)?);
    let arguments = received(method, receiver.as_expression(), arguments)?;
    definer_args_site(method, &arguments, call.span)
}
