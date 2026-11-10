use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, SPAN};
use oxc_str::{Ident, Str};
use rustc_hash::FxHashMap;
use untwist_core::semantic::build_scoping;

use super::layout::Id;
use crate::esbuild::mentions::binds_any;
use crate::model::Bundle;
use crate::path::relative_path;
use crate::runtime::synthetic_statements;

const REQUIRE: &str = "require";

struct Rewrite<'a, 'r> {
    alloc: &'a Allocator,
    builder: AstBuilder<'a>,
    scoping: Option<&'r Scoping>,
    context: &'r str,
    paths: &'r FxHashMap<String, String>,
    from: &'r str,
}

impl<'a> Rewrite<'a, '_> {
    fn is_context(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr else {
            return false;
        };
        id.name == self.context && crate::scan::is_free(self.scoping, id)
    }

    fn method<'k>(&self, expr: &'k Expression<'_>) -> Option<&'k str> {
        let Expression::StaticMemberExpression(member) = expr else {
            return None;
        };
        self.is_context(&member.object)
            .then(|| member.property.name.as_str())
    }

    fn identifier(&self, span: oxc_span::Span, name: &str) -> Expression<'a> {
        Expression::new_identifier(span, Ident::from_str_in(name, &self.builder), &self.builder)
    }

    fn require_path(&self, call: &mut CallExpression<'a>) {
        let target = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(Id::of)
            .and_then(|id| self.paths.get(&id.key));
        let Some(path) = target else {
            return;
        };
        let specifier = relative_path(self.from, path);
        call.callee = self.identifier(call.callee.span(), REQUIRE);
        if let Some(argument) = call.arguments.first_mut() {
            let span = argument.span();
            *argument = Argument::from(Expression::new_string_literal(
                span,
                Str::from_str_in(&specifier, &self.builder),
                None,
                &self.builder,
            ));
        }
    }

    fn exported(&self, expr: &mut Expression<'a>) {
        let Expression::CallExpression(call) = expr else {
            return;
        };
        let single = call.arguments.len() == 1 && call.arguments[0].is_expression();
        if self.method(&call.callee) != Some("v") || !single {
            return;
        }
        let Some(mut built) = synthetic_statements(self.alloc, "module.exports = 0;".to_string())
        else {
            return;
        };
        let Some(Statement::ExpressionStatement(mut statement)) = built.pop() else {
            return;
        };
        let Expression::AssignmentExpression(assign) = &mut statement.expression else {
            return;
        };
        let Some(value) = call.arguments.pop().map(Argument::into_expression) else {
            return;
        };
        assign.right = value;
        *expr = std::mem::replace(&mut statement.expression, self.identifier(SPAN, REQUIRE));
    }
}

impl<'a> VisitMut<'a> for Rewrite<'a, '_> {
    fn visit_expression_statement(&mut self, it: &mut ExpressionStatement<'a>) {
        walk_mut::walk_expression_statement(self, it);
        match &mut it.expression {
            Expression::SequenceExpression(sequence) => {
                sequence
                    .expressions
                    .iter_mut()
                    .for_each(|item| self.exported(item));
            }
            other => self.exported(other),
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::CallExpression(call) = it {
            if self.method(&call.callee) == Some("r") {
                return self.require_path(call);
            }
        }
        if self.method(it) == Some("e") {
            *it = self.identifier(it.span(), "exports");
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

pub fn link<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) {
    let paths: FxHashMap<String, String> = bundle
        .modules
        .iter()
        .map(|(id, module)| (id.clone(), module.path.as_str().to_string()))
        .collect();
    let contexts: FxHashMap<String, String> = std::mem::take(&mut bundle.fresh_requires)
        .into_iter()
        .collect();
    for (id, module) in bundle.modules.iter_mut() {
        let Some(context) = contexts.get(id) else {
            continue;
        };
        let from = module.path.as_str().to_string();
        let program = module.program_mut();
        let scoping = binds_any(program, std::iter::once(context.as_str()))
            .then(|| build_scoping(program).scoping);
        let mut rewrite = Rewrite {
            alloc,
            builder: AstBuilder::new(alloc),
            scoping: scoping.as_ref(),
            context,
            paths: &paths,
            from: &from,
        };
        rewrite.visit_program(program);
    }
}
