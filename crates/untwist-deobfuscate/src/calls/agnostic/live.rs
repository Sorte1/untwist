use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use crate::dead_code::opaque_test;

pub(super) fn opaque_branch<'k, 'a>(it: &'k IfStatement<'a>) -> Option<Option<&'k Statement<'a>>> {
    let taken = opaque_test(it.test.without_parentheses())?;
    Some(if taken {
        Some(&it.consequent)
    } else {
        it.alternate.as_ref()
    })
}

#[derive(Default)]
struct LiveReturns<'a> {
    first: Option<&'a ReturnStatement<'a>>,
    more: bool,
}

impl<'a> VisitJs<'a> for LiveReturns<'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _it: &Class<'a>) {}

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match opaque_branch(it) {
            Some(taken) => {
                if let Some(branch) = taken {
                    self.visit_statement(branch);
                }
            }
            None => walk_js::walk_if_statement(self, it),
        }
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if self.first.is_some() {
            self.more = true;
        } else {
            self.first = Some(self.alloc(it));
        }
    }
}

fn single_live_return<'a>(body: &'a FunctionBody<'a>) -> Option<&'a Expression<'a>> {
    let mut scan = LiveReturns::default();
    for statement in &body.statements {
        scan.visit_statement(statement);
    }
    if scan.more {
        return None;
    }
    scan.first?.argument.as_ref()
}

pub(super) fn iife_returned<'a>(init: &'a Expression<'a>) -> Option<&'a Function<'a>> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    let Expression::FunctionExpression(outer) = call.callee.without_parentheses() else {
        return None;
    };
    if call.optional
        || !call.arguments.is_empty()
        || !outer.params.items.is_empty()
        || outer.params.rest.is_some()
        || outer.generator
        || outer.r#async
    {
        return None;
    }
    let returned = single_live_return(outer.body.as_ref()?)?;
    let Expression::FunctionExpression(inner) = returned.without_parentheses() else {
        return None;
    };
    (inner.id.is_none() && !inner.generator && !inner.r#async).then_some(&**inner)
}
