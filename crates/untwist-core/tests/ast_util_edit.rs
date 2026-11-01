use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span, SPAN};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::respan::{fresh_spans_expression_with, SpanCounter};
use untwist_core::ast_util::search::expression_at;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, Pass, PassGroup, PassOptions, StopReason};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::{build_scoping, read_reference_count, rebuild_scoping};

#[test]
fn expression_edits_replace_by_span_and_report_the_count() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, "console.log(f(1), g(2));");
    let mut sem = build_scoping(&parsed.program);
    let mut spans = Vec::new();
    let Statement::ExpressionStatement(stmt) = &parsed.program.body[0] else {
        panic!("expression statement");
    };
    let Expression::CallExpression(outer) = &stmt.expression else {
        panic!("call");
    };
    for argument in &outer.arguments {
        if let Some(expr) = argument.as_expression() {
            spans.push(expr.span());
        }
    }

    let mut edits = ExprEdits::new();
    for (index, span) in spans.iter().enumerate() {
        let value = Expression::new_string_literal(
            *span,
            oxc_str::Str::from_str_in(&format!("v{index}"), &builder),
            None,
            &builder,
        );
        assert!(edits.insert(*span, value));
    }
    assert!(!edits.insert(SPAN, Expression::new_null_literal(SPAN, &builder)));

    let applied = edits.apply(&mut parsed.program, &alloc, &mut sem.scoping);
    assert_eq!(applied, 2);
    assert_eq!(
        print(&parsed.program, PrintStyle::Compact),
        "console.log(`v0`,`v1`);"
    );
}

#[test]
fn statement_edits_remove_and_expand_in_place() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "a();\nb();\nc();");
    let mut sem = build_scoping(&parsed.program);
    let spans: Vec<Span> = parsed.program.body.iter().map(GetSpan::span).collect();

    let mut edits = StmtEdits::new();
    assert!(edits.remove(spans[1]));
    let extra = {
        let inner = parse_program(&alloc, "x();\ny();");
        inner.program.body.into_iter().collect::<Vec<_>>()
    };
    assert!(edits.replace(spans[2], extra));

    let applied = edits.apply(&mut parsed.program, &alloc, &mut sem.scoping);
    assert_eq!(applied, 2);
    assert_eq!(print(&parsed.program, PrintStyle::Compact), "a();x();y();");
}

fn array_element_spans(program: &oxc_ast::ast::Program<'_>) -> Vec<Span> {
    let Statement::ExpressionStatement(stmt) = &program.body[0] else {
        panic!("expression statement");
    };
    let Expression::ArrayExpression(array) = &stmt.expression else {
        panic!("array literal");
    };
    array
        .elements
        .iter()
        .filter_map(|element| element.as_expression())
        .map(GetSpan::span)
        .collect()
}

#[test]
fn one_subtree_cloned_into_two_sites_stays_two_distinguishable_nodes() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, "[p(0), q(0)];");
    let mut sem = build_scoping(&parsed.program);
    let template = parse_program(&alloc, "m(7);");
    let Statement::ExpressionStatement(stmt) = &template.program.body[0] else {
        panic!("expression statement");
    };
    let source = &stmt.expression;

    let mut clones = ExprEdits::new();
    for span in array_element_spans(&parsed.program) {
        assert!(clones.insert(span, source.clone_in(&alloc)));
    }
    assert_eq!(
        clones.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        2
    );

    let cloned = array_element_spans(&parsed.program);
    let mut edits = ExprEdits::new();
    for (index, span) in cloned.iter().enumerate() {
        let value = Expression::new_string_literal(
            *span,
            oxc_str::Str::from_str_in(&format!("v{index}"), &builder),
            None,
            &builder,
        );
        assert!(edits.insert(*span, value));
    }
    let applied = edits.apply(&mut parsed.program, &alloc, &mut sem.scoping);

    assert_eq!(print(&parsed.program, PrintStyle::Compact), "[`v0`,`v1`];");
    assert_eq!(applied, 2);
    assert_ne!(cloned[0], cloned[1], "the two clones share one span");
}

