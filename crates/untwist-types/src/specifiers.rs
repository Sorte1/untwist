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

struct Collect<'s> {
    found: Vec<Specifier>,
    callees: Vec<u32>,
    scoping: &'s oxc_semantic::Scoping,
}

impl Collect<'_> {
    fn free(&self, ident: &IdentifierReference<'_>) -> bool {
        ident
            .reference_id
            .get()
            .is_none_or(|reference| self.scoping.get_reference(reference).symbol_id().is_none())
    }
}

impl Collect<'_> {
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

impl<'a> Visit<'a> for Collect<'_> {
    fn visit_import_meta(&mut self, it: &ImportMeta) {
        self.found.push(Specifier {
            start: it.span.start,
            end: it.span.end,
            value: None,
        });
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "require" && self.free(it) && !self.callees.contains(&it.span.start) {
            self.found.push(Specifier {
                start: it.span.start,
                end: it.span.end,
                value: None,
            });
        }
    }

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
            Expression::Identifier(id) if id.name == "require" && self.free(id)
        );
        if requires {
            self.callees.push(it.callee.span().start);
            let single = it.arguments.len() == 1;
            match it.arguments.first().and_then(Argument::as_expression) {
                Some(argument) if single => self.expression(argument),
                _ => self.found.push(Specifier {
                    start: it.span.start,
                    end: it.span.end,
                    value: None,
                }),
            }
        }
        walk::walk_call_expression(self, it);
    }
}

pub fn specifiers(code: &str, typescript: bool) -> Option<Vec<Specifier>> {
    let alloc = Allocator::default();
    let program = erase::parse_any(&alloc, code, typescript)?;
    let program = alloc.alloc(program);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut collect = Collect {
        found: Vec::new(),
        callees: Vec::new(),
        scoping: &scoping,
    };
    collect.visit_program(program);
    Some(collect.found)
}
