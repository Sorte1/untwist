use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};

use crate::matcher::literal_id;

const DEFERRED: &str = "O";
const STARTUP: &str = "x";
const ENSURE: &str = "e";
const ENTRY_SLOT: &str = "s";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Startup {
    pub entry: String,
    pub chunks: Vec<String>,
    pub spans: Vec<Span>,
}

fn is_name(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn member_call<'k, 'a>(
    expr: &'k Expression<'a>,
    require: &str,
    name: &str,
) -> Option<&'k [Argument<'a>]> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    (!call.optional && member.property.name == name && is_name(&member.object, require))
        .then_some(&call.arguments[..])
}

fn ids(expr: &Expression<'_>) -> Option<Vec<String>> {
    let Expression::ArrayExpression(array) = expr.without_parentheses() else {
        return None;
    };
    array
        .elements
        .iter()
        .map(|element| {
            element
                .as_expression()
                .and_then(|e| literal_id(e.without_parentheses()))
        })
        .collect()
}

fn is_undefined(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name == "undefined",
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        Expression::NumericLiteral(number) => number.value == 0.0,
        _ => false,
    }
}

fn arrow_value<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k ArrowFunctionExpression<'a>, &'k Expression<'a>)> {
    let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
        return None;
    };
    if arrow.r#async {
        return None;
    }
    let value = match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => {
            let [Statement::ReturnStatement(ret)] = &body.statements[..] else {
                return None;
            };
            ret.argument.as_ref()?
        }
        body => body.as_expression()?,
    };
    Some((arrow, value.without_parentheses()))
}

pub fn required_entry(expr: &Expression<'_>, require: &str, exec: Option<&str>) -> Option<String> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let [only] = &call.arguments[..] else {
        return None;
    };
    let argument = only.as_expression()?.without_parentheses();
    if exec.is_some_and(|exec| is_name(&call.callee, exec)) {
        return literal_id(argument);
    }
    if !is_name(&call.callee, require) || call.optional {
        return None;
    }
    if let Expression::AssignmentExpression(slot) = argument {
        let AssignmentTarget::StaticMemberExpression(member) = &slot.left else {
            return None;
        };
        let entry = member.property.name == ENTRY_SLOT
            && is_name(&member.object, require)
            && slot.operator == AssignmentOperator::Assign;
        return entry.then(|| literal_id(slot.right.without_parentheses()))?;
    }
    literal_id(argument)
}

fn deferred_start(
    expr: &Expression<'_>,
    require: &str,
    exec: Option<&str>,
) -> Option<(String, Vec<String>)> {
    let [result, chunks, start] = member_call(expr, require, DEFERRED)? else {
        return None;
    };
    if !is_undefined(result.as_expression()?) {
        return None;
    }
    let chunks = ids(chunks.as_expression()?)?;
    let (arrow, value) = arrow_value(start.as_expression()?)?;
    if !arrow.params.items.is_empty() || arrow.params.rest.is_some() {
        return None;
    }
    Some((required_entry(value, require, exec)?, chunks))
}

fn reruns(expr: &Expression<'_>, require: &str, exports: &str) -> bool {
    let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return false;
    };
    let Some([argument]) = member_call(&assignment.right, require, DEFERRED) else {
        return false;
    };
    assignment.operator == AssignmentOperator::Assign
        && target.name == exports
        && argument
            .as_expression()
            .is_some_and(|a| is_name(a, exports))
}

fn single_declarator<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(&'k str, &'k Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = &declaration.declarations[..] else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.name.as_str(), declarator.init.as_ref()?))
}

fn expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(statement) => Some(&statement.expression),
        _ => None,
    }
}

