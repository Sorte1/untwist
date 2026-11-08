use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use untwist_core::ast_util::matcher::iife;
use untwist_core::ast_util::prop::{identifier_name, member_parts};

use super::cookie::{
    counted_call, lookup_call, method_call, passes_format_check, property, tail_expressions,
    tests_removal_text,
};
use super::legacy::is_countdown_loop;
use crate::mutants::minified_rotators_trusted;

fn parameter_names<'a>(function: &'a Function<'a>) -> Vec<&'a str> {
    function
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

fn is_lookup(stmt: &Statement<'_>, names: (&str, &str)) -> Option<bool> {
    match stmt {
        Statement::VariableDeclaration(declaration) => {
            let mut lookups = 0;
            for declarator in &declaration.declarations {
                let Some(init) = declarator.init.as_ref() else {
                    continue;
                };
                let bound = matches!(&declarator.id,
                    BindingPattern::BindingIdentifier(id) if id.name == names.0);
                if !bound || !lookup_call(init, names) {
                    return None;
                }
                lookups += 1;
            }
            Some(lookups == 1)
        }
        Statement::ExpressionStatement(stmt) => {
            let Expression::AssignmentExpression(assign) = stmt.expression.without_parentheses()
            else {
                return None;
            };
            let target = matches!(&assign.left,
                AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == names.0);
            (target
                && assign.operator == AssignmentOperator::Assign
                && lookup_call(&assign.right, names))
            .then_some(true)
        }
        _ => None,
    }
}

fn bare(stmt: &Statement<'_>) -> bool {
    matches!(stmt, Statement::VariableDeclaration(declaration)
        if declaration.declarations.iter().all(|declarator| declarator.init.is_none()))
}

fn aliased_to<'k>(expr: &'k Expression<'_>, rotate: &str) -> Option<&'k str> {
    let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(alias) = &assign.left else {
        return None;
    };
    (assign.operator == AssignmentOperator::Assign
        && identifier_name(&assign.right) == Some(rotate)
        && alias.name != rotate)
        .then_some(alias.name.as_str())
}

fn reader_count(function: &Function<'_>, rotate: &str) -> Option<usize> {
    let names = parameter_names(function);
    let [matcher, key] = names.as_slice() else {
        return None;
    };
    let (matcher, key) = (*matcher, *key);
    if function.params.items.len() != 2 || function.params.rest.is_some() {
        return None;
    }
    let body = function.body.as_ref()?;
    let returns = body
        .statements
        .iter()
        .position(|stmt| matches!(stmt, Statement::ReturnStatement(_)))?;
    let (before, after) = body.statements.split_at(returns + 1);
    if !after.iter().all(bare) {
        return None;
    }
    let leading = before
        .iter()
        .take_while(|stmt| bare(stmt) || is_lookup(stmt, (matcher, key)).is_some())
        .count();
    let lookups: usize = before[..leading]
        .iter()
        .filter(|stmt| !bare(stmt))
        .filter_map(|stmt| is_lookup(stmt, (matcher, key)))
        .map(usize::from)
        .sum();
    if lookups != 1 {
        return None;
    }
    match tail_expressions(&before[leading..])?.as_slice() {
        [set, call, _] => counted_call(set, call, rotate),
        [alias, set, call, _] => counted_call(set, call, aliased_to(alias, rotate)?),
        _ => None,
    }
}

fn countdown_declaration<'k>(stmt: &'k Statement<'_>, target: &str) -> Option<&'k str> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    let names = parameter_names(function);
    let [counter] = names.as_slice() else {
        return None;
    };
    let counter = *counter;
    let body = function.body.as_ref()?;
    let simple = function.params.items.len() == 1
        && function.params.rest.is_none()
        && !function.generator
        && !function.r#async
        && body.directives.is_empty()
        && counter != target;
    let looped =
        matches!(body.statements.as_slice(), [stmt] if is_countdown_loop(stmt, counter, target));
    let name = function.id.as_ref()?.name.as_str();
    (simple && looped && name != target).then_some(name)
}

struct Cookie<'k, 'a> {
    holder: &'k str,
    object: &'k ObjectExpression<'a>,
}

fn object_of<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k ObjectExpression<'a>> {
    match expr.without_parentheses() {
        Expression::ObjectExpression(object) => Some(object),
        _ => None,
    }
}

