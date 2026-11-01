use oxc_ast::ast::{BindingPattern, Expression, Program, Statement};
use oxc_span::GetSpan;
use untwist_eval::decode::Call;

const ALIAS_HOP_CAP: usize = 16;

fn resolve_decoder_index(name: &str, names: &[&str], aliases: &[(&str, &str)]) -> Option<usize> {
    let mut current = name;
    for _ in 0..ALIAS_HOP_CAP {
        if let Some(index) = names.iter().position(|candidate| *candidate == current) {
            return Some(index);
        }
        current = aliases.iter().rev().find(|(alias, _)| *alias == current)?.1;
    }
    None
}

fn is_self_contained(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(node) => is_self_contained(&node.argument),
        Expression::BinaryExpression(node) => {
            is_self_contained(&node.left) && is_self_contained(&node.right)
        }
        Expression::LogicalExpression(node) => {
            is_self_contained(&node.left) && is_self_contained(&node.right)
        }
        Expression::ConditionalExpression(node) => {
            is_self_contained(&node.test)
                && is_self_contained(&node.consequent)
                && is_self_contained(&node.alternate)
        }
        Expression::ParenthesizedExpression(node) => is_self_contained(&node.expression),
        Expression::SequenceExpression(node) => node.expressions.iter().all(is_self_contained),
        _ => false,
    }
}

fn record_alias<'a>(
    declarator: &'a oxc_ast::ast::VariableDeclarator<'a>,
    aliases: &mut Vec<(&'a str, &'a str)>,
) {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return;
    };
    let Some(Expression::Identifier(target)) = &declarator.init else {
        return;
    };
    aliases.push((id.name.as_str(), target.name.as_str()));
}

pub fn decoder_calls_anywhere<'a>(program: &'a Program<'a>, names: &[&str]) -> Vec<Call<'a>> {
    let mut aliases: Vec<(&'a str, &'a str)> = Vec::new();
    let mut calls = Vec::new();
    scan_statements(&program.body, names, &mut aliases, &mut calls);
    calls
}

fn scan_statements<'a>(
    statements: &'a [Statement<'a>],
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    for statement in statements {
        scan_statement(statement, names, aliases, calls);
    }
}

fn scan_statement<'a>(
    statement: &'a Statement<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    match statement {
        Statement::ExpressionStatement(node) => {
            scan_expression(&node.expression, names, aliases, calls);
        }
        Statement::VariableDeclaration(decl) => {
            scan_variable_declaration(decl, names, aliases, calls)
        }
        Statement::ReturnStatement(node) => {
            if let Some(argument) = &node.argument {
                scan_expression(argument, names, aliases, calls);
            }
        }
        Statement::ThrowStatement(node) => scan_expression(&node.argument, names, aliases, calls),
        Statement::BlockStatement(block) => scan_statements(&block.body, names, aliases, calls),
        Statement::FunctionDeclaration(function) => {
            let is_decoder_definition = function
                .id
                .as_ref()
                .is_some_and(|id| names.contains(&id.name.as_str()));
            if !is_decoder_definition {
                if let Some(body) = &function.body {
                    scan_statements(&body.statements, names, aliases, calls);
                }
            }
        }
        Statement::IfStatement(node) => scan_if(node, names, aliases, calls),
        Statement::WhileStatement(node) => scan_loop(&node.test, &node.body, names, aliases, calls),
        Statement::DoWhileStatement(node) => {
            scan_loop(&node.test, &node.body, names, aliases, calls)
        }
        Statement::ForStatement(node) => scan_for(node, names, aliases, calls),
        Statement::TryStatement(node) => scan_try(node, names, aliases, calls),
        Statement::SwitchStatement(node) => scan_switch(node, names, aliases, calls),
        Statement::LabeledStatement(node) => scan_statement(&node.body, names, aliases, calls),
        _ => {}
    }
}

fn scan_variable_declaration<'a>(
    decl: &'a oxc_ast::ast::VariableDeclaration<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    for declarator in &decl.declarations {
        record_alias(declarator, aliases);
        let is_decoder_definition = matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if names.contains(&id.name.as_str()));
        if is_decoder_definition {
            continue;
        }
        if let Some(init) = &declarator.init {
            scan_expression(init, names, aliases, calls);
        }
    }
}

fn scan_if<'a>(
    node: &'a oxc_ast::ast::IfStatement<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    scan_expression(&node.test, names, aliases, calls);
    scan_statement(&node.consequent, names, aliases, calls);
    if let Some(alternate) = &node.alternate {
        scan_statement(alternate, names, aliases, calls);
    }
}

