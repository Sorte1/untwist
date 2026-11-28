use oxc_ast::ast::{
    ArrayExpressionElement, Expression, IdentifierReference, ObjectPropertyKind, PropertyKind,
    Statement, VariableDeclaration,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::registers::TYPED_ARRAY;
use super::resolved;

pub(super) type Latches = FxHashMap<(SymbolId, String), Vec<u32>>;

pub(super) fn record_latches(
    declaration: &VariableDeclaration<'_>,
    scoping: &Scoping,
    latches: &mut Latches,
) {
    for declarator in &declaration.declarations {
        let (Some(id), Some(Expression::ObjectExpression(object))) = (
            declarator.id.get_binding_identifier(),
            declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses),
        ) else {
            continue;
        };
        let Some(holder) = id.symbol_id.get().filter(|symbol| fixed(*symbol, scoping)) else {
            continue;
        };
        let mut found: Vec<(String, Vec<u32>)> = Vec::new();
        let mut names: FxHashSet<String> = FxHashSet::default();
        let mut plain = true;
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                plain = false;
                break;
            };
            let Some(name) = property
                .key
                .static_name()
                .and_then(|name| name.as_str().map(str::to_string))
            else {
                plain = false;
                break;
            };
            if property.computed || property.kind != PropertyKind::Init || name == "__proto__" {
                plain = false;
                break;
            }
            if let Some(values) = fresh_words(&property.value, scoping) {
                found.push((name.clone(), values));
            }
            if !names.insert(name) {
                plain = false;
                break;
            }
        }
        if plain {
            for (name, values) in found {
                latches.insert((holder, name), values);
            }
        }
    }
}

fn fixed(symbol: SymbolId, scoping: &Scoping) -> bool {
    scoping.symbol_redeclarations(symbol).is_empty()
        && !scoping
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write())
}

fn fresh_words(value: &Expression<'_>, scoping: &Scoping) -> Option<Vec<u32>> {
    let Expression::NewExpression(new) = value.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = &new.callee else {
        return None;
    };
    if callee.name != TYPED_ARRAY || resolved(callee, scoping).is_some() {
        return None;
    }
    let [argument] = new.arguments.as_slice() else {
        return None;
    };
    let Expression::ArrayExpression(array) = argument.as_expression()?.without_parentheses() else {
        return None;
    };
    array
        .elements
        .iter()
        .map(|element| match element {
            ArrayExpressionElement::NumericLiteral(literal) => to_uint32(literal.value),
            _ => None,
        })
        .collect()
}

fn to_uint32(value: f64) -> Option<u32> {
    if !value.is_finite() || value.fract() != 0.0 || value.abs() >= 9_007_199_254_740_992.0 {
        return None;
    }
    Some((value as i64) as u32)
}

pub(super) fn latch_read(
    expr: &Expression<'_>,
    latches: &Latches,
    scoping: &Scoping,
) -> Option<u32> {
    let Expression::ComputedMemberExpression(slot) = expr.without_parentheses() else {
        return None;
    };
    let Expression::NumericLiteral(index) = slot.expression.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(field) = slot.object.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(holder) = &field.object else {
        return None;
    };
    if slot.optional || field.optional || index.value.fract() != 0.0 || index.value < 0.0 {
        return None;
    }
    let symbol = resolved(holder, scoping)?;
    let values = latches.get(&(symbol, field.property.name.to_string()))?;
    values.get(index.value as usize).copied()
}

pub(super) fn is_function_length(
    expr: &Expression<'_>,
    functions: &dyn Fn(SymbolId) -> bool,
    scoping: &Scoping,
) -> bool {
    let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
        return false;
    };
    let Expression::Identifier(function) = &member.object else {
        return false;
    };
    !member.optional
        && member.property.name == "length"
        && resolved(function, scoping)
            .is_some_and(|symbol| functions(symbol) && fixed(symbol, scoping))
}

pub(super) fn length_guard(
    expr: &Expression<'_>,
    functions: &dyn Fn(SymbolId) -> bool,
    scoping: &Scoping,
) -> bool {
    let Expression::BinaryExpression(binary) = expr.without_parentheses() else {
        return false;
    };
    binary.operator == BinaryOperator::GreaterEqualThan
        && matches!(
            binary.right.without_parentheses(),
            Expression::NumericLiteral(zero) if zero.value == 0.0
        )
        && is_function_length(&binary.left, functions, scoping)
}

pub(super) fn all_of(
    expr: &Expression<'_>,
    part: &dyn Fn(&Expression<'_>) -> Option<bool>,
) -> Option<bool> {
    if let Expression::LogicalExpression(logical) = expr.without_parentheses() {
        let left = all_of(&logical.left, part);
        let right = all_of(&logical.right, part);
        return match logical.operator {
            LogicalOperator::And if left == Some(false) || right == Some(false) => Some(false),
            LogicalOperator::And if left == Some(true) && right == Some(true) => Some(true),
            _ => None,
        };
    }
    part(expr)
}

pub(super) fn statement_is_sealed(
    statement: &Statement<'_>,
    digests: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) => true,
        Statement::ExpressionStatement(expression) => {
            matches!(expression.expression, Expression::StringLiteral(_))
        }
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().all(|declarator| {
                let digest = declarator
                    .id
                    .get_binding_identifier()
                    .and_then(|id| id.symbol_id.get())
                    .is_some_and(|symbol| digests.contains(&symbol));
                digest
                    || declarator
                        .init
                        .as_ref()
                        .is_none_or(|init| reaches_no_binding(init, scoping))
            })
        }
        _ => false,
    }
}

fn reaches_no_binding(init: &Expression<'_>, scoping: &Scoping) -> bool {
    let mut scan = Bindings {
        scoping,
        reached: false,
    };
    scan.visit_expression(init);
    !scan.reached
}

struct Bindings<'s> {
    scoping: &'s Scoping,
    reached: bool,
}

impl<'a> VisitJs<'a> for Bindings<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let root = self.scoping.root_scope_id();
        if resolved(it, self.scoping)
            .is_some_and(|symbol| self.scoping.symbol_scope_id(symbol) == root)
        {
            self.reached = true;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.reached {
            walk_js::walk_expression(self, it);
        }
    }
}

pub(super) fn quiet(
    init: &Expression<'_>,
    latches: &Latches,
    functions: &dyn Fn(SymbolId) -> bool,
    scoping: &Scoping,
) -> bool {
    let mut scan = Quiet {
        latches,
        functions,
        scoping,
        loud: false,
    };
    scan.visit_expression(init);
    !scan.loud
}

struct Quiet<'l, 'f, 's> {
    latches: &'l Latches,
    functions: &'f dyn Fn(SymbolId) -> bool,
    scoping: &'s Scoping,
    loud: bool,
}

impl<'a> VisitJs<'a> for Quiet<'_, '_, '_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.loud {
            return;
        }
        match it {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => {}
            Expression::Identifier(id) => {
                if resolved(id, self.scoping).is_none() {
                    self.loud = true;
                }
            }
            Expression::ParenthesizedExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::LogicalExpression(_)
            | Expression::ConditionalExpression(_) => walk_js::walk_expression(self, it),
            Expression::UnaryExpression(unary) if !unary.operator.is_delete() => {
                walk_js::walk_expression(self, it);
            }
            Expression::ComputedMemberExpression(_) => {
                if latch_read(it, self.latches, self.scoping).is_none() {
                    self.loud = true;
                }
            }
            Expression::StaticMemberExpression(_) => {
                if !is_function_length(it, self.functions, self.scoping) {
                    self.loud = true;
                }
            }
            _ => self.loud = true,
        }
    }
}
