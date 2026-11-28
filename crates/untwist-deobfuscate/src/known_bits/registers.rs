use oxc_ast::ast::{
    Argument, AssignmentExpression, AssignmentTarget, ComputedMemberExpression, Expression,
    Function, IdentifierReference, SimpleAssignmentTarget, UnaryExpression, VariableDeclarator,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::resolved;

pub(super) const TYPED_ARRAY: &str = "Uint32Array";

const MAX_LENGTH: usize = 16;

pub(super) fn typed_registers(function: &Function<'_>, scoping: &Scoping) -> Option<Vec<usize>> {
    let mut scan = Registers {
        scoping,
        lengths: FxHashMap::default(),
        order: Vec::new(),
        broken: false,
    };
    scan.visit_function(function, oxc_syntax::scope::ScopeFlags::Function);
    (!scan.broken).then_some(scan.order)
}

pub(super) fn as_plain_arrays(text: &str, lengths: &[usize]) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    for length in lengths {
        let needle = format!("new {TYPED_ARRAY}({length})");
        let at = rest.find(&needle)?;
        out.push_str(&rest[..at]);
        out.push('[');
        out.push_str(&vec!["0"; *length].join(", "));
        out.push(']');
        rest = &rest[at + needle.len()..];
    }
    if rest.contains(TYPED_ARRAY) {
        return None;
    }
    out.push_str(rest);
    Some(out)
}

struct Registers<'s> {
    scoping: &'s Scoping,
    lengths: FxHashMap<SymbolId, usize>,
    order: Vec<usize>,
    broken: bool,
}

impl Registers<'_> {
    fn register(&self, expr: &Expression<'_>) -> Option<usize> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        self.lengths.get(&resolved(id, self.scoping)?).copied()
    }

    fn slot(&self, member: &ComputedMemberExpression<'_>) -> Option<()> {
        let length = self.register(&member.object)?;
        let Expression::NumericLiteral(index) = member.expression.without_parentheses() else {
            return None;
        };
        let fits = index.value.fract() == 0.0 && index.value >= 0.0 && index.value < length as f64;
        fits.then_some(())
    }

    fn fresh_length(&self, init: &Expression<'_>) -> Option<usize> {
        let Expression::NewExpression(new) = init.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(callee) = &new.callee else {
            return None;
        };
        if callee.name != TYPED_ARRAY || resolved(callee, self.scoping).is_some() {
            return None;
        }
        let [Argument::NumericLiteral(length)] = new.arguments.as_slice() else {
            return None;
        };
        let value = length.value;
        let whole = value.fract() == 0.0 && value >= 1.0 && value <= MAX_LENGTH as f64;
        whole.then_some(value as usize)
    }
}

fn is_unsigned(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::BinaryExpression(binary)
            if binary.operator == BinaryOperator::ShiftRightZeroFill
                && matches!(binary.right.without_parentheses(), Expression::NumericLiteral(zero) if zero.value == 0.0)
    )
}

impl<'a> VisitJs<'a> for Registers<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let length = it.init.as_ref().and_then(|init| self.fresh_length(init));
        let symbol = it
            .id
            .get_binding_identifier()
            .and_then(|id| id.symbol_id.get());
        if let (Some(length), Some(symbol)) = (length, symbol) {
            let fixed = !self
                .scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write());
            if !fixed {
                self.broken = true;
            }
            self.lengths.insert(symbol, length);
            self.order.push(length);
            return;
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::ComputedMemberExpression(member) = &it.left {
            if self.register(&member.object).is_some() {
                let stored = it.operator == AssignmentOperator::Assign
                    && self.slot(member).is_some()
                    && is_unsigned(&it.right);
                if !stored {
                    self.broken = true;
                }
                self.visit_expression(&it.right);
                return;
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::ComputedMemberExpression(member) = it {
            if self.register(&member.object).is_some() {
                self.broken = true;
                return;
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Expression::ComputedMemberExpression(member) = it.argument.without_parentheses()
            {
                if self.register(&member.object).is_some() {
                    self.broken = true;
                    return;
                }
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.register(&it.object).is_some() {
            if self.slot(it).is_none() {
                self.broken = true;
            }
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let bare =
            resolved(it, self.scoping).is_some_and(|symbol| self.lengths.contains_key(&symbol));
        if bare || (it.name == TYPED_ARRAY && resolved(it, self.scoping).is_none()) {
            self.broken = true;
        }
    }
}
