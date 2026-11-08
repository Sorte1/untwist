use oxc_ast::ast::*;
use oxc_syntax::operator::UnaryOperator;

use super::{
    closes, has_no_params, is_named, is_null, param_name, returned, single_var, statements_of,
};
use crate::ast_util::literal::{is_false_literal, is_true_literal};

const CALL_ARGUMENTS: &str = "arguments";
const PROXY_PARAMS: usize = 2;

pub(super) fn controller_target<'k, 'a>(
    statements: &'k [Statement<'a>],
) -> Option<&'k Function<'a>> {
    let [leading @ .., declaration, handoff] = statements else {
        return None;
    };
    let (flag, Some(init)) = single_var(declaration)? else {
        return None;
    };
    if !is_true_literal(init) {
        return None;
    }
    let Some(Expression::FunctionExpression(guarded)) = returned(handoff) else {
        return None;
    };
    if !leading.iter().all(is_constants_declaration) {
        return None;
    }
    guarded_target(guarded, Some(flag))
}

pub(super) fn handed_out_target<'k, 'a>(guarded: &'k Function<'a>) -> Option<&'k Function<'a>> {
    guarded_target(guarded, None)
}

fn guarded_target<'k, 'a>(
    guarded: &'k Function<'a>,
    flag: Option<&str>,
) -> Option<&'k Function<'a>> {
    if guarded.params.items.len() != 2 || guarded.params.rest.is_some() {
        return None;
    }
    let (caller, callee) = (param_name(guarded, 0)?, param_name(guarded, 1)?);
    let statements = statements_of(guarded)?;
    let tables = statements
        .iter()
        .take_while(|statement| is_proxy_table(statement))
        .count();
    let (first, rest) = statements.get(tables..)?.split_first()?;
    let flag = match flag {
        Some(flag) => flag,
        None => choice_flag(first, [caller, callee])?,
    };
    let (result, target) = guarded_choice(first, flag)?;
    let closed = closes(rest, flag, is_false_flag, result);
    (closed && runs_once(target, caller, callee)).then_some(target)
}

fn is_false_flag(expr: &Expression<'_>) -> bool {
    if is_false_literal(expr) {
        return true;
    }
    matches!(expr.without_parentheses(), Expression::UnaryExpression(not)
        if not.operator == UnaryOperator::LogicalNot
            && matches!(&not.argument, Expression::NumericLiteral(n) if n.value != 0.0 && !n.value.is_nan()))
}

fn choice_flag<'k>(statement: &'k Statement<'_>, params: [&str; 2]) -> Option<&'k str> {
    let (_, Some(Expression::ConditionalExpression(choice))) = single_var(statement)? else {
        return None;
    };
    match &choice.test {
        Expression::Identifier(flag) if !params.contains(&flag.name.as_str()) => {
            Some(flag.name.as_str())
        }
        _ => None,
    }
}

pub(super) fn opens_with_a_choice(statements: &[Statement<'_>]) -> bool {
    let Some(first) = statements
        .iter()
        .find(|statement| !is_proxy_table(statement))
    else {
        return false;
    };
    matches!(single_var(first), Some((_, Some(Expression::ConditionalExpression(choice))))
        if matches!(choice.consequent, Expression::FunctionExpression(_)))
}

fn is_empty_function(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::FunctionExpression(function)
        if has_no_params(function)
            && function.body.as_ref().is_some_and(|body| body.statements.is_empty() && body.directives.is_empty()))
}

fn guarded_choice<'k, 'a>(
    statement: &'k Statement<'a>,
    flag: &str,
) -> Option<(&'k str, &'k Function<'a>)> {
    let (result, Some(Expression::ConditionalExpression(choice))) = single_var(statement)? else {
        return None;
    };
    let Expression::FunctionExpression(target) = &choice.consequent else {
        return None;
    };
    let shaped = is_named(&choice.test, flag)
        && is_empty_function(&choice.alternate)
        && has_no_params(target);
    shaped.then_some((result, target))
}

fn is_leading_alias(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(function) => {
            function.id.is_some()
                && statements_of(function)
                    .is_some_and(|body| matches!(body, [only] if returned(only).is_some()))
        }
        _ => matches!(single_var(statement), Some((_, Some(init)))
            if is_alias_or_constants(init) || is_wrapper_value(init)),
    }
}

