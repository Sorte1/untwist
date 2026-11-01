use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use untwist_core::pass::{Cx, Pass};

#[derive(Default)]
pub struct PanicOnMarker;

impl<'a> Pass<'a> for PanicOnMarker {
    fn name(&self) -> &'static str {
        "test-panic-on-marker"
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        let Expression::StringLiteral(literal) = expr else {
            return;
        };
        assert!(literal.value != "boom", "the test pass panicked on purpose");
    }
}

pub fn panicking<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(PanicOnMarker)
}
