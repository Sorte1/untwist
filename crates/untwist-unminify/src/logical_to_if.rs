use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN};
use untwist_core::pass::{Cx, Interest, Pass, StatementKind, StatementVariants};

#[derive(Default)]
pub struct LogicalToIf;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(LogicalToIf)
}

fn take_logical<'a>(stmt: &mut Statement<'a>, cx: &Cx<'a, '_>) -> Option<LogicalExpression<'a>> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let operator = match &statement.expression {
        Expression::LogicalExpression(logical) => logical.operator,
        _ => return None,
    };
    if operator == LogicalOperator::Coalesce {
        return None;
    }
    let placeholder = Expression::new_null_literal(SPAN, cx);
    let expression = std::mem::replace(&mut statement.expression, placeholder);
    match expression {
        Expression::LogicalExpression(logical) => Some(logical.unbox()),
        other => {
            statement.expression = other;
            None
        }
    }
}

const ACTED_ON_STATEMENTS: StatementVariants =
    StatementVariants::of(&[StatementKind::ExpressionStatement]);

impl<'a> Pass<'a> for LogicalToIf {
    fn name(&self) -> &'static str {
        "logical-to-if"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT.on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        let statement_span = stmt.span();
        let Some(logical) = take_logical(stmt, cx) else {
            return;
        };
        let operator = logical.operator;
        let body_span = logical.right.span();
        let body = Statement::new_expression_statement(body_span, logical.right, cx);
        let consequent = cx.block(body_span, AVec::from_iter_in([body], cx));
        let test = if operator == LogicalOperator::And {
            logical.left
        } else {
            let left_span = logical.left.span();
            Expression::new_unary_expression(left_span, UnaryOperator::LogicalNot, logical.left, cx)
        };
        *stmt = Statement::new_if_statement(statement_span, test, consequent, None, cx);
        cx.changed();
    }
}
