use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::global::is_global_reference;

use crate::bindings::resolved_symbol;

const CONSTANT_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];

#[derive(Default)]
struct LexicalBindings {
    found: Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for LexicalBindings {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found.extend(it.symbol_id.get());
    }
}

fn lexical_bindings(stmt: &Statement<'_>) -> Vec<SymbolId> {
    let mut bindings = LexicalBindings::default();
    match stmt {
        Statement::VariableDeclaration(declaration) => {
            for declarator in &declaration.declarations {
                bindings.visit_binding_pattern(&declarator.id);
            }
        }
        Statement::ClassDeclaration(class) => {
            if let Some(id) = &class.id {
                bindings.visit_binding_identifier(id);
            }
        }
        _ => {}
    }
    bindings.found
}

#[derive(Default)]
pub(super) struct Settled {
    scanned: usize,
    declared: FxHashSet<SymbolId>,
}

impl Settled {
    fn declares(&mut self, stmts: &[Statement<'_>], before: usize, symbol: SymbolId) -> bool {
        if before < self.scanned {
            self.scanned = 0;
            self.declared.clear();
        }
        for stmt in stmts.iter().take(before).skip(self.scanned) {
            self.declared.extend(lexical_bindings(stmt));
        }
        self.scanned = before;
        self.declared.contains(&symbol)
    }
}

fn is_bound_on_entry(id: &IdentifierReference<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let flags = scoping.symbol_flags(symbol);
    let lexical = SymbolFlags::BlockScopedVariable | SymbolFlags::Class | SymbolFlags::Import;
    if flags.intersects(lexical) {
        return false;
    }
    let declared_before = scoping.symbol_span(symbol).start <= id.span.start;
    flags.contains(SymbolFlags::Function)
        || (flags.contains(SymbolFlags::FunctionScopedVariable) && declared_before)
}

fn is_constant_global(id: &IdentifierReference<'_>, scoping: &Scoping) -> bool {
    CONSTANT_GLOBALS.contains(&id.name.as_str()) && is_global_reference(scoping, id)
}

fn is_primitive(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::Identifier(id) => is_constant_global(id, scoping),
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_primitive(&unary.argument, scoping)
        }
        Expression::BinaryExpression(binary) => {
            !matches!(
                binary.operator,
                BinaryOperator::In | BinaryOperator::Instanceof
            ) && is_primitive(&binary.left, scoping)
                && is_primitive(&binary.right, scoping)
        }
        _ => false,
    }
}

pub(super) struct Inert<'k, 'a> {
    pub(super) stmts: &'k [Statement<'a>],
    pub(super) before: usize,
    pub(super) scoping: &'k Scoping,
    pub(super) settled: &'k mut Settled,
}

impl Inert<'_, '_> {
    fn is_initialised(&mut self, id: &IdentifierReference<'_>) -> bool {
        match resolved_symbol(id, self.scoping) {
            Some(symbol) => {
                is_bound_on_entry(id, self.scoping, symbol)
                    || self.settled.declares(self.stmts, self.before, symbol)
            }
            None => is_constant_global(id, self.scoping),
        }
    }

    fn is_inert_property(&mut self, property: &ObjectPropertyKind<'_>) -> bool {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        let literal_key = matches!(
            property.key,
            PropertyKey::StringLiteral(_) | PropertyKey::NumericLiteral(_)
        );
        (!property.computed || literal_key) && self.accepts(&property.value)
    }

    fn is_inert_element(&mut self, element: &ArrayExpressionElement<'_>) -> bool {
        match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            other => other.as_expression().is_some_and(|expr| self.accepts(expr)),
        }
    }

    fn is_inert_unary(&mut self, unary: &UnaryExpression<'_>) -> bool {
        match unary.operator {
            UnaryOperator::LogicalNot | UnaryOperator::Void => self.accepts(&unary.argument),
            UnaryOperator::Typeof => match unary.argument.without_parentheses() {
                Expression::Identifier(id) if resolved_symbol(id, self.scoping).is_none() => {
                    is_global_reference(self.scoping, id)
                }
                argument => self.accepts(argument),
            },
            _ => false,
        }
    }

    fn is_inert_binary(&mut self, binary: &BinaryExpression<'_>) -> bool {
        matches!(
            binary.operator,
            BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
        ) && self.accepts(&binary.left)
            && self.accepts(&binary.right)
    }

    pub(super) fn accepts(&mut self, expr: &Expression<'_>) -> bool {
        if is_primitive(expr, self.scoping) {
            return true;
        }
        match expr.without_parentheses() {
            Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(id) => self.is_initialised(id),
            Expression::ArrayExpression(array) => array
                .elements
                .iter()
                .all(|element| self.is_inert_element(element)),
            Expression::ObjectExpression(object) => object
                .properties
                .iter()
                .all(|property| self.is_inert_property(property)),
            Expression::UnaryExpression(unary) => self.is_inert_unary(unary),
            Expression::BinaryExpression(binary) => self.is_inert_binary(binary),
            Expression::LogicalExpression(logical) => {
                self.accepts(&logical.left) && self.accepts(&logical.right)
            }
            Expression::ConditionalExpression(conditional) => {
                self.accepts(&conditional.test)
                    && self.accepts(&conditional.consequent)
                    && self.accepts(&conditional.alternate)
            }
            _ => false,
        }
    }
}
