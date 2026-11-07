use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::AssignmentOperator;

use super::fresh::Bound;
use super::sites::Sites;

const HOLD_DEPTH: u8 = 6;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Hold {
    No,
    Stored,
    Yes,
}

pub(super) fn primitive_result(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_)
    )
}

fn plain_class(class: &Class<'_>) -> bool {
    class.body.body.iter().all(|element| match element {
        ClassElement::PropertyDefinition(property) => !property.r#static,
        ClassElement::AccessorProperty(_) | ClassElement::StaticBlock(_) => false,
        ClassElement::MethodDefinition(method) => {
            !method.computed
                && matches!(
                    method.kind,
                    MethodDefinitionKind::Method | MethodDefinitionKind::Constructor
                )
        }
        ClassElement::TSIndexSignature(_) => true,
    })
}

#[derive(Default)]
struct MemberTarget {
    found: bool,
}

impl<'a> VisitJs<'a> for MemberTarget {
    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.found |= it.as_member_expression().is_some();
        walk_js::walk_simple_assignment_target(self, it);
    }
}

fn writes_a_member(target: &AssignmentTarget<'_>) -> bool {
    let mut scan = MemberTarget::default();
    scan.visit_assignment_target(target);
    scan.found
}

fn prototype_base(member: &MemberExpression<'_>) -> bool {
    let mut base = member.object().without_parentheses();
    while let Some(inner) = base.as_member_expression() {
        if let MemberExpression::StaticMemberExpression(m) = inner {
            if matches!(m.property.name.as_str(), "prototype" | "__proto__") {
                return true;
            }
        }
        base = inner.object().without_parentheses();
    }
    false
}

fn literal_key(property: &ObjectProperty<'_>) -> bool {
    !property.computed || matches!(property.key, PropertyKey::StringLiteral(_))
}