fn chooses<'k, 'a>(expr: &'k Expression<'a>, holder: &str) -> Option<&'k Expression<'a>> {
    let Expression::ConditionalExpression(choice) = expr.without_parentheses() else {
        return None;
    };
    let reads = method_call(&choice.consequent, holder, "getCookie").is_some_and(|arguments| {
        matches!(arguments, [first, second]
            if matches!(first.as_expression(), Some(Expression::NullLiteral(_)))
            && matches!(second.as_expression(), Some(Expression::StringLiteral(_))))
    });
    let sets = method_call(&choice.alternate, holder, "setCookie").is_some();
    (reads && sets).then_some(&choice.test)
}

fn assigned_holder<'k, 'a>(expr: &'k Expression<'a>) -> Option<Cookie<'k, 'a>> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let (object, name) = member_parts(call.callee.without_parentheses())?;
    let Expression::AssignmentExpression(assign) = object.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(holder) = &assign.left else {
        return None;
    };
    let shaped = name == "updateCookie"
        && call.arguments.is_empty()
        && assign.operator == AssignmentOperator::Assign;
    shaped.then_some(Cookie {
        holder: holder.name.as_str(),
        object: object_of(&assign.right)?,
    })
}

fn assigned_guard<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<Cookie<'k, 'a>> {
    let [Statement::VariableDeclaration(declaration), Statement::ExpressionStatement(choice)] =
        statements
    else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let test = chooses(&choice.expression, id.name.as_str())?;
    let cookie = assigned_holder(test)?;
    (declarator.init.is_none() && cookie.holder == id.name).then_some(cookie)
}

fn single_binding<'k, 'a>(stmt: &'k Statement<'a>) -> Option<(&'k str, &'k Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.name.as_str(), declarator.init.as_ref()?))
}

fn block_guard<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<Cookie<'k, 'a>> {
    let [Statement::BlockStatement(block)] = statements else {
        return None;
    };
    let [object, check, Statement::ExpressionStatement(choice)] = block.body.as_slice() else {
        return None;
    };
    let (holder, object) = single_binding(object)?;
    let (flag, init) = single_binding(check)?;
    let checked = method_call(init, holder, "updateCookie").is_some_and(<[_]>::is_empty);
    let test = chooses(&choice.expression, holder)?;
    (checked && identifier_name(test) == Some(flag) && flag != holder).then_some(Cookie {
        holder,
        object: object_of(object)?,
    })
}

fn guarded_count(cookie: &Cookie<'_, '_>, rotate: &str, source: &str) -> Option<usize> {
    let update = property(cookie.object, "updateCookie")?;
    let removal = property(cookie.object, "removeCookie")?;
    let reader = property(cookie.object, "getCookie")?;
    property(cookie.object, "setCookie")?;
    if !tests_removal_text(update, cookie.holder) {
        return None;
    }
    let count = reader_count(reader, rotate)?;
    let span = removal.span();
    let text = source.get(span.start as usize..span.end as usize)?;
    passes_format_check(text).then_some(count)
}

fn references(scoping: &Scoping, function: &Function<'_>, name: &str) -> Option<usize> {
    let scope = function.scope_id.get()?;
    let symbol = scoping.find_binding(scope, name.into())?;
    Some(scoping.get_resolved_reference_ids(symbol).len())
}

pub(super) fn minified_cookie_rotation(
    stmt: &Statement<'_>,
    array: &str,
    (scoping, source): (&Scoping, &str),
) -> Option<usize> {
    let Statement::ExpressionStatement(outer) = stmt else {
        return None;
    };
    let call = match outer.expression.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    let (function, arguments) = iife(call)?;
    let [argument] = arguments.as_slice() else {
        return None;
    };
    let names = parameter_names(function);
    let [target] = names.as_slice() else {
        return None;
    };
    let target = *target;
    if argument.as_expression().and_then(identifier_name) != Some(array)
        || function.params.items.len() != 1
        || function.params.rest.is_some()
    {
        return None;
    }
    let body = function.body.as_ref()?;
    let (rotator, guard) = body.statements.split_first()?;
    let rotate = countdown_declaration(rotator, target)?;
    let sealed = body.directives.is_empty()
        && references(scoping, function, rotate) == Some(1)
        && references(scoping, function, target) == Some(2);
    if !sealed && !minified_rotators_trusted() {
        return None;
    }
    let cookie = assigned_guard(guard).or_else(|| block_guard(guard))?;
    guarded_count(&cookie, rotate, source)
}
