use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolFlags;
use untwist_core::ast_util::global::is_global_undefined;
use untwist_core::semantic::is_constant_binding;

use untwist_core::pass::Cx;

pub fn is_skipped_source(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    let expression = expression.without_parentheses();
    matches!(expression, Expression::NullLiteral(_))
        || expression.is_void_0()
        || is_global_undefined(expression, scoping)
}

pub fn is_string_like(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
    )
}

pub fn is_non_array_literal(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    let expression = expression.without_parentheses();
    is_skipped_source(expression, scoping)
        || matches!(
            expression,
            Expression::StringLiteral(_)
                | Expression::TemplateLiteral(_)
                | Expression::NumericLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::BigIntLiteral(_)
                | Expression::RegExpLiteral(_)
                | Expression::ObjectExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ArrowFunctionExpression(_)
                | Expression::ClassExpression(_)
                | Expression::NewExpression(_)
                | Expression::UnaryExpression(_)
                | Expression::BinaryExpression(_)
                | Expression::UpdateExpression(_)
        )
}

pub fn has_elision(array: &ArrayExpression<'_>) -> bool {
    array
        .elements
        .iter()
        .any(|element| matches!(element, ArrayExpressionElement::Elision(_)))
}

fn binding_is_inert(id: &IdentifierReference<'_>, scoping: &Scoping) -> bool {
    let Some(symbol) = id
        .reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
    else {
        return false;
    };
    let lexical =
        SymbolFlags::BlockScopedVariable | SymbolFlags::ConstVariable | SymbolFlags::Class;
    is_constant_binding(scoping, symbol) && !scoping.symbol_flags(symbol).intersects(lexical)
}

fn key_is_inert(property: &ObjectProperty<'_>) -> bool {
    !property.computed
        || matches!(
            property.key,
            PropertyKey::StringLiteral(_) | PropertyKey::NumericLiteral(_)
        )
}

fn member_is_inert(member: &ObjectPropertyKind<'_>, scoping: &Scoping) -> bool {
    match member {
        ObjectPropertyKind::ObjectProperty(property) => {
            key_is_inert(property) && is_inert(&property.value, scoping)
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    }
}

fn element_is_inert(element: &ArrayExpressionElement<'_>, scoping: &Scoping) -> bool {
    match element {
        ArrayExpressionElement::Elision(_) => true,
        ArrayExpressionElement::SpreadElement(_) => false,
        other => other
            .as_expression()
            .is_some_and(|expression| is_inert(expression, scoping)),
    }
}

pub fn is_inert(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::ThisExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::Identifier(id) => {
            is_global_undefined(expression, scoping) || binding_is_inert(id, scoping)
        }
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::Void | UnaryOperator::LogicalNot | UnaryOperator::UnaryNegation
            ) && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .all(|member| member_is_inert(member, scoping)),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element_is_inert(element, scoping)),
        _ => false,
    }
}

#[derive(Default)]
struct SuperScan {
    found: bool,
}

impl<'a> VisitJs<'a> for SuperScan {
    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }
}

pub fn uses_super(expression: &Expression<'_>) -> bool {
    let mut scan = SuperScan::default();
    walk_js::walk_expression(&mut scan, expression);
    scan.found
}

pub fn sets_prototype(property: &ObjectProperty<'_>) -> bool {
    property.kind == PropertyKind::Init
        && !property.computed
        && !property.shorthand
        && !property.method
        && property.key.is_specific_static_name("__proto__")
}

pub fn into_inner(expression: Expression<'_>) -> Expression<'_> {
    match expression {
        Expression::ParenthesizedExpression(parenthesized) => {
            into_inner(parenthesized.unbox().expression)
        }
        other => other,
    }
}

pub fn take_arguments<'a>(
    node: &mut Expression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<(oxc_span::Span, Vec<Expression<'a>>)> {
    let Expression::CallExpression(call) = node else {
        return None;
    };
    let span = call.span;
    let arguments = std::mem::replace(&mut call.arguments, AVec::new_in(cx));
    let taken: Vec<Expression<'a>> = arguments
        .into_iter()
        .filter_map(|argument| Expression::try_from(argument).ok())
        .map(into_inner)
        .collect();
    *node = Expression::new_null_literal(SPAN, cx);
    Some((span, taken))
}
