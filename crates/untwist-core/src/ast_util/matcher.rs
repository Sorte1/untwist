use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_syntax::operator::UnaryOperator;

use super::literal::is_truthy_literal;

pub struct FnDecl<'k, 'a> {
    pub name: &'k str,
    pub function: &'k Function<'a>,
}

pub fn is_plain_declaration(kind: VariableDeclarationKind) -> bool {
    matches!(
        kind,
        VariableDeclarationKind::Var
            | VariableDeclarationKind::Let
            | VariableDeclarationKind::Const
    )
}

pub fn var_function_or_declaration<'k, 'a>(stmt: &'k Statement<'a>) -> Option<FnDecl<'k, 'a>> {
    match stmt {
        Statement::FunctionDeclaration(function) => {
            let id = function.id.as_ref()?;
            Some(FnDecl {
                name: id.name.as_str(),
                function,
            })
        }
        Statement::VariableDeclaration(declaration) => {
            if !is_plain_declaration(declaration.kind) {
                return None;
            }
            if declaration.declarations.len() != 1 {
                return None;
            }
            let declarator = declaration.declarations.first()?;
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let Some(Expression::FunctionExpression(function)) = &declarator.init else {
                return None;
            };
            Some(FnDecl {
                name: id.name.as_str(),
                function,
            })
        }
        _ => None,
    }
}

pub fn anonymous_function<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    let Expression::FunctionExpression(function) = expr else {
        return None;
    };
    if function.id.is_some() {
        return None;
    }
    Some(function)
}

pub fn call_parts<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, &'k AVec<'a, Argument<'a>>)> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    if call.optional {
        return None;
    }
    Some((&call.callee, &call.arguments))
}

pub fn iife<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Function<'a>, &'k AVec<'a, Argument<'a>>)> {
    let (callee, arguments) = call_parts(expr)?;
    Some((anonymous_function(callee)?, arguments))
}

pub fn statement_iife<'k, 'a>(
    stmt: &'k Statement<'a>,
) -> Option<(&'k Function<'a>, &'k AVec<'a, Argument<'a>>)> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    match &stmt.expression {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            iife(&unary.argument)
        }
        other => iife(other),
    }
}

pub fn infinite_loop_body<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Statement<'a>> {
    match stmt {
        Statement::WhileStatement(node) if is_truthy_literal(&node.test) => Some(&node.body),
        Statement::ForStatement(node) => match &node.test {
            None => Some(&node.body),
            Some(test) if is_truthy_literal(test) => Some(&node.body),
            Some(_) => None,
        },
        _ => None,
    }
}

pub fn is_literal_argument(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::NumericLiteral(_) | Expression::StringLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::UnaryNegation && is_literal_argument(&unary.argument)
        }
        Expression::BinaryExpression(binary) => {
            is_literal_argument(&binary.left) && is_literal_argument(&binary.right)
        }
        _ => false,
    }
}
