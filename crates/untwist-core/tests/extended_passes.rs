use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, Cx, Pass, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;

#[derive(Default)]
struct Widening;

impl<'a> Pass<'a> for Widening {
    fn name(&self) -> &'static str {
        "widening"
    }

    fn requires_extended(&self) -> bool {
        true
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
    Box::new(Widening)
}

fn run(opts: PassOptions) -> String {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "var x = 1;");
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(&[boxed], opts);
    let _ = run_once(&alloc, &mut parsed.program, sem, &mut group);
    untwist_core::print::print(&parsed.program, untwist_core::print::PrintStyle::Compact)
}

#[test]
fn the_option_is_on_by_default() {
    assert!(PassOptions::default().extended);
}

#[test]
fn a_superset_pass_runs_by_default() {
    assert_eq!(run(PassOptions::default()), "var x=2;");
}

#[test]
fn a_superset_pass_is_inert_under_conservative() {
    let opts = PassOptions {
        extended: false,
        ..PassOptions::default()
    };
    assert_eq!(run(opts), "var x=1;");
}
