use oxc_ast::ast::{ArrowFunctionExpression, Class, Function};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    functions: u32,
    closures: u32,
}

impl Frame {
    #[inline(always)]
    pub fn is_own(&self) -> bool {
        self.functions == 0
    }

    #[inline(always)]
    pub fn is_top(&self) -> bool {
        self.functions == 0 && self.closures == 0
    }
}

pub trait FrameScan<'a>: VisitJs<'a> {
    fn frame(&mut self) -> &mut Frame;
}

#[inline(always)]
pub fn walk_function<'a, V: FrameScan<'a>>(visitor: &mut V, it: &Function<'a>, flags: ScopeFlags) {
    visitor.frame().functions += 1;
    walk_js::walk_function(visitor, it, flags);
    visitor.frame().functions -= 1;
}

#[inline(always)]
pub fn walk_class<'a, V: FrameScan<'a>>(visitor: &mut V, it: &Class<'a>) {
    visitor.frame().functions += 1;
    walk_js::walk_class(visitor, it);
    visitor.frame().functions -= 1;
}

#[inline(always)]
pub fn walk_arrow<'a, V: FrameScan<'a>>(visitor: &mut V, it: &ArrowFunctionExpression<'a>) {
    visitor.frame().closures += 1;
    walk_js::walk_arrow_function_expression(visitor, it);
    visitor.frame().closures -= 1;
}
