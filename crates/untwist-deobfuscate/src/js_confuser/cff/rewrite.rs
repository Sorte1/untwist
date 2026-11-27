use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::naming_slot::unnamed;
use untwist_core::ast_util::respan::fresh_span;
use untwist_eval::fold::{const_to_expression, Const};

use super::build::{member_parts, target_parts, Build};
use super::value::{symbol_at, Val};

pub(super) struct Rewriter<'r, 'b, 'k, 'a> {
    pub(super) build: &'r mut Build<'b, 'k, 'a>,
    pub(super) values: &'r [f64],
    pub(super) rest: Option<(&'r str, SymbolId)>,
}

impl<'a> Rewriter<'_, '_, '_, 'a> {
    fn literal(&self, expr: &Expression<'a>) -> Option<Expression<'a>> {
        let value = match self.build.env(self.values).eval(expr) {
            Some((Val::Num(n), true)) => Const::Num(n),
            Some((Val::Bool(b), true)) => Const::Bool(b),
            _ => Const::Str(self.build.xor(expr, self.values)?),
        };
        const_to_expression(&value, expr.span(), self.build.builder)
    }

    fn lifted(&mut self, pair: (String, String)) -> Option<(Span, String)> {
        let found = self.build.reference(&pair.0, &pair.1);
        self.build.failed |= found.is_none();
        found
    }

    fn forbidden(&self, symbol: SymbolId) -> bool {
        let syms = &self.build.plan.syms;
        let rest_ok = self.rest.is_some();
        symbol == syms.main
            || symbol == syms.states
            || symbol == syms.scope
            || symbol == syms.runtime
            || symbol == syms.sum
            || Some(symbol) == syms.slice
            || Some(symbol) == syms.xor
            || (Some(symbol) == syms.arg && !rest_ok)
            || syms.entry.contains(&symbol)
    }

    fn name_slot(&self, value: &mut Expression<'a>) {
        if self.build.names_observed && value.is_anonymous_function_definition() {
            let taken = std::mem::replace(
                value,
                Expression::new_null_literal(SPAN, self.build.builder),
            );
            *value = unnamed(taken, self.build.builder);
        }
    }
}

impl<'a> VisitMut<'a> for Rewriter<'_, '_, '_, 'a> {
    fn visit_expression(&mut self, expr: &mut Expression<'a>) {
        if self.build.failed {
            return;
        }
        if let Some(literal) = self.literal(expr) {
            *expr = literal;
            return;
        }
        if let Expression::FunctionExpression(function) = expr {
            if self.build.plan.wrappers.contains_key(&function.span) {
                let span = function.span;
                match (self.build.wrapper_body(span), function.body.as_mut()) {
                    (Some(statements), Some(body)) => body.statements = statements,
                    _ => self.build.failed = true,
                }
                if self.build.names_observed {
                    let taken = std::mem::replace(
                        expr,
                        Expression::new_null_literal(SPAN, self.build.builder),
                    );
                    *expr = unnamed(taken, self.build.builder);
                }
                return;
            }
        }
        if let Some(pair) = self.build.scope_pair(expr, self.values) {
            if let Some((span, name)) = self.lifted(pair) {
                *expr = Expression::new_identifier(
                    span,
                    Ident::from_str_in(&name, self.build.builder),
                    self.build.builder,
                );
            }
            return;
        }
        if let (Some((rest, symbol)), Some(found)) = (self.rest, symbol_at(expr, self.build.spans))
        {
            if Some(found) == self.build.plan.syms.arg {
                let Some(span) = fresh_span() else {
                    self.build.failed = true;
                    return;
                };
                self.build
                    .lifted
                    .created
                    .push((span, self.build.scoping.symbol_span(symbol)));
                *expr = Expression::new_identifier(
                    span,
                    Ident::from_str_in(rest, self.build.builder),
                    self.build.builder,
                );
                return;
            }
        }
        let direct_call = match &*expr {
            Expression::CallExpression(call) => {
                self.build.scope_pair(&call.callee, self.values).is_some()
            }
            Expression::TaggedTemplateExpression(tag) => {
                self.build.scope_pair(&tag.tag, self.values).is_some()
            }
            Expression::UnaryExpression(u) => {
                u.operator == oxc_syntax::operator::UnaryOperator::Delete
                    && self.build.scope_pair(&u.argument, self.values).is_some()
            }
            _ => false,
        };
        if direct_call {
            self.build.failed = true;
            return;
        }
        walk_mut::walk_expression(self, expr);
    }

    fn visit_simple_assignment_target(&mut self, target: &mut SimpleAssignmentTarget<'a>) {
        if self.build.failed {
            return;
        }
        let pair = target_parts(target).and_then(|(object, prop)| {
            let (base, scope) = member_parts(object)?;
            (symbol_at(base, self.build.spans) == Some(self.build.plan.syms.scope)).then_some(())?;
            Some((
                self.build.member_key(scope, self.values)?,
                self.build.member_key(prop, self.values)?,
            ))
        });
        if let Some(pair) = pair {
            if let Some((span, name)) = self.lifted(pair) {
                *target = SimpleAssignmentTarget::new_assignment_target_identifier(
                    span,
                    Ident::from_str_in(&name, self.build.builder),
                    self.build.builder,
                );
            }
            return;
        }
        walk_mut::walk_simple_assignment_target(self, target);
    }

    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        walk_mut::walk_call_expression(self, it);
        let lifted = match it.callee.without_parentheses() {
            Expression::SequenceExpression(sequence) => match sequence.expressions.as_slice() {
                [Expression::NumericLiteral(_), Expression::Identifier(id)] => self
                    .build
                    .lifted
                    .created
                    .iter()
                    .any(|(reference, _)| *reference == id.span),
                _ => false,
            },
            _ => false,
        };
        if !lifted {
            return;
        }
        let callee = std::mem::replace(
            &mut it.callee,
            Expression::new_null_literal(SPAN, self.build.builder),
        );
        let inner = match callee {
            Expression::ParenthesizedExpression(parenthesized) => parenthesized.unbox().expression,
            other => other,
        };
        let Expression::SequenceExpression(sequence) = inner else {
            self.build.failed = true;
            return;
        };
        match sequence.unbox().expressions.pop() {
            Some(identifier) => it.callee = identifier,
            None => self.build.failed = true,
        }
    }

    fn visit_assignment_expression(&mut self, it: &mut AssignmentExpression<'a>) {
        walk_mut::walk_assignment_expression(self, it);
        if matches!(it.left, AssignmentTarget::AssignmentTargetIdentifier(_)) {
            self.name_slot(&mut it.right);
        }
    }

    fn visit_assignment_target_with_default(&mut self, it: &mut AssignmentTargetWithDefault<'a>) {
        walk_mut::walk_assignment_target_with_default(self, it);
        if matches!(it.binding, AssignmentTarget::AssignmentTargetIdentifier(_)) {
            self.name_slot(&mut it.init);
        }
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let symbol = self.build.spans.get(&it.span).copied();
        if symbol.is_some_and(|s| self.forbidden(s)) {
            self.build.failed = true;
        }
    }
}
