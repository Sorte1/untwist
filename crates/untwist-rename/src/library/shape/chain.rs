use oxc_ast::ast::*;
use oxc_ast::AstType;
use oxc_ast_visit::VisitJs;
use oxc_syntax::operator::LogicalOperator;

use super::{tagged, Emitter, TAG_OP};

fn operands<'e, 'a>(
    expr: &'e Expression<'a>,
    operator: LogicalOperator,
    out: &mut Vec<&'e Expression<'a>>,
) {
    match expr.without_parentheses() {
        Expression::LogicalExpression(logical) if logical.operator == operator => {
            operands(&logical.left, operator, out);
            operands(&logical.right, operator, out);
        }
        _ => out.push(expr),
    }
}

impl<'n, 'a: 'n> Emitter<'n, 'a> {
    pub(super) fn emit_logical(&mut self, logical: &LogicalExpression<'a>) {
        let mut chain = Vec::new();
        operands(&logical.left, logical.operator, &mut chain);
        operands(&logical.right, logical.operator, &mut chain);
        self.open(AstType::LogicalExpression);
        self.push(tagged(TAG_OP, logical.operator.as_str()));
        for operand in chain {
            self.visit_expression(operand);
        }
        self.close();
    }
}
