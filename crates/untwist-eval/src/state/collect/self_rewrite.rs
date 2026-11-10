use oxc_ast::ast::{
    Argument, ArrayExpressionElement, AssignmentTarget, BindingPattern, CallExpression, Expression,
    Function, Statement, VariableDeclaration,
};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};

use crate::mutants::{template_arity_trusted, template_prefix_trusted};

pub(in crate::state) fn template_rewrite(
    function: &Function<'_>,
    own_name: &str,
    array: &str,
) -> Option<Span> {
    let body = function.body.as_ref()?;
    let rewrite = body
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::ExpressionStatement(node) => rewrite_span(&node.expression, own_name),
            Statement::ReturnStatement(node) => match node.argument.as_ref()?.without_parentheses()
            {
                Expression::SequenceExpression(seq) => seq
                    .expressions
                    .first()
                    .and_then(|first| rewrite_span(first, own_name)),
                Expression::CallExpression(call) => rewrite_span(&call.callee, own_name),
                _ => None,
            },
            _ => None,
        });
    rewrite.filter(|_| outer_is_template(function, own_name, array))
}

fn rewrite_span(expr: &Expression<'_>, own_name: &str) -> Option<Span> {
    is_self_rewrite_assignment(expr, own_name).then(|| expr.without_parentheses().span())
}

fn has_plain_parameters(function: &Function<'_>) -> bool {
    function.params.rest.is_none()
        && function
            .params
            .items
            .iter()
            .all(|param| param.initializer.is_none())
}

fn is_inert_element(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::UnaryExpression(node) if node.operator == UnaryOperator::Void => {
            is_inert_element(&node.argument)
        }
        Expression::RegExpLiteral(_) => false,
        other => other.is_literal(),
    }
}

fn is_array_of_literals(expr: &Expression<'_>) -> bool {
    let Expression::ArrayExpression(array) = expr.without_parentheses() else {
        return false;
    };
    array.elements.iter().all(|element| match element {
        ArrayExpressionElement::Elision(_) => true,
        ArrayExpressionElement::SpreadElement(_) => false,
        other => other.as_expression().is_some_and(is_inert_element),
    })
}

fn is_array_fetch(expr: &Expression<'_>, array: &str, shadowed: &[&str]) -> bool {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return false;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    let name = callee.name.as_str();
    !call.optional && call.arguments.is_empty() && name == array && !shadowed.contains(&name)
}

fn is_known_declaration(
    declaration: &VariableDeclaration<'_>,
    function: &Function<'_>,
    own_name: &str,
    array: &str,
) -> bool {
    let [declarator] = declaration.declarations.as_slice() else {
        return false;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return false;
    };
    let mut shadowed: Vec<&str> = function
        .params
        .items
        .iter()
        .filter_map(|param| param.pattern.get_binding_identifier())
        .map(|param| param.name.as_str())
        .collect();
    shadowed.push(id.name.as_str());
    match &declarator.init {
        None => true,
        Some(init) if own_name == array => is_array_of_literals(init),
        Some(init) => is_array_fetch(init, array, &shadowed),
    }
}

fn inner_fits_the_forward(function: &Function<'_>, own_name: &str) -> bool {
    let Some(body) = &function.body else {
        return false;
    };
    let Some(inner) = self_rewrite_target(&body.statements, Some(own_name)) else {
        return false;
    };
    template_arity_trusted()
        || (inner.params.rest.is_none() && inner.params.items.len() <= function.params.items.len())
}

fn outer_is_template(function: &Function<'_>, own_name: &str, array: &str) -> bool {
    let Some(body) = &function.body else {
        return false;
    };
    if !has_plain_parameters(function) || !inner_fits_the_forward(function, own_name) {
        return false;
    }
    let mut statements = body.statements.iter();
    let mut current = statements.next();
    if let Some(Statement::VariableDeclaration(declaration)) = current {
        if !template_prefix_trusted()
            && !is_known_declaration(declaration, function, own_name, array)
        {
            return false;
        }
        current = statements.next();
    }
    let matched = match current {
        Some(Statement::ExpressionStatement(expr_stmt)) => {
            is_self_rewrite_assignment(&expr_stmt.expression, own_name)
                && match statements.next() {
                    Some(Statement::ReturnStatement(ret)) => {
                        is_forwarding_call(ret.argument.as_ref(), own_name, function)
                    }
                    _ => false,
                }
        }
        Some(Statement::ReturnStatement(ret)) => {
            is_forwarding_sequence(ret.argument.as_ref(), own_name, function)
                || is_rewrite_and_call(ret.argument.as_ref(), own_name, function)
        }
        _ => false,
    };
    matched && statements.next().is_none()
}

