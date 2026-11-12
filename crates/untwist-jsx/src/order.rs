use oxc_ast::ast::*;
use oxc_span::{Span, SPAN};
use rustc_hash::FxHashSet;

use crate::automatic::is_pragma_callee;

pub fn first_calls(statement: &Statement<'_>, out: &mut FxHashSet<Span>) {
    let mut order = Order { out, clean: true };
    match statement {
        Statement::ExpressionStatement(statement) => order.expression(&statement.expression),
        Statement::ReturnStatement(statement) => {
            if let Some(argument) = &statement.argument {
                order.expression(argument);
            }
        }
        Statement::ThrowStatement(statement) => order.expression(&statement.argument),
        Statement::VariableDeclaration(declaration) => {
            for declarator in &declaration.declarations {
                if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
                    order.clean = false;
                }
                if let Some(init) = &declarator.init {
                    order.expression(init);
                }
            }
        }
        _ => {}
    }
}

struct Order<'o> {
    out: &'o mut FxHashSet<Span>,
    clean: bool,
}

fn inert(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::Identifier(_)
            | Expression::ThisExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
    ) || matches!(expression.without_parentheses(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty())
}

impl Order<'_> {
    fn expression(&mut self, expression: &Expression<'_>) {
        if !self.clean || inert(expression) {
            return;
        }
        match expression.without_parentheses() {
            Expression::CallExpression(call) => self.call(call),
            Expression::ObjectExpression(object) => self.object(object),
            Expression::ArrayExpression(array) => self.array(array),
            Expression::SequenceExpression(sequence) => {
                sequence
                    .expressions
                    .iter()
                    .for_each(|it| self.expression(it));
            }
            Expression::AssignmentExpression(assignment) => {
                if matches!(
                    assignment.left,
                    AssignmentTarget::AssignmentTargetIdentifier(_)
                ) {
                    self.expression(&assignment.right);
                }
                self.clean = false;
            }
            _ => self.clean = false,
        }
    }

    fn call(&mut self, call: &CallExpression<'_>) {
        let pragma = is_pragma_callee(&call.callee);
        if !pragma && !inert(&call.callee) {
            self.clean = false;
            return;
        }
        if pragma && !call.optional && call.span != SPAN {
            self.out.insert(call.span);
        }
        for argument in &call.arguments {
            match argument.as_expression() {
                Some(expression) => self.expression(expression),
                None => self.clean = false,
            }
        }
        if !pragma {
            self.clean = false;
        }
    }

    fn object(&mut self, object: &ObjectExpression<'_>) {
        for property in &object.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(property)
                    if !property.computed || property.key.as_expression().is_some_and(inert) =>
                {
                    self.expression(&property.value);
                }
                _ => self.clean = false,
            }
        }
    }

    fn array(&mut self, array: &ArrayExpression<'_>) {
        for element in &array.elements {
            match element {
                ArrayExpressionElement::Elision(_) => {}
                ArrayExpressionElement::SpreadElement(_) => self.clean = false,
                other => match other.as_expression() {
                    Some(expression) => self.expression(expression),
                    None => self.clean = false,
                },
            }
        }
    }
}