impl<'a> Sites<'_, 'a> {
    pub(super) fn argument_hold(&self, expr: &Expression<'a>) -> Hold {
        match expr.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                Hold::Stored
            }
            Expression::ClassExpression(class) if plain_class(class) => Hold::Stored,
            Expression::Identifier(id) if self.substituted(id).is_none() => {
                self.binding_hold(expr, id, HOLD_DEPTH, true)
            }
            other => self.hold_at(other, HOLD_DEPTH),
        }
    }

    pub(super) fn stored_hold(&self, expr: &Expression<'a>) -> Hold {
        if self.maybe_global(expr) {
            return Hold::Yes;
        }
        self.argument_hold(expr)
    }

    fn hold_at(&self, expr: &Expression<'a>, depth: u8) -> Hold {
        let Some(next) = depth.checked_sub(1) else {
            return Hold::Yes;
        };
        let inner = expr.without_parentheses();
        if primitive_result(inner)
            || self.collection_primitive(inner)
            || self.awaited_array_primitive(inner)
        {
            return Hold::No;
        }
        match inner {
            Expression::ObjectExpression(object) => {
                Hold::Stored.max(self.object_hold(object, next))
            }
            Expression::ArrayExpression(array) => Hold::Stored.max(self.array_hold(array, next)),
            Expression::RegExpLiteral(_) => Hold::Stored,
            Expression::ConditionalExpression(cond) => self
                .hold_at(&cond.consequent, next)
                .max(self.hold_at(&cond.alternate, next)),
            Expression::LogicalExpression(logical) => self
                .hold_at(&logical.left, next)
                .max(self.hold_at(&logical.right, next)),
            Expression::SequenceExpression(seq) => seq
                .expressions
                .last()
                .map_or(Hold::Yes, |last| self.hold_at(last, next)),
            Expression::AssignmentExpression(assign) => match assign.operator {
                AssignmentOperator::Assign => self.hold_at(&assign.right, next),
                operator if operator.is_arithmetic() || operator.is_bitwise() => Hold::No,
                _ => Hold::Yes,
            },
            Expression::Identifier(id) => match self.substituted(id) {
                Some(arg) => self.hold_at(arg, next),
                None => self.binding_hold(inner, id, next, false),
            },
            Expression::CallExpression(call) => self.call_hold(call, next),
            _ if self.string_valued(inner) => Hold::No,
            _ => Hold::Yes,
        }
    }

    fn string_valued(&self, expr: &Expression<'a>) -> bool {
        if let Expression::StaticMemberExpression(member) = expr {
            if member.property.name == "length" && self.fresh.holds_fresh_sequence(&member.object) {
                return true;
            }
        }
        self.watched_element(expr) || self.str_value(expr, super::roles::KEY_DEPTH).is_some()
    }

    fn call_hold(&self, call: &CallExpression<'a>, depth: u8) -> Hold {
        if self.resolve.is_some_and(|map| map.contains_key(&call.span)) {
            return Hold::No;
        }
        let Some(func) = self.resolve_called_function(&call.callee) else {
            return Hold::Yes;
        };
        let Some(exprs) = self.returned_exprs(func) else {
            return Hold::Yes;
        };
        self.with_params(func, call, || {
            exprs
                .iter()
                .map(|expr| self.returned_hold(expr, depth))
                .max()
                .unwrap_or(Hold::No)
        })
        .unwrap_or(Hold::Yes)
    }

    fn returned_hold(&self, expr: &Expression<'a>, depth: u8) -> Hold {
        match expr.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                Hold::Stored
            }
            Expression::Identifier(id) if self.substituted(id).is_none() => {
                self.binding_hold(expr, id, depth, true)
            }
            other => self.hold_at(other, depth),
        }
    }

    fn object_hold(&self, object: &ObjectExpression<'a>, depth: u8) -> Hold {
        let mut hold = Hold::No;
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(p) = property else {
                return Hold::Yes;
            };
            if p.kind != PropertyKind::Init || !literal_key(p) {
                return Hold::Yes;
            }
            hold = hold.max(self.hold_at(&p.value, depth));
        }
        hold
    }

    fn array_hold(&self, array: &ArrayExpression<'a>, depth: u8) -> Hold {
        let mut hold = Hold::No;
        for element in &array.elements {
            let value = match element {
                ArrayExpressionElement::Elision(_) => continue,
                ArrayExpressionElement::SpreadElement(spread) => {
                    if self.maybe_global(&spread.argument) {
                        return Hold::Yes;
                    }
                    &spread.argument
                }
                other => match other.as_expression() {
                    Some(value) => value,
                    None => return Hold::Yes,
                },
            };
            hold = hold.max(self.hold_at(value, depth));
        }
        hold
    }

    fn binding_hold(
        &self,
        expr: &Expression<'a>,
        id: &IdentifierReference<'a>,
        depth: u8,
        function_ok: bool,
    ) -> Hold {
        if self.fresh.symbol(id).is_none() {
            return if matches!(id.name.as_str(), "undefined" | "NaN" | "Infinity") {
                Hold::No
            } else {
                Hold::Yes
            };
        }
        if self.string_valued(expr) {
            return Hold::No;
        }
        match self.fresh.bound_value(id) {
            Some(Bound::Primitive) => Hold::No,
            Some(Bound::Function) if function_ok => Hold::Stored,
            Some(Bound::Container(init)) => {
                Hold::Stored.max(self.hold_at(init, depth.saturating_sub(1)))
            }
            Some(Bound::Function) | None => Hold::Yes,
        }
    }
}

impl<'a> Sites<'_, 'a> {
    pub(super) fn note_store(&mut self, it: &AssignmentExpression<'a>) {
        if !self.watch.observe || self.out.stores_global || self.in_machinery(it.span) {
            return;
        }
        if it.operator.is_arithmetic() || it.operator.is_bitwise() {
            return;
        }
        let stores = match it.left.as_member_expression() {
            Some(member) => {
                prototype_base(member)
                    || (unknown_key(member) && !primitive_result(it.right.without_parentheses()))
                    || self.stored_hold(&it.right) == Hold::Yes
            }
            None => writes_a_member(&it.left) && self.stored_hold(&it.right) != Hold::No,
        };
        self.out.stores_global |= stores;
    }

    pub(super) fn note_class_stores(&mut self, it: &ClassBody<'a>) {
        if !self.watch.observe || self.out.stores_global {
            return;
        }
        let stores = it.body.iter().any(|element| match element {
            ClassElement::PropertyDefinition(property) => property
                .value
                .as_ref()
                .is_some_and(|value| self.stored_hold(value) == Hold::Yes),
            ClassElement::AccessorProperty(property) => property
                .value
                .as_ref()
                .is_some_and(|value| self.stored_hold(value) == Hold::Yes),
            _ => false,
        });
        self.out.stores_global |= stores;
    }
}

fn unknown_key(member: &MemberExpression<'_>) -> bool {
    match member {
        MemberExpression::ComputedMemberExpression(computed) => !matches!(
            computed.expression.without_parentheses(),
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        ),
        _ => false,
    }
}
