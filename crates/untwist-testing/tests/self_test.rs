use oxc_ast::ast::*;
use untwist_core::pass::{Cx, Pass};
use untwist_testing::assert_transform;

struct NullToUndefinedName;
impl<'a> Pass<'a> for NullToUndefinedName {
    fn name(&self) -> &'static str {
        "null-to-undefined"
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NullLiteral(n) = &*expr else {
            return;
        };
        let span = n.span;
        *expr = cx.tcx.create_unbound_ident_expr(
            span,
            oxc_str::Ident::from("undefined"),
            oxc_syntax::reference::ReferenceFlags::Read,
        );
        cx.changed();
    }
}
fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(NullToUndefinedName)
}

#[test]
fn macro_runs_passes_and_snapshots_pretty_output() {
    assert_transform!("x = null;", [boxed], @"x = undefined;");
}