fn is_self_rewrite_assignment(expr: &Expression<'_>, own_name: &str) -> bool {
    let Expression::AssignmentExpression(node) = expr.without_parentheses() else {
        return false;
    };
    if node.operator != AssignmentOperator::Assign {
        return false;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &node.left else {
        return false;
    };
    id.name.as_str() == own_name && matches!(node.right, Expression::FunctionExpression(_))
}

fn is_forwarding_sequence(
    argument: Option<&Expression<'_>>,
    own_name: &str,
    function: &Function<'_>,
) -> bool {
    let Some(Expression::SequenceExpression(seq)) = argument.map(Expression::without_parentheses)
    else {
        return false;
    };
    let [first, second] = seq.expressions.as_slice() else {
        return false;
    };
    is_self_rewrite_assignment(first, own_name)
        && is_forwarding_call(Some(second), own_name, function)
}

fn is_rewrite_and_call(
    argument: Option<&Expression<'_>>,
    own_name: &str,
    function: &Function<'_>,
) -> bool {
    let Some(Expression::CallExpression(call)) = argument.map(Expression::without_parentheses)
    else {
        return false;
    };
    !call.optional
        && is_self_rewrite_assignment(&call.callee, own_name)
        && call.arguments.len() == function.params.items.len()
        && call
            .arguments
            .iter()
            .zip(function.params.items.iter())
            .all(|(arg, param)| forwarded_argument_matches(arg, &param.pattern))
}

fn is_forwarding_call(
    argument: Option<&Expression<'_>>,
    own_name: &str,
    function: &Function<'_>,
) -> bool {
    let Some(Expression::CallExpression(call)) = argument.map(Expression::without_parentheses)
    else {
        return false;
    };
    is_forwarding_call_expression(call, own_name, function)
}

fn is_forwarding_call_expression(
    call: &CallExpression<'_>,
    own_name: &str,
    function: &Function<'_>,
) -> bool {
    if call.optional {
        return false;
    }
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    if callee.name.as_str() != own_name {
        return false;
    }
    if call.arguments.len() != function.params.items.len() {
        return false;
    }
    call.arguments
        .iter()
        .zip(function.params.items.iter())
        .all(|(arg, param)| forwarded_argument_matches(arg, &param.pattern))
}

fn forwarded_argument_matches(argument: &Argument<'_>, param_pattern: &BindingPattern<'_>) -> bool {
    let Some(arg_expr) = argument.as_expression() else {
        return false;
    };
    let Expression::Identifier(arg_id) = arg_expr.without_parentheses() else {
        return false;
    };
    let BindingPattern::BindingIdentifier(param_id) = param_pattern else {
        return false;
    };
    arg_id.name.as_str() == param_id.name.as_str()
}

pub(in crate::state) fn self_rewrite_target<'a, 'n>(
    statements: &'n [Statement<'a>],
    own_name: Option<&str>,
) -> Option<&'n Function<'a>> {
    let own_name = own_name?;
    let mut found = None;
    scan_for_self_rewrite(statements, own_name, &mut found);
    found
}

fn scan_for_self_rewrite<'a, 'n>(
    statements: &'n [Statement<'a>],
    own_name: &str,
    found: &mut Option<&'n Function<'a>>,
) {
    for statement in statements {
        scan_statement_for_self_rewrite(statement, own_name, found);
    }
}

fn scan_statement_for_self_rewrite<'a, 'n>(
    statement: &'n Statement<'a>,
    own_name: &str,
    found: &mut Option<&'n Function<'a>>,
) {
    match statement {
        Statement::ReturnStatement(node) => {
            if let Some(argument) = &node.argument {
                scan_expression_for_self_rewrite(argument, own_name, found);
            }
        }
        Statement::ExpressionStatement(node) => {
            scan_expression_for_self_rewrite(&node.expression, own_name, found);
        }
        _ => {}
    }
}

fn scan_expression_for_self_rewrite<'a, 'n>(
    expr: &'n Expression<'a>,
    own_name: &str,
    found: &mut Option<&'n Function<'a>>,
) {
    match expr {
        Expression::SequenceExpression(node) => {
            for item in &node.expressions {
                scan_expression_for_self_rewrite(item, own_name, found);
            }
        }
        Expression::ParenthesizedExpression(node) => {
            scan_expression_for_self_rewrite(&node.expression, own_name, found);
        }
        Expression::CallExpression(node) => {
            scan_expression_for_self_rewrite(&node.callee, own_name, found);
        }
        Expression::AssignmentExpression(node) => {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &node.left {
                if id.name.as_str() == own_name {
                    if let Expression::FunctionExpression(inner) = &node.right {
                        *found = Some(inner);
                    }
                }
            }
        }
        _ => {}
    }
}
