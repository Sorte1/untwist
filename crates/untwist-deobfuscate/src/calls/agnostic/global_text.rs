use oxc_ast::ast::{Argument, Expression, Program, Statement};

const NESTED_TEXTS: u8 = 3;

pub(super) fn returns_the_global_object(program: &Program<'_>) -> bool {
    returns_global(program, NESTED_TEXTS)
}

fn returns_global(program: &Program<'_>, depth: u8) -> bool {
    returned(program).is_some_and(|value| is_global_object(value, depth))
}

fn returned<'p>(program: &'p Program<'p>) -> Option<&'p Expression<'p>> {
    if !program.directives.is_empty() {
        return None;
    }
    if let Some(value) = sole_return(&program.body) {
        return Some(value);
    }
    let [Statement::ExpressionStatement(statement)] = program.body.as_slice() else {
        return None;
    };
    let Expression::FunctionExpression(function) = statement.expression.without_parentheses()
    else {
        return None;
    };
    let body = function.body.as_ref()?;
    if !body.directives.is_empty() {
        return None;
    }
    sole_return(&body.statements)
}

fn sole_return<'p>(statements: &'p [Statement<'p>]) -> Option<&'p Expression<'p>> {
    let mut statements = statements
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)));
    let (Some(Statement::ReturnStatement(returned)), None) = (statements.next(), statements.next())
    else {
        return None;
    };
    returned.argument.as_ref()
}

fn is_global_object(value: &Expression<'_>, depth: u8) -> bool {
    match value.without_parentheses() {
        Expression::ThisExpression(_) => true,
        Expression::CallExpression(call) if call.arguments.is_empty() && !call.optional => {
            constructed_text(&call.callee).is_some_and(|text| text_returns_global(&text, depth))
        }
        _ => false,
    }
}

fn constructed_text(callee: &Expression<'_>) -> Option<String> {
    let Expression::CallExpression(call) = callee.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let Expression::FunctionExpression(function) = member.object.without_parentheses() else {
        return None;
    };
    let empty = function.params.items.is_empty()
        && function.params.rest.is_none()
        && !function.r#async
        && !function.generator
        && function
            .body
            .as_ref()
            .is_some_and(|body| body.statements.is_empty() && body.directives.is_empty());
    if member.property.name != "constructor" || member.optional || call.optional || !empty {
        return None;
    }
    let [Argument::StringLiteral(literal)] = call.arguments.as_slice() else {
        return None;
    };
    literal.value.as_str().map(str::to_string)
}

fn text_returns_global(text: &str, depth: u8) -> bool {
    let Some(next) = depth.checked_sub(1) else {
        return false;
    };
    crate::proof::with_parsed_text(text, |program| returns_global(program, next))
        .flatten()
        .unwrap_or(false)
}
