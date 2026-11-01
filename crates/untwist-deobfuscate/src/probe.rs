use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};

type Test<'t, 'a> = &'t dyn Fn(&Expression<'a>) -> bool;

struct Probe<'t, 'a> {
    test: Test<'t, 'a>,
    hit: bool,
}

impl<'a> VisitJs<'a> for Probe<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.hit {
            return;
        }
        if (self.test)(it) {
            self.hit = true;
            return;
        }
        walk_js::walk_expression(self, it);
    }
}

pub fn contains_expression<'a>(
    stmt: &Statement<'a>,
    test: impl Fn(&Expression<'a>) -> bool + Copy,
) -> bool {
    let mut probe = Probe {
        test: &test,
        hit: false,
    };
    probe.visit_statement(stmt);
    probe.hit
}

pub fn expression_contains<'a>(
    expr: &Expression<'a>,
    test: impl Fn(&Expression<'a>) -> bool + Copy,
) -> bool {
    let mut probe = Probe {
        test: &test,
        hit: false,
    };
    probe.visit_expression(expr);
    probe.hit
}

pub(crate) fn holds_own_scope(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
    )
}
