use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};

fn statement(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::EmptyStatement(_) => true,
        Statement::BlockStatement(block) => block.body.iter().all(statement_ok),
        Statement::ReturnStatement(ret) => ret.argument.as_ref().is_none_or(expression),
        Statement::ExpressionStatement(stmt) => expression(&stmt.expression),
        Statement::IfStatement(stmt) => {
            expression(&stmt.test)
                && statement_ok(&stmt.consequent)
                && stmt.alternate.as_ref().is_none_or(statement_ok)
        }
        Statement::VariableDeclaration(declaration) => {
            !declaration.kind.is_using()
                && declaration.declarations.iter().all(|declarator| {
                    matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                        && declarator.init.as_ref().is_none_or(expression)
                })
        }
        _ => false,
    }
}

fn statement_ok(it: &Statement<'_>) -> bool {
    statement(it)
}

fn property(property: &ObjectPropertyKind<'_>) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return false;
    };
    let key = match &property.key {
        PropertyKey::StaticIdentifier(_) => true,
        key => key.as_expression().is_some_and(|key| {
            matches!(
                key,
                Expression::StringLiteral(_) | Expression::NumericLiteral(_)
            )
        }),
    };
    key && expression(&property.value)
}

fn expression(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.reference_id.get().is_some(),
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::ObjectExpression(object) => object.properties.iter().all(property),
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            other => other.as_expression().is_some_and(expression),
        }),
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::LogicalNot | UnaryOperator::Void
            ) && expression(&unary.argument)
        }
        Expression::LogicalExpression(logical) => {
            expression(&logical.left) && expression(&logical.right)
        }
        Expression::ConditionalExpression(conditional) => {
            expression(&conditional.test)
                && expression(&conditional.consequent)
                && expression(&conditional.alternate)
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().all(expression),
        Expression::AssignmentExpression(assignment) => {
            assignment.operator == AssignmentOperator::Assign
                && matches!(
                    assignment.left,
                    AssignmentTarget::AssignmentTargetIdentifier(_)
                )
                && expression(&assignment.right)
        }
        _ => false,
    }
}

pub fn is_trivial(function: &Function<'_>) -> bool {
    function.params.items.is_empty()
        && function.params.rest.is_none()
        && !function.r#async
        && !function.generator
        && function
            .body
            .as_ref()
            .is_some_and(|body| body.statements.iter().all(statement))
}
