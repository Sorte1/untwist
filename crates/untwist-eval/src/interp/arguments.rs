use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, AssignmentTarget, BindingPattern, Expression,
    Function, IdentifierReference, Statement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use crate::stack;

const ARGUMENTS: &str = "arguments";

pub(crate) fn needs_arguments_object(function: &Function<'_>) -> bool {
    let shadowed = function.params.items.iter().any(|param| {
        matches!(&param.pattern, BindingPattern::BindingIdentifier(id) if id.name == ARGUMENTS)
    });
    if shadowed {
        return false;
    }
    let mut scan = ArgumentsScan { found: false };
    scan.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        scan.visit_function_body(body);
    }
    scan.found
}

struct ArgumentsScan {
    found: bool,
}

fn is_plain_literal(element: &ArrayExpressionElement<'_>) -> bool {
    matches!(
        element,
        ArrayExpressionElement::StringLiteral(_)
            | ArrayExpressionElement::NumericLiteral(_)
            | ArrayExpressionElement::BooleanLiteral(_)
            | ArrayExpressionElement::NullLiteral(_)
            | ArrayExpressionElement::Elision(_)
    )
}

impl<'a> VisitJs<'a> for ArgumentsScan {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.found {
            stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        for element in &it.elements {
            if self.found {
                return;
            }
            if !is_plain_literal(element) {
                self.visit_array_expression_element(element);
            }
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == ARGUMENTS {
            self.found = true;
        }
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
}
