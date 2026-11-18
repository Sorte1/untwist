use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use untwist_core::ast_util::naming_slot::without_unnamed;

#[derive(Default)]
struct KeyLoops {
    over_value: usize,
    over_member: usize,
}

impl<'a> VisitJs<'a> for KeyLoops {
    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        if it.right.without_parentheses().is_member_expression() {
            self.over_member += 1;
        } else {
            self.over_value += 1;
        }
        walk_js::walk_for_in_statement(self, it);
    }
}

pub(crate) fn is_domain_lock_callback(callback: &Expression<'_>) -> bool {
    let body = match without_unnamed(callback.without_parentheses()).without_parentheses() {
        Expression::FunctionExpression(function) => function.body.as_deref(),
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => Some(&**body),
            _ => None,
        },
        _ => None,
    };
    let Some(body) = body else {
        return false;
    };
    let mut loops = KeyLoops::default();
    loops.visit_function_body(body);
    loops.over_value > 0 && loops.over_member > 0
}
