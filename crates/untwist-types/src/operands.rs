use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::{AstNodes, Scoping};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

struct ObjectOperands<'s, 'a> {
    scoping: &'s Scoping,
    nodes: &'s AstNodes<'a>,
    found: bool,
}

fn is_arithmetic(operator: BinaryOperator) -> bool {
    !(operator.is_equality()
        || operator.is_compare()
        || matches!(operator, BinaryOperator::In | BinaryOperator::Instanceof))
}

fn is_object_literal(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::NewExpression(_)
            | Expression::RegExpLiteral(_)
    )
}

impl ObjectOperands<'_, '_> {
    fn bound_to_object(&self, ident: &IdentifierReference<'_>) -> bool {
        let Some(symbol) = ident
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        else {
            return false;
        };
        match self.nodes.kind(self.scoping.symbol_declaration(symbol)) {
            AstKind::VariableDeclarator(declarator) => {
                declarator.init.as_ref().is_some_and(is_object_literal)
            }
            AstKind::Function(_) | AstKind::Class(_) => true,
            _ => false,
        }
    }

    fn is_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(ident) => self.bound_to_object(ident),
            other => is_object_literal(other),
        }
    }

    fn target_is_object(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                self.bound_to_object(ident)
            }
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for ObjectOperands<'_, 'a> {
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.found |=
            is_arithmetic(it.operator) && (self.is_object(&it.left) || self.is_object(&it.right));
        walk::walk_binary_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let numeric = matches!(
            it.operator,
            UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation | UnaryOperator::BitwiseNot
        );
        self.found |= numeric && self.is_object(&it.argument);
        walk::walk_unary_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        self.found |= self.target_is_object(&it.argument);
        walk::walk_update_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let compound = it.operator.to_binary_operator().is_some_and(is_arithmetic);
        let target = it
            .left
            .as_simple_assignment_target()
            .is_some_and(|target| self.target_is_object(target));
        self.found |= compound && (target || self.is_object(&it.right));
        walk::walk_assignment_expression(self, it);
    }
}

pub(crate) fn has_object_arithmetic(
    program: &Program<'_>,
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
) -> bool {
    let mut operands = ObjectOperands {
        scoping,
        nodes,
        found: false,
    };
    operands.visit_program(program);
    operands.found
}
