use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{SourceType, SPAN};
use untwist_core::semantic::build_scoping;

use crate::runtime::synthetic_statements;
use crate::webpack::container::is_module_container;

pub type FreeNames = Vec<(String, bool)>;

pub fn scoped<'a>(alloc: &'a Allocator, statement: Statement<'a>) -> (Statement<'a>, FreeNames) {
    let builder = AstBuilder::new(alloc);
    let mut body = AVec::with_capacity_in(1, &builder);
    body.push(statement);
    let mut program = Program::new(
        SPAN,
        SourceType::cjs(),
        "",
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        body,
        &builder,
    );
    let scoping = build_scoping(&program).scoping;
    let free = scoping
        .root_unresolved_references()
        .iter()
        .map(|(name, references)| {
            let written = references
                .iter()
                .any(|&id| scoping.get_reference(id).is_write());
            (name.to_string(), written)
        })
        .collect();
    let statement = program
        .body
        .pop()
        .unwrap_or_else(|| Statement::new_empty_statement(SPAN, &builder));
    (statement, free)
}

pub fn expression_names<'a>(alloc: &'a Allocator, expr: &Expression<'a>) -> FreeNames {
    let builder = AstBuilder::new(alloc);
    let statement = Statement::new_expression_statement(SPAN, expr.clone_in(alloc), &builder);
    scoped(alloc, statement).1
}

fn without_container<'a>(alloc: &'a Allocator, statement: &Statement<'a>) -> Statement<'a> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return statement.clone_in(alloc);
    };
    let builder = AstBuilder::new(alloc);
    let mut declarations = AVec::with_capacity_in(declaration.declarations.len(), &builder);
    for declarator in &declaration.declarations {
        let init = declarator
            .init
            .as_ref()
            .filter(|init| !is_module_container(init))
            .map(|init| init.clone_in(alloc));
        declarations.push(VariableDeclarator::new(
            declarator.span,
            declarator.id.clone_in(alloc),
            None,
            init,
            declarator.definite,
            &builder,
        ));
    }
    Statement::new_variable_declaration(
        declaration.span,
        declaration.kind,
        declarations,
        declaration.declare,
        &builder,
    )
}

pub fn statement_names<'a>(alloc: &'a Allocator, statement: &Statement<'a>) -> FreeNames {
    scoped(alloc, without_container(alloc, statement)).1
}

pub fn mentions(names: &FreeNames, name: &str) -> bool {
    names.iter().any(|(free, _)| free == name)
}

pub fn writes(names: &FreeNames, name: &str) -> bool {
    names.iter().any(|(free, written)| *written && free == name)
}

pub fn local_unit<'a>(
    alloc: &'a Allocator,
    locals: &[&str],
    inits: Vec<Expression<'a>>,
    helper: Statement<'a>,
) -> Option<Statement<'a>> {
    let text = format!("(() => {{ var {}; }})();", locals.join(", "));
    let mut unit = synthetic_statements(alloc, text)?.pop()?;
    let builder = AstBuilder::new(alloc);
    let Statement::ExpressionStatement(statement) = &mut unit else {
        return None;
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        return None;
    };
    let Expression::ArrowFunctionExpression(arrow) = &mut call.callee else {
        return None;
    };
    let ArrowFunctionBody::FunctionBody(body) = &mut arrow.body else {
        return None;
    };
    let body = &mut body.statements;
    for init in inits {
        body.push(Statement::new_expression_statement(SPAN, init, &builder));
    }
    body.push(helper);
    Some(unit)
}

pub fn closed_value(value: &Expression<'_>) -> bool {
    let function = |expr: &Expression<'_>| {
        matches!(
            expr.without_parentheses(),
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
        )
    };
    match value.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::ConditionalExpression(choice) => {
            let global_member = matches!(
                choice.test.without_parentheses(),
                Expression::StaticMemberExpression(member)
                    if matches!(member.object, Expression::Identifier(_))
            );
            global_member && function(&choice.consequent) && function(&choice.alternate)
        }
        other => function(other),
    }
}
