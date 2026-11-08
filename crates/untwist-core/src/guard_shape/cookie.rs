use oxc_ast::ast::*;
use oxc_syntax::operator::AssignmentOperator;

use super::state::{STATE_HEAD, STATE_TAIL};
use super::{has_no_params, is_named, is_string, returned, single_var, statements_of};
use crate::ast_util::prop::{literal_key, member_parts, prop_name};

const READ_KEY: &str = "removeCookie";
const CHECK_KEY: &str = "updateCookie";

pub(super) fn opens_like_a_cookie(statements: &[Statement<'_>]) -> bool {
    let [Statement::VariableDeclaration(first), Statement::ExpressionStatement(second), ..] =
        statements
    else {
        return false;
    };
    let assigns_the_check = matches!(&second.expression, Expression::AssignmentExpression(assign)
        if matches!(&assign.left, AssignmentTarget::StaticMemberExpression(member)
            if member.property.name == CHECK_KEY)
            || matches!(&assign.left, AssignmentTarget::ComputedMemberExpression(member)
            if literal_key(&member.expression).as_deref() == Some(CHECK_KEY)));
    matches!(first.declarations.as_slice(),
        [only] if matches!(only.init, Some(Expression::ObjectExpression(_))))
        && assigns_the_check
}

pub(super) fn cookie_target<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<&'k Function<'a>> {
    let [first, Statement::ExpressionStatement(second), ..] = statements else {
        return None;
    };
    let (holder, Some(Expression::ObjectExpression(object))) = single_var(first)? else {
        return None;
    };
    let check = assigned_check(&second.expression, holder)?;
    if !tests_the_read(check, holder) {
        return None;
    }
    read_function(object)
}

fn is_member_of(expr: &Expression<'_>, holder: &str, key: &str) -> bool {
    member_parts(expr).is_some_and(|(object, name)| name == key && is_named(object, holder))
}

fn assigned_check<'k, 'a>(expr: &'k Expression<'a>, holder: &str) -> Option<&'k Function<'a>> {
    let Expression::AssignmentExpression(assign) = expr else {
        return None;
    };
    let on_holder = match &assign.left {
        AssignmentTarget::StaticMemberExpression(member) => {
            member.property.name == CHECK_KEY && is_named(&member.object, holder)
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            literal_key(&member.expression).as_deref() == Some(CHECK_KEY)
                && is_named(&member.object, holder)
        }
        _ => false,
    };
    let Expression::FunctionExpression(check) = &assign.right else {
        return None;
    };
    (assign.operator == AssignmentOperator::Assign && on_holder && has_no_params(check))
        .then_some(check)
}

fn tests_the_read(check: &Function<'_>, holder: &str) -> bool {
    let Some([only]) = statements_of(check) else {
        return false;
    };
    let Some(Expression::CallExpression(test)) = returned(only) else {
        return false;
    };
    let Some((regex, method)) = member_parts(&test.callee) else {
        return false;
    };
    let [Argument::CallExpression(source)] = test.arguments.as_slice() else {
        return false;
    };
    let reads = source.arguments.is_empty()
        && member_parts(&source.callee)
            .is_some_and(|(read, name)| name == "toString" && is_member_of(read, holder, READ_KEY));
    reads && method == "test" && is_cookie_regex(regex)
}

fn is_cookie_regex(expr: &Expression<'_>) -> bool {
    let Expression::NewExpression(built) = expr.without_parentheses() else {
        return false;
    };
    let pattern = format!("{STATE_HEAD}{STATE_TAIL}");
    is_named(&built.callee, "RegExp")
        && matches!(built.arguments.as_slice(),
            [only] if only.as_expression().is_some_and(|text| is_string(text, &pattern)))
}

fn read_function<'k, 'a>(object: &'k ObjectExpression<'a>) -> Option<&'k Function<'a>> {
    let mut reads = object
        .properties
        .iter()
        .filter_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if !property.computed && prop_name(&property.key).as_deref() == Some(READ_KEY) =>
            {
                Some(property)
            }
            _ => None,
        });
    let property = reads.next()?;
    if reads.next().is_some() || property.kind != PropertyKind::Init || property.method {
        return None;
    }
    let Expression::FunctionExpression(read) = &property.value else {
        return None;
    };
    let returns_text = matches!(statements_of(read), Some([only])
        if matches!(returned(only), Some(Expression::StringLiteral(_))));
    let plain = read
        .body
        .as_ref()
        .is_some_and(|body| body.directives.is_empty());
    (has_no_params(read) && returns_text && plain && !read.r#async && !read.generator)
        .then_some(&**read)
}

fn checks_a_holder(check: &Function<'_>) -> bool {
    let Some([only]) = statements_of(check) else {
        return false;
    };
    let Some(Expression::CallExpression(test)) = returned(only) else {
        return false;
    };
    let Some((regex, method)) = member_parts(&test.callee) else {
        return false;
    };
    let [Argument::CallExpression(source)] = test.arguments.as_slice() else {
        return false;
    };
    let reads = source.arguments.is_empty()
        && member_parts(&source.callee).is_some_and(|(read, name)| {
            name == "toString"
                && member_parts(read).is_some_and(|(holder, key)| {
                    key == READ_KEY && matches!(holder, Expression::Identifier(_))
                })
        });
    reads && method == "test" && is_cookie_regex(regex)
}

pub(super) fn holder_target<'k, 'a>(object: &'k ObjectExpression<'a>) -> Option<&'k Function<'a>> {
    let check = object
        .properties
        .iter()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if !property.computed && prop_name(&property.key).as_deref() == Some(CHECK_KEY) =>
            {
                match &property.value {
                    Expression::FunctionExpression(check) => Some(&**check),
                    _ => None,
                }
            }
            _ => None,
        })?;
    (has_no_params(check) && checks_a_holder(check))
        .then(|| read_function(object))
        .flatten()
}