fn deferred_pair(
    statements: &[Statement<'_>],
    require: &str,
    exec: Option<&str>,
) -> Option<(String, Vec<String>, String)> {
    let [first, second, ..] = statements else {
        return None;
    };
    let (exports, init) = single_declarator(first)?;
    let (entry, chunks) = deferred_start(init, require, exec)?;
    reruns(expression(second)?, require, exports).then(|| (entry, chunks, exports.to_string()))
}

pub fn top_startup(statements: &[Statement<'_>], require: &str) -> Option<Startup> {
    for (index, statement) in statements.iter().enumerate() {
        if let Some((entry, chunks, _)) = deferred_pair(&statements[index..], require, None) {
            let spans = vec![statement.span(), statements[index + 1].span()];
            return Some(Startup {
                entry,
                chunks,
                spans,
            });
        }
    }
    startup_function(statements, require)
}

fn startup_definition<'k, 'a>(
    statement: &'k Statement<'a>,
    require: &str,
) -> Option<&'k AssignmentExpression<'a>> {
    let Expression::AssignmentExpression(assignment) = expression(statement)?.without_parentheses()
    else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign
        && member.property.name == STARTUP
        && is_name(&member.object, require))
    .then_some(assignment)
}

fn startup_body(
    assignment: &AssignmentExpression<'_>,
    require: &str,
) -> Option<(String, Vec<String>)> {
    let Expression::ArrowFunctionExpression(arrow) = assignment.right.without_parentheses() else {
        return None;
    };
    if arrow.r#async || !arrow.params.items.is_empty() {
        return None;
    }
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        return None;
    };
    let [_, _, Statement::ReturnStatement(ret)] = &body.statements[..] else {
        return None;
    };
    let (entry, chunks, exports) = deferred_pair(&body.statements, require, None)?;
    ret.argument
        .as_ref()
        .is_some_and(|argument| is_name(argument, &exports))
        .then_some((entry, chunks))
}

fn dependency_wrapper(statement: &Statement<'_>, require: &str) -> Option<Vec<String>> {
    let Expression::CallExpression(call) = expression(statement)?.without_parentheses() else {
        return None;
    };
    let Expression::ArrowFunctionExpression(outer) = call.callee.without_parentheses() else {
        return None;
    };
    if !call.arguments.is_empty() || !outer.params.items.is_empty() || outer.r#async {
        return None;
    }
    let ArrowFunctionBody::FunctionBody(body) = &outer.body else {
        return None;
    };
    let [saved, redefined] = &body.statements[..] else {
        return None;
    };
    let (next, value) = single_declarator(saved)?;
    let Expression::StaticMemberExpression(read) = value.without_parentheses() else {
        return None;
    };
    if read.property.name != STARTUP || !is_name(&read.object, require) {
        return None;
    }
    let assignment = startup_definition(redefined, require)?;
    let Expression::ArrowFunctionExpression(inner) = assignment.right.without_parentheses() else {
        return None;
    };
    let ArrowFunctionBody::FunctionBody(inner) = &inner.body else {
        return None;
    };
    let (last, loads) = inner.statements.split_last()?;
    let Statement::ReturnStatement(ret) = last else {
        return None;
    };
    let calls_next = ret.argument.as_ref().is_some_and(|argument| {
        matches!(argument.without_parentheses(), Expression::CallExpression(call)
            if call.arguments.is_empty() && is_name(&call.callee, next))
    });
    if !calls_next {
        return None;
    }
    loads
        .iter()
        .map(|statement| {
            let [id] = member_call(expression(statement)?, require, ENSURE)? else {
                return None;
            };
            literal_id(id.as_expression()?.without_parentheses())
        })
        .collect()
}