#[test]
fn a_second_edit_for_one_span_is_refused_rather_than_overwriting_the_first() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, "f(1);");
    let mut sem = build_scoping(&parsed.program);
    let span = array_call_argument_span(&parsed.program);

    let mut edits = ExprEdits::new();
    assert!(edits.insert(
        span,
        Expression::new_string_literal(
            span,
            oxc_str::Str::from_str_in("first", &builder),
            None,
            &builder
        )
    ));
    assert!(!edits.insert(
        span,
        Expression::new_string_literal(
            span,
            oxc_str::Str::from_str_in("second", &builder),
            None,
            &builder
        )
    ));
    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );
    assert_eq!(print(&parsed.program, PrintStyle::Compact), "f(`first`);");
}

fn array_call_argument_span(program: &oxc_ast::ast::Program<'_>) -> Span {
    let Statement::ExpressionStatement(stmt) = &program.body[0] else {
        panic!("expression statement");
    };
    let Expression::CallExpression(call) = &stmt.expression else {
        panic!("call");
    };
    call.arguments[0].as_expression().expect("argument").span()
}

#[test]
fn an_exhausted_span_counter_refuses_to_respan_rather_than_wrapping() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let counter = SpanCounter::starting_at(u64::from(u32::MAX));
    assert!(counter.is_exhausted());

    let mut expr = Expression::new_numeric_literal(
        Span::new(10, 20),
        1.0,
        None,
        oxc_syntax::number::NumberBase::Decimal,
        &builder,
    );
    assert!(!fresh_spans_expression_with(&counter, &mut expr));
    assert_eq!(expr.span(), Span::new(10, 20));
}

#[test]
fn a_fresh_counter_respans_above_every_source_offset() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let counter = SpanCounter::starting_at(0);
    let mut first = Expression::new_null_literal(Span::new(1, 2), &builder);
    assert!(fresh_spans_expression_with(&counter, &mut first));
    assert!(first.span().start >= 1 << 31, "{:?}", first.span());
    assert!(!counter.is_exhausted());
}

const CLONE_SOURCE: &str = "call(function (a) { return a; });\nsite;";
const CLONED_FUNCTION: Span = Span::new(5, 31);
const INSERT_SITE: Span = Span::new(34, 38);

#[derive(Default)]
struct Walker;

fn walker<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Walker)
}

impl<'a> Pass<'a> for Walker {
    fn name(&self) -> &'static str {
        "walker"
    }
}

#[test]
fn an_inserted_cloned_function_can_be_walked_by_oxc_traverse() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, CLONE_SOURCE);
    let mut sem = build_scoping(&parsed.program);
    let clone = expression_at(&parsed.program, CLONED_FUNCTION)
        .expect("function")
        .clone_in(&alloc);

    let mut edits = ExprEdits::new();
    assert!(edits.insert(INSERT_SITE, clone));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    let mut group = PassGroup::from_ctors(&[walker], PassOptions::default());
    let (_sem, report) = run_once(&alloc, &mut parsed.program, sem, &mut group);
    assert_eq!(report.stop, StopReason::Converged);
}

#[test]
fn a_scoping_rebuild_resolves_the_inserted_functions_inner_references() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, CLONE_SOURCE);
    let mut sem = build_scoping(&parsed.program);
    let clone = expression_at(&parsed.program, CLONED_FUNCTION)
        .expect("function")
        .clone_in(&alloc);

    let mut edits = ExprEdits::new();
    edits.insert(INSERT_SITE, clone);
    edits.apply(&mut parsed.program, &alloc, &mut sem.scoping);

    let rebuilt = rebuild_scoping(&parsed.program, sem.stats);
    let parameters: Vec<_> = rebuilt
        .scoping
        .symbol_ids()
        .filter(|symbol| rebuilt.scoping.symbol_name(*symbol) == "a")
        .collect();
    assert_eq!(parameters.len(), 2);
    for symbol in parameters {
        assert_eq!(read_reference_count(&rebuilt.scoping, symbol), 1);
    }
}
