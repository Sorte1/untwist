use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;

use crate::erase;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Specifier {
    pub start: u32,
    pub end: u32,
    pub value: Option<String>,
}

struct Collect {
    found: Vec<Specifier>,
}

impl Collect {
    fn literal(&mut self, literal: &StringLiteral<'_>) {
        self.found.push(Specifier {
            start: literal.span.start,
            end: literal.span.end,
            value: literal.value.as_str().map(str::to_string),
        });
    }

    fn expression(&mut self, expression: &Expression<'_>) {
        match expression.without_parentheses() {
            Expression::StringLiteral(literal) => self.literal(literal),
            other => {
                let span = other.span();
                self.found.push(Specifier {
                    start: span.start,
                    end: span.end,
                    value: None,
                });
            }
        }
    }
}

impl<'a> Visit<'a> for Collect {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.literal(&it.source);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.literal(&it.source);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.literal(&it.source);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.expression(&it.source);
        walk::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let requires = matches!(
            it.callee.without_parentheses(),
            Expression::Identifier(id) if id.name == "require"
        );
        if requires {
            match it.arguments.first().and_then(Argument::as_expression) {
                Some(argument) => self.expression(argument),
                None => self.found.push(Specifier {
                    start: it.span.start,
                    end: it.span.end,
                    value: None,
                }),
            }
        }
        walk::walk_call_expression(self, it);
    }
}

pub fn specifiers(code: &str) -> Option<Vec<Specifier>> {
    let alloc = Allocator::default();
    let (program, _) = erase::parse_js(&alloc, code)?;
    let mut collect = Collect { found: Vec::new() };
    collect.visit_program(&program);
    Some(collect.found)
}
