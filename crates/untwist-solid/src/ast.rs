use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BindingPattern, CallExpression, Expression,
    IdentifierReference, Statement,
};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

pub fn string<'e>(expr: &'e Expression<'_>) -> Option<&'e str> {
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked)
            .and_then(|cooked| cooked.as_str()),
        _ => None,
    }
}

pub fn arg<'e, 'a>(call: &'e CallExpression<'a>, index: usize) -> Option<&'e Expression<'a>> {
    call.arguments.get(index).and_then(Argument::as_expression)
}

pub fn args<'e, 'a>(call: &'e CallExpression<'a>) -> Option<Vec<&'e Expression<'a>>> {
    call.arguments.iter().map(Argument::as_expression).collect()
}

pub fn symbol_of(ident: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let reference = ident.reference_id.get()?;
    scoping.get_reference(reference).symbol_id()
}

pub fn expr_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(ident) => symbol_of(ident, scoping),
        _ => None,
    }
}

pub fn binding(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(ident) => Some(ident.symbol_id()),
        _ => None,
    }
}

pub fn arrow_statements<'e, 'a>(arrow: &'e ArrowFunctionExpression<'a>) -> &'e [Statement<'a>] {
    arrow
        .get_function_body()
        .map_or(&[], |body| body.statements.as_slice())
}

pub fn thunk<'e, 'a>(expr: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
    let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
        return None;
    };
    if !arrow.params.items.is_empty() || arrow.params.rest.is_some() || arrow.r#async {
        return None;
    }
    if let Some(expr) = arrow.get_expression() {
        return Some(expr);
    }
    match arrow_statements(arrow) {
        [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
        _ => None,
    }
}

pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

pub fn is_simple(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::CallExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::ParenthesizedExpression(_)
    )
}

pub fn is_dynamic(expr: &Expression<'_>) -> bool {
    use oxc_ast::ast::{ArrayExpressionElement, BinaryOperator, ObjectPropertyKind};
    match expr.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_) => false,
        Expression::TemplateLiteral(template) => template.expressions.iter().any(is_dynamic),
        Expression::UnaryExpression(unary) => is_dynamic(&unary.argument),
        Expression::BinaryExpression(binary) => {
            binary.operator == BinaryOperator::In
                || is_dynamic(&binary.left)
                || is_dynamic(&binary.right)
        }
        Expression::LogicalExpression(logical) => {
            is_dynamic(&logical.left) || is_dynamic(&logical.right)
        }
        Expression::ConditionalExpression(conditional) => {
            is_dynamic(&conditional.test)
                || is_dynamic(&conditional.consequent)
                || is_dynamic(&conditional.alternate)
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().any(is_dynamic),
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| match element {
            ArrayExpressionElement::SpreadElement(_) => true,
            ArrayExpressionElement::Elision(_) => false,
            other => other.as_expression().is_none_or(is_dynamic),
        }),
        Expression::ObjectExpression(object) => {
            object.properties.iter().any(|property| match property {
                ObjectPropertyKind::SpreadProperty(_) => true,
                ObjectPropertyKind::ObjectProperty(property) => {
                    property.computed
                        || (!property.method
                            && property.kind == oxc_ast::ast::PropertyKind::Init
                            && is_dynamic(&property.value))
                }
            })
        }
        _ => true,
    }
}

pub fn once(expr: &Expression<'_>, text: String) -> String {
    match is_dynamic(expr) {
        true => format!("/*@once*/ {text}"),
        false => text,
    }
}
