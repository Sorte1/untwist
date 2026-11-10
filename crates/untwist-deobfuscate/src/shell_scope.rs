use oxc_allocator::{Allocator, Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{Span, SPAN};
use oxc_syntax::scope::ScopeFlags;

const EVAL: &str = "eval";

#[derive(Default)]
struct OwnLevelBindings {
    found: bool,
}

impl<'a> VisitJs<'a> for OwnLevelBindings {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if it.r#type == FunctionType::FunctionDeclaration {
            self.found = true;
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            self.found = true;
            return;
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if matches!(it.callee.without_parentheses(), Expression::Identifier(id) if id.name == EVAL)
        {
            self.found = true;
            return;
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn binds_at_top_level(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::VariableDeclaration(_)
            | Statement::FunctionDeclaration(_)
            | Statement::ClassDeclaration(_)
    )
}

pub fn binds_at_own_level(program: &Program<'_>) -> bool {
    let mut scan = OwnLevelBindings::default();
    for statement in &program.body {
        if binds_at_top_level(statement) {
            return true;
        }
        scan.visit_statement(statement);
        if scan.found {
            return true;
        }
    }
    false
}

fn called_function<'a>(
    span: Span,
    body: ABox<'a, FunctionBody<'a>>,
    builder: &AstBuilder<'a>,
) -> Statement<'a> {
    let params = FormalParameters::boxed(
        span,
        FormalParameterKind::FormalParameter,
        AVec::new_in(builder),
        None,
        builder,
    );
    let function = Expression::new_function_expression(
        SPAN,
        FunctionType::FunctionExpression,
        None,
        false,
        false,
        false,
        None,
        None,
        params,
        None,
        Some(body),
        builder,
    );
    let callee = Expression::new_parenthesized_expression(SPAN, function, builder);
    let call =
        Expression::new_call_expression(span, callee, None, AVec::new_in(builder), false, builder);
    Statement::new_expression_statement(span, call, builder)
}

pub fn keep_body_scoped<'a>(program: &mut Program<'a>, always: bool, alloc: &'a Allocator) -> bool {
    if crate::mutants::shell_bodies_lifted() || (!always && !binds_at_own_level(program)) {
        return false;
    }
    let builder = AstBuilder::new(alloc);
    let span = program.span;
    let statements = std::mem::replace(&mut program.body, AVec::new_in(&builder));
    let directives = std::mem::replace(&mut program.directives, AVec::new_in(&builder));
    let body = FunctionBody::boxed(span, directives, statements, &builder);
    program.body.push(called_function(span, body, &builder));
    true
}
