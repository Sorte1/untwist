use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::Span;

use crate::matcher::{call_parts, const_member};
use crate::model::Module;

struct Injection<'a> {
    span: Span,
    params: Vec<(Span, Ident<'a>)>,
    arguments: Vec<Expression<'a>>,
    body: AVec<'a, Statement<'a>>,
}

fn is_injection(stmt: &Statement<'_>) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(&stmt.expression) else {
        return false;
    };
    if arguments.len() < 2 {
        return false;
    }
    if !arguments
        .iter()
        .skip(1)
        .all(|argument| !matches!(argument, Argument::SpreadElement(_)))
    {
        return false;
    }
    let first_is_receiver = matches!(
        arguments.first().and_then(Argument::as_expression),
        Some(Expression::ThisExpression(_))
    ) || matches!(
        arguments.first().and_then(Argument::as_expression),
        Some(Expression::Identifier(id)) if id.name == "exports"
    );
    if !first_is_receiver {
        return false;
    }
    let Some(object) = const_member(callee, "call") else {
        return false;
    };
    let Expression::FunctionExpression(function) = object else {
        return false;
    };
    function.body.is_some()
        && !function.generator
        && !function.r#async
        && function
            .params
            .items
            .iter()
            .all(|item| matches!(item.pattern, BindingPattern::BindingIdentifier(_)))
        && function.params.rest.is_none()
}

fn injected_params<'a>(params: FormalParameters<'a>) -> Vec<(Span, Ident<'a>)> {
    params
        .items
        .into_iter()
        .filter_map(|item| match item.pattern {
            BindingPattern::BindingIdentifier(id) => {
                let id = id.unbox();
                Some((id.span, id.name))
            }
            _ => None,
        })
        .collect()
}

fn take_injection<'a>(stmt: Statement<'a>) -> Option<Injection<'a>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let stmt = stmt.unbox();
    let span = stmt.span;
    let Expression::CallExpression(call) = stmt.expression else {
        return None;
    };
    let call = call.unbox();
    let mut arguments: Vec<Expression<'a>> = call
        .arguments
        .into_iter()
        .filter_map(|argument| match argument {
            Argument::SpreadElement(_) => None,
            other => Some(other.into_expression()),
        })
        .collect();
    if arguments.is_empty() {
        return None;
    }
    arguments.remove(0);
    let function = match call.callee {
        Expression::StaticMemberExpression(member) => member.unbox().object,
        Expression::ComputedMemberExpression(member) => member.unbox().object,
        _ => return None,
    };
    let Expression::FunctionExpression(function) = function else {
        return None;
    };
    let function = function.unbox();
    let params = injected_params(function.params.unbox());
    let body = function.body?.unbox().statements;
    Some(Injection {
        span,
        params,
        arguments,
        body,
    })
}

fn push_injection<'a>(
    out: &mut AVec<'a, Statement<'a>>,
    injection: Injection<'a>,
    builder: &AstBuilder<'a>,
) {
    let Injection {
        span,
        params,
        arguments,
        body,
    } = injection;
    let mut arguments = arguments.into_iter();
    for (param_span, name) in params {
        let declarator = VariableDeclarator::new(
            param_span,
            BindingPattern::new_binding_identifier(param_span, name, builder),
            None,
            arguments.next(),
            false,
            builder,
        );
        let declarations = AVec::from_iter_in([declarator], builder);
        out.push(Statement::new_variable_declaration(
            span,
            VariableDeclarationKind::Var,
            declarations,
            false,
            builder,
        ));
    }
    out.extend(body);
}

pub fn inline_var_injections<'a>(alloc: &'a Allocator, module: &mut Module<'a>) -> usize {
    let program = module.program_mut();
    if !program.body.iter().any(is_injection) {
        return 0;
    }
    let builder = AstBuilder::new(alloc);
    let old = std::mem::replace(&mut program.body, AVec::new_in(&builder));
    let mut inlined = 0usize;
    for stmt in old {
        if !is_injection(&stmt) {
            program.body.push(stmt);
            continue;
        }
        if let Some(injection) = take_injection(stmt) {
            push_injection(&mut program.body, injection, &builder);
            inlined += 1;
        }
    }
    inlined
}
