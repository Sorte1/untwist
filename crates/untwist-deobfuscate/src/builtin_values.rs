use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::bindings::resolved_symbol;
use crate::proof::GLOBAL_OBJECTS;

struct Exposure<'k> {
    roots: &'k [&'k str],
    scoping: &'k Scoping,
    found: bool,
}

fn property_name<'k>(member: &'k MemberExpression<'_>) -> Option<&'k str> {
    match member {
        MemberExpression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(member) => {
            match member.expression.without_parentheses() {
                Expression::StringLiteral(key) => key.value.as_str(),
                _ => None,
            }
        }
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

impl Exposure<'_> {
    fn is_global(&self, id: &IdentifierReference<'_>, names: &[&str]) -> bool {
        names.contains(&id.name.as_str()) && resolved_symbol(id, self.scoping).is_none()
    }

    fn is_root(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.is_global(id, self.roots),
            other => other.as_member_expression().is_some_and(|member| {
                let global_object = match member.object().without_parentheses() {
                    Expression::Identifier(id) => self.is_global(id, GLOBAL_OBJECTS),
                    _ => false,
                };
                global_object
                    && property_name(member).is_some_and(|name| self.roots.contains(&name))
            }),
        }
    }

    fn is_prototype(&self, expr: &Expression<'_>) -> bool {
        expr.without_parentheses()
            .as_member_expression()
            .is_some_and(|member| {
                property_name(member) == Some("prototype") && self.is_root(member.object())
            })
    }

    fn is_exposable(&self, expr: &Expression<'_>) -> bool {
        self.is_root(expr) || self.is_prototype(expr)
    }

    fn visit_unless_root(&mut self, expr: &Expression<'_>) {
        if !self.is_root(expr) {
            self.visit_expression_of(expr);
        }
    }

    fn visit_expression_of(&mut self, expr: &Expression<'_>) {
        if self.is_exposable(expr) {
            self.found = true;
            return;
        }
        walk_js::walk_expression(self, expr);
    }
}

impl<'a> VisitJs<'a> for Exposure<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.found {
            return;
        }
        self.visit_expression_of(it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if self.is_exposable(it.object()) {
            if let MemberExpression::ComputedMemberExpression(member) = it {
                self.visit_expression(&member.expression);
            }
            return;
        }
        walk_js::walk_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.visit_unless_root(&it.callee);
        self.visit_arguments(&it.arguments);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.visit_unless_root(&it.callee);
        self.visit_arguments(&it.arguments);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::Typeof => self.visit_unless_root(&it.argument),
            _ => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let compares = it.operator.is_equality() || it.operator == BinaryOperator::Instanceof;
        if !compares {
            walk_js::walk_binary_expression(self, it);
            return;
        }
        self.visit_unless_root(&it.left);
        self.visit_unless_root(&it.right);
    }
}

pub fn exposes_a_builtin(program: &Program<'_>, scoping: &Scoping, roots: &[&str]) -> bool {
    let mut exposure = Exposure {
        roots,
        scoping,
        found: false,
    };
    exposure.visit_program(program);
    exposure.found
}
