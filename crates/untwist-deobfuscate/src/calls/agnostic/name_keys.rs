use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;

use super::bound_once::Stable;
use crate::value_flow::{flow_rule_disabled, INERT_NAME_KEYS};

pub(super) fn names_observed() -> bool {
    !flow_rule_disabled(INERT_NAME_KEYS)
}

pub(super) fn loop_key(
    scoping: &Scoping,
    stable: &Stable<'_>,
    left: &ForStatementLeft<'_>,
) -> Option<SymbolId> {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let (BindingPattern::BindingIdentifier(binding), None) = (&declarator.id, &declarator.init)
    else {
        return None;
    };
    let symbol = binding.symbol_id.get()?;
    let written = scoping
        .get_resolved_references(symbol)
        .any(|reference| reference.is_write());
    (!written && stable.holds(symbol)).then_some(symbol)
}

pub(super) fn names_only(scoping: &Scoping, stable: &Stable<'_>, it: &ForInStatement<'_>) -> bool {
    let Some(key) = loop_key(scoping, stable, &it.left) else {
        return false;
    };
    let mut uses = Uses {
        scoping,
        key,
        bad: false,
        seen: 0,
        sink: false,
    };
    uses.visit_statement(&it.body);
    let outside = scoping.get_resolved_references(key).count() != uses.seen;
    !uses.bad && !outside
}

struct Uses<'s> {
    scoping: &'s Scoping,
    key: SymbolId,
    bad: bool,
    seen: usize,
    sink: bool,
}

impl Uses<'_> {
    fn is_key(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        symbol == Some(self.key)
    }

    fn sunk(&mut self, expr: &Expression<'_>, sink: bool) {
        let outer = std::mem::replace(&mut self.sink, sink);
        self.visit_expression(expr);
        self.sink = outer;
    }

    fn method_call(&mut self, it: &CallExpression<'_>) -> bool {
        let callee = it.callee.without_parentheses().as_member_expression();
        let Some(member) = callee.filter(|member| self.is_key(member.object())) else {
            return false;
        };
        self.seen += 1;
        self.bad |= !self.sink;
        if let MemberExpression::ComputedMemberExpression(computed) = member {
            self.sunk(&computed.expression, false);
        }
        let outer = std::mem::replace(&mut self.sink, false);
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
        self.sink = outer;
        true
    }
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol == Some(self.key) {
            self.seen += 1;
            self.bad |= !self.sink;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        match it {
            Expression::Identifier(id) => self.visit_identifier_reference(id),
            Expression::ParenthesizedExpression(inner) => self.visit_expression(&inner.expression),
            Expression::LogicalExpression(logical) => {
                self.visit_expression(&logical.left);
                self.visit_expression(&logical.right);
            }
            Expression::ConditionalExpression(conditional) => {
                self.sunk(&conditional.test, true);
                self.visit_expression(&conditional.consequent);
                self.visit_expression(&conditional.alternate);
            }
            Expression::BinaryExpression(binary) => {
                let sink = binary.operator != BinaryOperator::Addition;
                self.sunk(&binary.left, sink);
                self.sunk(&binary.right, sink);
            }
            Expression::UnaryExpression(unary) if unary.operator != UnaryOperator::Delete => {
                self.sunk(&unary.argument, true);
            }
            Expression::StaticMemberExpression(member)
                if member.property.name == "length" && self.is_key(&member.object) =>
            {
                self.seen += 1;
            }
            Expression::CallExpression(call) if self.method_call(call) => {}
            other => {
                let outer = std::mem::replace(&mut self.sink, false);
                walk_js::walk_expression(self, other);
                self.sink = outer;
            }
        }
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.sunk(&it.test, true);
        self.visit_statement(&it.consequent);
        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.sunk(&it.test, true);
        self.visit_statement(&it.body);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.visit_statement(&it.body);
        self.sunk(&it.test, true);
    }
}