fn is_wrapper_value(init: &Expression<'_>) -> bool {
    if super::narrowed() {
        return false;
    }
    match init {
        Expression::FunctionExpression(function) => {
            !function.generator && !function.r#async && is_proxy_function(function)
        }
        Expression::ArrowFunctionExpression(arrow) => {
            !arrow.r#async
                && arrow.params.rest.is_none()
                && arrow.params.items.len() <= PROXY_PARAMS
                && match &arrow.body {
                    ArrowFunctionBody::FunctionBody(body) => {
                        matches!(body.statements.as_slice(), [only] if returned(only).is_some())
                    }
                    _ => true,
                }
        }
        _ => false,
    }
}

fn is_constant_property(property: &ObjectPropertyKind<'_>) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return false;
    };
    property.kind == PropertyKind::Init
        && !property.computed
        && !property.method
        && is_table_value(&property.value)
}

fn is_table_value(value: &Expression<'_>) -> bool {
    match value {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
        Expression::FunctionExpression(function) => is_proxy_function(function),
        _ => value
            .as_member_expression()
            .is_some_and(|member| matches!(member.object(), Expression::Identifier(_))),
    }
}

fn is_proxy_function(function: &Function<'_>) -> bool {
    function.params.rest.is_none()
        && function.params.items.len() <= PROXY_PARAMS
        && statements_of(function)
            .is_some_and(|body| matches!(body, [only] if returned(only).is_some()))
}

fn is_constants_declaration(statement: &Statement<'_>) -> bool {
    matches!(single_var(statement), Some((_, Some(init @ Expression::ObjectExpression(_)))) if is_alias_or_constants(init))
}

fn is_proxy_table(statement: &Statement<'_>) -> bool {
    let Some((_, Some(Expression::ObjectExpression(object)))) = single_var(statement) else {
        return false;
    };
    let holds_a_proxy = object.properties.iter().any(|property| {
        matches!(property, ObjectPropertyKind::ObjectProperty(property)
            if matches!(&property.value, Expression::FunctionExpression(function) if is_proxy_function(function)))
    });
    holds_a_proxy && object.properties.iter().all(is_constant_property)
}

fn is_alias_or_constants(init: &Expression<'_>) -> bool {
    match init {
        Expression::Identifier(_) => true,
        Expression::ObjectExpression(object) => object.properties.iter().all(is_constant_property),
        _ => false,
    }
}

fn runs_once(target: &Function<'_>, caller: &str, callee: &str) -> bool {
    let Some((last, leading)) = statements_of(target).and_then(<[_]>::split_last) else {
        return false;
    };
    leading.iter().all(is_leading_alias) && guarded_call(last, caller, callee)
}

fn is_forwarding_call(expr: &Expression<'_>, caller: &str, callee: &str) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    let forwards = matches!(call.arguments.as_slice(),
        [Argument::Identifier(this), Argument::Identifier(rest)]
            if this.name == caller && rest.name == CALL_ARGUMENTS);
    !call.optional
        && forwards
        && call
            .callee
            .as_member_expression()
            .is_some_and(|member| !member.optional() && is_named(member.object(), callee))
}

fn guarded_call(statement: &Statement<'_>, caller: &str, callee: &str) -> bool {
    let Statement::IfStatement(guard) = statement else {
        return false;
    };
    if !is_named(&guard.test, callee) {
        return guard
            .alternate
            .as_ref()
            .is_some_and(|alternate| dead_code_branch(alternate, caller, callee));
    }
    let Statement::BlockStatement(block) = &guard.consequent else {
        return false;
    };
    let Some((first, rest)) = block.body.split_first() else {
        return false;
    };
    let Some((result, Some(call))) = single_var(first) else {
        return false;
    };
    guard.alternate.is_none()
        && is_named(&guard.test, callee)
        && is_forwarding_call(call, caller, callee)
        && closes(rest, callee, is_null, result)
}

fn dead_code_branch(alternate: &Statement<'_>, caller: &str, callee: &str) -> bool {
    match alternate {
        Statement::IfStatement(_) => guarded_call(alternate, caller, callee),
        Statement::BlockStatement(block) => {
            matches!(block.body.as_slice(), [only] if guarded_call(only, caller, callee))
        }
        _ => false,
    }
}
