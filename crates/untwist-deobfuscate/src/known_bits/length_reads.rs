use oxc_ast::ast::{Expression, Program};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashSet;

use super::resolved;

pub(super) fn function_length_reads(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<Span> {
    let mut scan = LengthReads {
        scoping,
        found: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.found
}

struct LengthReads<'s> {
    scoping: &'s Scoping,
    found: FxHashSet<Span>,
}

impl<'a> VisitJs<'a> for LengthReads<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::StaticMemberExpression(member) = it {
            if let Expression::Identifier(object) = &member.object {
                let declared_function = resolved(object, self.scoping).is_some_and(|symbol| {
                    self.scoping.symbol_flags(symbol).is_function()
                        && self.scoping.symbol_redeclarations(symbol).is_empty()
                });
                if member.property.name == "length" && declared_function {
                    self.found.insert(object.span);
                }
            }
        }
        walk_js::walk_expression(self, it);
    }
}
