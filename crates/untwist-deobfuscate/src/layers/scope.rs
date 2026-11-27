use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_ast_visit::Visit;
use oxc_semantic::SemanticBuilder;
use untwist_core::parse::{has_error, parse_with, SourceType};
use untwist_core::semantic::program_is_strict;

use super::eval::var_names;
use super::loader::Syntax;
use super::observe::observes_globals;

pub const MODULE_NAMES: [&str; 5] = ["require", "module", "exports", "__filename", "__dirname"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Runs {
    InFunction,
    IndirectEval,
}

pub fn reaches_module_scope(text: &str, outer: &[String], runs: Runs) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return true;
    }
    let program = &parsed.program;
    let mut free = Syntax::default();
    free.visit_program(program);
    if free.this || free.arguments {
        return true;
    }
    let built = SemanticBuilder::new().build(program);
    let module_bound = built
        .semantic
        .scoping()
        .root_unresolved_references()
        .keys()
        .any(|name| {
            let name = name.as_str();
            MODULE_NAMES.contains(&name) || outer.iter().any(|outer| outer == name)
        });
    module_bound
        || runs == Runs::IndirectEval
            && !program_is_strict(program)
            && !var_names(program).is_empty()
            && observes_globals(program)
}

pub fn called_function(text: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let [Statement::ExpressionStatement(only)] = parsed.program.body.as_slice() else {
        return None;
    };
    let Expression::CallExpression(call) = only.expression.without_parentheses() else {
        return None;
    };
    let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
        return None;
    };
    let span = function.span;
    text.get(span.start as usize..span.end as usize)
        .map(|function| format!("({function})"))
}

pub fn is_direct_eval(callee: &Expression<'_>) -> bool {
    matches!(callee.without_parentheses(), Expression::Identifier(id) if id.name == "eval")
}

pub fn declared_by(statements: &[Statement<'_>]) -> Vec<String> {
    let mut syntax = Syntax::default();
    for statement in statements {
        syntax.visit_statement(statement);
    }
    syntax.bindings
}