fn startup_function(statements: &[Statement<'_>], require: &str) -> Option<Startup> {
    let (at, assignment) = statements
        .iter()
        .enumerate()
        .find_map(|(at, statement)| startup_definition(statement, require).map(|a| (at, a)))?;
    let (entry, mut chunks) = startup_body(assignment, require)?;
    let mut spans = vec![statements[at].span()];
    for statement in &statements[at + 1..] {
        if let Some(loaded) = dependency_wrapper(statement, require) {
            chunks.extend(loaded);
            spans.push(statement.span());
            continue;
        }
        let Some((_, init)) = single_declarator(statement) else {
            continue;
        };
        if member_call(init, require, STARTUP).is_some_and(<[_]>::is_empty) {
            spans.push(statement.span());
            return Some(Startup {
                entry,
                chunks,
                spans,
            });
        }
    }
    None
}

pub fn chunk_startup(start: &Expression<'_>) -> Option<Startup> {
    let (require, body) = match start.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) => {
            match (&arrow.params.items[..], &arrow.body) {
                ([param], ArrowFunctionBody::FunctionBody(body)) if !arrow.r#async => (param, body),
                _ => return None,
            }
        }
        Expression::FunctionExpression(function) => match &function.params.items[..] {
            [param] if !function.r#async && !function.generator => (param, function.body.as_ref()?),
            _ => return None,
        },
        _ => return None,
    };
    let BindingPattern::BindingIdentifier(require) = &require.pattern else {
        return None;
    };
    let require = require.name.as_str();
    let (first, rest) = body.statements.split_first()?;
    let (exec, value) = single_declarator(first)?;
    let (arrow, called) = arrow_value(value)?;
    let [param] = &arrow.params.items[..] else {
        return None;
    };
    let BindingPattern::BindingIdentifier(param) = &param.pattern else {
        return None;
    };
    let Expression::CallExpression(call) = called else {
        return None;
    };
    let [argument] = &call.arguments[..] else {
        return None;
    };
    let forwards = match argument.as_expression()?.without_parentheses() {
        Expression::Identifier(id) => id.name == param.name,
        Expression::AssignmentExpression(slot) => {
            matches!(&slot.left, AssignmentTarget::StaticMemberExpression(member)
                if member.property.name == ENTRY_SLOT && is_name(&member.object, require))
                && slot.operator == AssignmentOperator::Assign
                && is_name(&slot.right, &param.name)
        }
        _ => false,
    };
    if !forwards || !is_name(&call.callee, require) {
        return None;
    }
    match rest {
        [only] => {
            let (_, init) = single_declarator(only)?;
            let entry = required_entry(init, require, Some(exec))?;
            Some(Startup {
                entry,
                ..Startup::default()
            })
        }
        _ => None,
    }
}

pub struct Deferred {
    pub spans: Vec<Span>,
    pub entry: Option<(String, Vec<String>)>,
}

fn pushed(expr: &Expression<'_>) -> Option<(String, Vec<String>)> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(push) = call.callee.without_parentheses() else {
        return None;
    };
    if push.property.name != "push" || !matches!(push.object, Expression::Identifier(_)) {
        return None;
    }
    let [only] = &call.arguments[..] else {
        return None;
    };
    let mut list = ids(only.as_expression()?)?;
    if list.is_empty() {
        return None;
    }
    let entry = list.remove(0);
    Some((entry, list))
}

fn checks(statement: &Statement<'_>, check: &str) -> bool {
    let call = match statement {
        Statement::ExpressionStatement(statement) => &statement.expression,
        Statement::ReturnStatement(ret) => match &ret.argument {
            Some(argument) => argument,
            None => return false,
        },
        _ => return false,
    };
    matches!(call.without_parentheses(), Expression::CallExpression(call)
        if call.arguments.is_empty() && is_name(&call.callee, check))
}

pub fn webpack4_deferred(statements: &[Statement<'_>], check: &str) -> Option<Deferred> {
    let (last, rest) = statements.split_last()?;
    if !checks(last, check) {
        return None;
    }
    let pushed = rest.last().and_then(expression).and_then(pushed);
    let mut spans = vec![last.span()];
    if pushed.is_some() {
        spans.push(rest.last()?.span());
    }
    Some(Deferred {
        spans,
        entry: pushed,
    })
}
