use oxc_ast::ast::*;

pub const DUMP: &str = "_DumpException";
pub const INITIALIZER: &str = "_ModuleManager_initialize";

pub struct Shell<'k, 'a> {
    pub namespace: &'k str,
    pub param: &'k BindingIdentifier<'a>,
    pub function: &'k Function<'a>,
    pub body: &'k FunctionBody<'a>,
}

fn member_of_this<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k str> {
    let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
        return None;
    };
    matches!(
        member.object.without_parentheses(),
        Expression::ThisExpression(_)
    )
    .then(|| member.property.name.as_str())
}

fn namespace_of<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k str> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = statement.expression.without_parentheses()
    else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::StaticMemberExpression(target) = &assign.left else {
        return None;
    };
    if !matches!(
        target.object.without_parentheses(),
        Expression::ThisExpression(_)
    ) {
        return None;
    }
    let name = target.property.name.as_str();
    let Expression::LogicalExpression(logical) = assign.right.without_parentheses() else {
        return None;
    };
    let empty = matches!(
        logical.right.without_parentheses(),
        Expression::ObjectExpression(object) if object.properties.is_empty()
    );
    (logical.operator == LogicalOperator::Or
        && empty
        && member_of_this(&logical.left) == Some(name))
    .then_some(name)
}

fn wrapper_function<'k, 'a>(call: &'k CallExpression<'a>) -> Option<&'k Function<'a>> {
    let Expression::StaticMemberExpression(callee) = call.callee.without_parentheses() else {
        return None;
    };
    if callee.property.name != "call" || call.optional {
        return None;
    }
    match callee.object.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function),
        _ => None,
    }
}

fn single_param<'k, 'a>(function: &'k Function<'a>) -> Option<&'k BindingIdentifier<'a>> {
    let params = &function.params;
    if params.rest.is_some() || params.items.len() != 1 {
        return None;
    }
    let item = &params.items[0];
    match (&item.pattern, &item.initializer) {
        (BindingPattern::BindingIdentifier(id), None) => Some(id),
        _ => None,
    }
}

pub fn shell<'k, 'a>(top: &'k [Statement<'a>]) -> Option<Shell<'k, 'a>> {
    let [first, second] = top else {
        return None;
    };
    let namespace = namespace_of(first)?;
    let Statement::ExpressionStatement(statement) = second else {
        return None;
    };
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return None;
    };
    let function = wrapper_function(call)?;
    if function.r#async || function.generator || function.id.is_some() {
        return None;
    }
    let [receiver, shared] = &call.arguments[..] else {
        return None;
    };
    let receiver_is_this = matches!(
        receiver.as_expression()?.without_parentheses(),
        Expression::ThisExpression(_)
    );
    if !receiver_is_this || member_of_this(shared.as_expression()?) != Some(namespace) {
        return None;
    }
    Some(Shell {
        namespace,
        param: single_param(function)?,
        function,
        body: function.body.as_deref()?,
    })
}

pub fn guarded(attempt: &TryStatement<'_>, param: &str) -> bool {
    if attempt.finalizer.is_some() {
        return false;
    }
    let Some(handler) = attempt.handler.as_deref() else {
        return false;
    };
    let Some(CatchParameter {
        pattern: BindingPattern::BindingIdentifier(caught),
        ..
    }) = &handler.param
    else {
        return false;
    };
    let [Statement::ExpressionStatement(statement)] = &handler.body.body[..] else {
        return false;
    };
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return false;
    };
    let Expression::StaticMemberExpression(callee) = call.callee.without_parentheses() else {
        return false;
    };
    let on_param = matches!(
        callee.object.without_parentheses(),
        Expression::Identifier(id) if id.name == param
    );
    let passes_caught = matches!(
        &call.arguments[..],
        [Argument::Identifier(id)] if id.name == caught.name
    );
    on_param && callee.property.name == DUMP && passes_caught
}

pub fn is_var(statement: &Statement<'_>) -> bool {
    matches!(statement, Statement::VariableDeclaration(_))
}

pub fn probe_body(shell: &Shell<'_, '_>) -> bool {
    shell
        .body
        .statements
        .iter()
        .find(|statement| !is_var(statement))
        .is_some_and(|statement| {
            matches!(statement, Statement::TryStatement(attempt) if guarded(attempt, &shell.param.name))
        })
}
