use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, Cx, Pass, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;

#[derive(Default)]
struct BumpEveryNumber;

impl<'a> Pass<'a> for BumpEveryNumber {
    fn name(&self) -> &'static str {
        "bump-every-number"
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NumericLiteral(literal) = &*expr else {
            return;
        };
        let value = literal.value + 1.0;
        let span = expr.span();
        *expr = Expression::new_numeric_literal(span, value, None, NumberBase::Decimal, &*cx);
        cx.changed();
    }
}

fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(BumpEveryNumber)
}

#[test]
fn run_once_traverses_exactly_one_time() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "var x = 1;");
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(&[boxed], PassOptions::default());
    let (_sem, report) = run_once(&alloc, &mut parsed.program, sem, &mut group);
    assert_eq!(report.iterations, 1);
    assert_eq!(report.total_changes, 1);
    assert!(report.converged());
    assert_eq!(
        untwist_core::print::print(&parsed.program, untwist_core::print::PrintStyle::Compact),
        "var x=2;"
    );
}
