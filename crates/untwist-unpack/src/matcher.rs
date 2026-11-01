use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_syntax::operator::UnaryOperator;
use untwist_core::ast_util::prop::{identifier_name, member_parts, number_key};

#[derive(Clone, Copy)]
pub struct Factory<'k, 'a> {
    pub params: &'k FormalParameters<'a>,
    pub body: &'k FunctionBody<'a>,
}

#[inline]
pub fn const_member<'k, 'a>(
    expr: &'k Expression<'a>,
    property: &str,
) -> Option<&'k Expression<'a>> {
    let (object, name) = member_parts(expr)?;
    if name == property {
        Some(object)
    } else {
        None
    }
}

pub fn named_const_member(expr: &Expression<'_>, object: &str, property: &str) -> bool {
    const_member(expr, property).and_then(identifier_name) == Some(object)
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

pub fn argument_expression<'k, 'a>(argument: &'k Argument<'a>) -> Option<&'k Expression<'a>> {
    match argument {
        Argument::SpreadElement(_) => None,
        other => other.as_expression(),
    }
}

pub fn literal_id(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::NumericLiteral(literal) => Some(number_key(literal.value)),
        Expression::StringLiteral(literal) if !literal.lone_surrogates => {
            Some(literal.value.as_str().to_string())
        }
        _ => None,
    }
}

pub fn anonymous_factory<'k, 'a>(expr: &'k Expression<'a>) -> Option<Factory<'k, 'a>> {
    match expr {
        Expression::FunctionExpression(function) => {
            if function.id.is_some() || function.generator {
                return None;
            }
            Some(Factory {
                params: &function.params,
                body: function.body.as_deref()?,
            })
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return None;
            };
            Some(Factory {
                params: &arrow.params,
                body,
            })
        }
        _ => None,
    }
}

pub fn expression_statement<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match stmt {
        Statement::ExpressionStatement(stmt) => Some(&stmt.expression),
        _ => None,
    }
}

pub fn unwrap_statement_expression<'k, 'a>(expr: &'k Expression<'a>) -> &'k Expression<'a> {
    match expr {
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::LogicalNot
                    | UnaryOperator::Void
                    | UnaryOperator::UnaryPlus
                    | UnaryOperator::UnaryNegation
                    | UnaryOperator::BitwiseNot
            ) =>
        {
            unwrap_statement_expression(&unary.argument)
        }
        other => other,
    }
}

pub fn single_param_name<'k>(params: &'k FormalParameters<'_>) -> Option<&'k str> {
    if params.rest.is_some() || params.items.len() != 1 {
        return None;
    }
    let BindingPattern::BindingIdentifier(id) = &params.items.first()?.pattern else {
        return None;
    };
    Some(id.name.as_str())
}

pub fn param_names<'a>(params: &FormalParameters<'a>) -> Vec<&'a str> {
    let mut names = Vec::with_capacity(params.items.len());
    for item in &params.items {
        let BindingPattern::BindingIdentifier(id) = &item.pattern else {
            break;
        };
        names.push(id.name.as_str());
    }
    names
}

pub fn unreassigned_var(kind: VariableDeclarationKind) -> bool {
    matches!(
        kind,
        VariableDeclarationKind::Var | VariableDeclarationKind::Const
    )
}
