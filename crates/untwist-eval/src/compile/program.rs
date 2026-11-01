use std::rc::Rc;

use oxc_ast::ast::{Expression, Statement};

use crate::bail::Bail;
use crate::compile::scope::ScopeChain;
use crate::compile::{depth, Lazy, LazyExpr};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::value::{Completion, Eval, Value};

pub(crate) fn root() -> Option<Rc<ScopeChain>> {
    ScopeChain::root()
}

pub(crate) fn run_top_level<'a>(
    interp: &mut Interp<'a>,
    (statements, root): (&'a [Statement<'a>], &Rc<ScopeChain>),
    env: &FrameRef<'a>,
    mut last: Value<'a>,
) -> Eval<'a, Value<'a>> {
    let reach = depth::frame_reach(interp);
    for statement in statements {
        if let Statement::ExpressionStatement(node) = statement {
            last = LazyExpr::new(&node.expression, root.clone()).eval(interp, env, reach)?;
            continue;
        }
        let completion = Lazy::new(statement, root.clone()).run(interp, env, reach)?;
        if let Completion::Return(value) = completion {
            return Ok(value);
        }
        if !matches!(completion, Completion::Normal) {
            return Err(Bail::Unsupported(&"break or continue at program level").into());
        }
    }
    Ok(last)
}

fn unlaid_root<'a>() -> crate::value::Abrupt<'a> {
    Bail::Unsupported(&"a program root the compiler could not lay out").into()
}

impl<'a> Interp<'a> {
    pub fn run_root_statement(&mut self, statement: &'a Statement<'a>) -> Eval<'a, Completion<'a>> {
        let root = root().ok_or_else(unlaid_root)?;
        let env = self.global_env();
        let reach = depth::frame_reach(self);
        Lazy::new(statement, root).run(self, &env, reach)
    }

    pub fn eval_root_expression(&mut self, expression: &'a Expression<'a>) -> Eval<'a, Value<'a>> {
        let root = root().ok_or_else(unlaid_root)?;
        let env = self.global_env();
        let reach = depth::frame_reach(self);
        LazyExpr::new(expression, root).eval(self, &env, reach)
    }
}