fn scan_loop<'a>(
    test: &'a Expression<'a>,
    body: &'a Statement<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    scan_expression(test, names, aliases, calls);
    scan_statement(body, names, aliases, calls);
}

fn scan_for<'a>(
    node: &'a oxc_ast::ast::ForStatement<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    if let Some(oxc_ast::ast::ForStatementInit::VariableDeclaration(decl)) = &node.init {
        scan_variable_declaration(decl, names, aliases, calls);
    }
    if let Some(test) = &node.test {
        scan_expression(test, names, aliases, calls);
    }
    if let Some(update) = &node.update {
        scan_expression(update, names, aliases, calls);
    }
    scan_statement(&node.body, names, aliases, calls);
}

fn scan_try<'a>(
    node: &'a oxc_ast::ast::TryStatement<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    scan_statements(&node.block.body, names, aliases, calls);
    if let Some(handler) = &node.handler {
        scan_statements(&handler.body.body, names, aliases, calls);
    }
    if let Some(finalizer) = &node.finalizer {
        scan_statements(&finalizer.body, names, aliases, calls);
    }
}

fn scan_switch<'a>(
    node: &'a oxc_ast::ast::SwitchStatement<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    scan_expression(&node.discriminant, names, aliases, calls);
    for case in &node.cases {
        if let Some(test) = &case.test {
            scan_expression(test, names, aliases, calls);
        }
        scan_statements(&case.consequent, names, aliases, calls);
    }
}

fn scan_expression<'a>(
    expr: &'a Expression<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    match expr {
        Expression::CallExpression(call) => scan_call(call, names, aliases, calls),
        Expression::NewExpression(node) => scan_new(node, names, aliases, calls),
        Expression::StaticMemberExpression(node) => {
            scan_expression(&node.object, names, aliases, calls)
        }
        Expression::ComputedMemberExpression(node) => {
            scan_expression(&node.object, names, aliases, calls);
            scan_expression(&node.expression, names, aliases, calls);
        }
        Expression::UnaryExpression(node) => scan_expression(&node.argument, names, aliases, calls),
        Expression::BinaryExpression(node) => {
            scan_expression(&node.left, names, aliases, calls);
            scan_expression(&node.right, names, aliases, calls);
        }
        Expression::LogicalExpression(node) => {
            scan_expression(&node.left, names, aliases, calls);
            scan_expression(&node.right, names, aliases, calls);
        }
        Expression::AssignmentExpression(node) => {
            scan_expression(&node.right, names, aliases, calls)
        }
        Expression::ConditionalExpression(node) => {
            scan_expression(&node.test, names, aliases, calls);
            scan_expression(&node.consequent, names, aliases, calls);
            scan_expression(&node.alternate, names, aliases, calls);
        }
        Expression::SequenceExpression(node) => {
            for item in &node.expressions {
                scan_expression(item, names, aliases, calls);
            }
        }
        Expression::ParenthesizedExpression(node) => {
            scan_expression(&node.expression, names, aliases, calls);
        }
        Expression::FunctionExpression(node) => {
            if let Some(body) = &node.body {
                scan_statements(&body.statements, names, aliases, calls);
            }
        }
        _ => {}
    }
}

fn scan_new<'a>(
    node: &'a oxc_ast::ast::NewExpression<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    scan_expression(&node.callee, names, aliases, calls);
    for argument in &node.arguments {
        if let Some(arg) = argument.as_expression() {
            scan_expression(arg, names, aliases, calls);
        }
    }
}

fn scan_call<'a>(
    call: &'a oxc_ast::ast::CallExpression<'a>,
    names: &[&str],
    aliases: &mut Vec<(&'a str, &'a str)>,
    calls: &mut Vec<Call<'a>>,
) {
    let arguments: Vec<&Expression<'a>> = call
        .arguments
        .iter()
        .filter_map(|argument| argument.as_expression())
        .collect();
    if let Expression::Identifier(id) = &call.callee {
        let evaluable = arguments.len() == call.arguments.len()
            && arguments.iter().all(|argument| is_self_contained(argument));
        if evaluable {
            if let Some(index) = resolve_decoder_index(id.name.as_str(), names, aliases) {
                calls.push(Call {
                    decoder: index,
                    span: call.span(),
                    arguments: arguments.clone(),
                });
            }
        }
    }
    scan_expression(&call.callee, names, aliases, calls);
    for argument in &arguments {
        scan_expression(argument, names, aliases, calls);
    }
}
