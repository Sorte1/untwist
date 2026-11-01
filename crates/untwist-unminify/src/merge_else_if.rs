use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;
use untwist_core::pass::{Cx, Interest, Pass, StatementKind, StatementVariants};

#[derive(Default)]
pub struct MergeElseIf;

fn lone_nested_if(stmt: &Statement<'_>) -> bool {
    let Statement::BlockStatement(block) = stmt else {
        return false;
    };
    block.body.len() == 1 && matches!(block.body[0], Statement::IfStatement(_))
}

fn negated_test<'a>(statement: &IfStatement<'a>) -> bool {
    matches!(&statement.test, Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::LogicalNot)
}

impl MergeElseIf {
    fn merge_else(&mut self, stmt: &mut Statement<'_>) -> bool {
        let Statement::IfStatement(statement) = stmt else {
            return false;
        };
        let Some(alternate) = statement.alternate.as_mut() else {
            return false;
        };
        if !lone_nested_if(alternate) {
            return false;
        }
        let Statement::BlockStatement(block) = alternate else {
            return false;
        };
        let Some(nested) = block.body.pop() else {
            return false;
        };
        *alternate = nested;
        true
    }

    fn invert<'a>(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Statement::IfStatement(statement) = stmt else {
            return false;
        };
        if !negated_test(statement) || !lone_nested_if(&statement.consequent) {
            return false;
        }
        let Some(alternate) = statement.alternate.take() else {
            return false;
        };
        let placeholder = Expression::new_null_literal(SPAN, &*cx);
        let unary = match std::mem::replace(&mut statement.test, placeholder) {
            Expression::UnaryExpression(unary) => unary,
            other => {
                statement.test = other;
                statement.alternate = Some(alternate);
                return false;
            }
        };
        statement.test = unary.unbox().argument;
        let consequent = std::mem::replace(&mut statement.consequent, alternate);
        statement.alternate = Some(consequent);
        true
    }
}

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[StatementKind::IfStatement]);

impl<'a> Pass<'a> for MergeElseIf {
    fn name(&self) -> &'static str {
        "merge-else-if"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT.on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if self.merge_else(stmt) || (cx.opts.extended && self.invert(stmt, cx)) {
            cx.changed();
        }
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(MergeElseIf)
}
