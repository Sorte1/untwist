use oxc_ast::ast::{
    AssignmentTarget, BindingPattern, ComputedMemberExpression, Expression, Function,
};
use oxc_ast::ast::{IdentifierReference, Statement};
use oxc_ast_visit::{walk_js, VisitJs};

use crate::stack;

struct ParamUsage<'a> {
    names: Vec<Option<&'a str>>,
    used: Vec<bool>,
}

impl<'a> VisitJs<'a> for ParamUsage<'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name.as_str() == "arguments" {
            self.used.iter_mut().for_each(|used| *used = true);
            return;
        }
        let name = it.name.as_str();
        if let Some(index) = self.names.iter().position(|slot| *slot == Some(name)) {
            self.used[index] = true;
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if is_arguments(&it.object) {
            match literal_index(&it.expression) {
                Some(index) if index < self.used.len() => self.used[index] = true,
                _ => self.used.iter_mut().for_each(|used| *used = true),
            }
            self.visit_expression(&it.expression);
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

fn is_arguments(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name.as_str() == "arguments")
}

fn literal_index(expr: &Expression<'_>) -> Option<usize> {
    match expr {
        Expression::NumericLiteral(literal)
            if literal.value >= 0.0 && literal.value.fract() == 0.0 =>
        {
            Some(literal.value as usize)
        }
        _ => None,
    }
}

fn param_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn visit_body<'a>(usage: &mut ParamUsage<'a>, statements: &'a [Statement<'a>]) {
    for statement in statements {
        usage.visit_statement(statement);
    }
}

pub(super) fn used_params(function: &Function<'_>) -> Vec<bool> {
    let names: Vec<Option<&str>> = function
        .params
        .items
        .iter()
        .map(|param| param_name(&param.pattern))
        .collect();
    let mut usage = ParamUsage {
        used: vec![false; names.len()],
        names,
    };
    if let Some(body) = &function.body {
        visit_body(&mut usage, &body.statements);
    }
    usage.used
}
