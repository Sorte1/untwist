use oxc_ast::ast::{Expression, LogicalOperator, Statement};
use oxc_span::{GetSpan, Span};

use super::gate_guard;

fn gate_test(statement: &Statement<'_>) -> Option<Span> {
    match statement {
        Statement::IfStatement(node) if node.alternate.is_none() => {
            gate_guard(&node.test).map(|_| node.test.span())
        }
        Statement::ExpressionStatement(node) => match node.expression.without_parentheses() {
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                gate_guard(&logical.left).map(|_| logical.left.span())
            }
            _ => None,
        },
        _ => None,
    }
}

fn keeps_prefix(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::VariableDeclaration(_)
            | Statement::ExpressionStatement(_)
            | Statement::FunctionDeclaration(_)
            | Statement::EmptyStatement(_)
    )
}

pub(super) fn prefix_gate_tests(statements: &[Statement<'_>], out: &mut Vec<Span>) {
    for statement in statements {
        if let Some(test) = gate_test(statement) {
            out.push(test);
        } else if !keeps_prefix(statement) {
            return;
        }
    }
}
