use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{CallExpression, Expression, Program, Statement, UnaryOperator};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span, SPAN};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_core::source_map::{fill_spans, Origins, Recording};

const CALLS: &str = "console.log(f(1), g(2));";
const NUDGE: u32 = 2;

fn sits_in(actual: Span, expected: Span) -> bool {
    actual.end == expected.end
        && actual.start >= expected.start
        && actual.start - expected.start <= NUDGE
}

fn outer_call<'p, 'a>(program: &'p Program<'a>) -> &'p CallExpression<'a> {
    let Statement::ExpressionStatement(statement) = &program.body[0] else {
        panic!("expression statement");
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("call");
    };
    call
}

fn argument<'p, 'a>(program: &'p Program<'a>, index: usize) -> &'p Expression<'a> {
    outer_call(program).arguments[index]
        .as_expression()
        .expect("an expression argument")
}

fn span_of(text: &str, needle: &str) -> Span {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is in the text"));
    let start = u32::try_from(at).expect("a small offset");
    Span::new(
        start,
        start + u32::try_from(needle.len()).expect("a small length"),
    )
}

#[test]
fn a_replacement_without_a_span_takes_the_span_it_replaced() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, CALLS);
    let mut sem = build_scoping(&parsed.program);
    let recording = Recording::start();
    let mut edits = ExprEdits::new();
    let value = Expression::new_string_literal(
        SPAN,
        oxc_str::Str::from_str_in("decoded", &builder),
        None,
        &builder,
    );
    assert!(edits.insert(span_of(CALLS, "g(2)"), value));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    fill_spans(&mut parsed.program, &recording.finish());

    assert_eq!(argument(&parsed.program, 1).span(), span_of(CALLS, "g(2)"));
    assert!(sits_in(
        argument(&parsed.program, 0).span(),
        span_of(CALLS, "f(1)")
    ));
}

#[test]
fn a_node_moved_in_from_elsewhere_takes_the_span_of_its_site() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, CALLS);
    let mut sem = build_scoping(&parsed.program);
    let moved = argument(&parsed.program, 0).clone_in(&alloc);
    let recording = Recording::start();
    let mut edits = ExprEdits::new();
    assert!(edits.insert(span_of(CALLS, "g(2)"), moved));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    fill_spans(&mut parsed.program, &recording.finish());

    let Expression::CallExpression(landed) = argument(&parsed.program, 1) else {
        panic!("the moved call");
    };
    assert!(sits_in(landed.span, span_of(CALLS, "g(2)")));
    assert_eq!(landed.callee.span(), span_of(CALLS, "g(2)"));
    assert_eq!(landed.arguments[0].span(), span_of(CALLS, "g(2)"));
}

#[test]
fn a_part_kept_from_the_replaced_node_keeps_its_own_span() {
    let source = "use(wrap(inner + 1));";
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let mut sem = build_scoping(&parsed.program);
    let Expression::CallExpression(wrap) = argument(&parsed.program, 0) else {
        panic!("inner call");
    };
    let kept = wrap.arguments[0]
        .as_expression()
        .expect("an expression")
        .clone_in(&alloc);
    let recording = Recording::start();
    let mut edits = ExprEdits::new();
    assert!(edits.insert(span_of(source, "wrap(inner + 1)"), kept));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    fill_spans(&mut parsed.program, &recording.finish());

    let Expression::BinaryExpression(sum) = argument(&parsed.program, 0) else {
        panic!("the kept sum");
    };
    assert!(sits_in(sum.span, span_of(source, "inner + 1")));
    assert_eq!(sum.left.span(), span_of(source, "inner"));
    assert_eq!(sum.right.span(), span_of(source, "1"));
}

#[test]
fn statements_put_in_place_of_one_take_its_span() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "a();\nb();\nc();");
    let mut sem = build_scoping(&parsed.program);
    let spans: Vec<Span> = parsed.program.body.iter().map(GetSpan::span).collect();
    let extra = {
        let inner = parse_program(&alloc, "first();\nsecond();");
        inner.program.body.into_iter().collect::<Vec<_>>()
    };
    let recording = Recording::start();
    let mut edits = StmtEdits::new();
    assert!(edits.replace(spans[2], extra));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    fill_spans(&mut parsed.program, &recording.finish());

    let after: Vec<Span> = parsed.program.body.iter().map(GetSpan::span).collect();
    let expected = [spans[0], spans[1], spans[2], spans[2]];
    assert_eq!(after.len(), expected.len());
    for (actual, expected) in after.into_iter().zip(expected) {
        assert!(sits_in(actual, expected), "{actual:?} sits in {expected:?}");
    }
}

#[test]
fn a_node_without_a_span_takes_the_first_span_inside_it() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, "use(flag);");
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("expression statement");
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        panic!("call");
    };
    let argument = call.arguments[0]
        .as_expression_mut()
        .expect("an expression");
    let inner = std::mem::replace(argument, Expression::new_null_literal(SPAN, &builder));
    *argument = Expression::new_unary_expression(SPAN, UnaryOperator::LogicalNot, inner, &builder);

    fill_spans(&mut parsed.program, &Origins::default());

    let Expression::UnaryExpression(wrapped) = self::argument(&parsed.program, 0) else {
        panic!("the wrapping negation");
    };
    assert!(sits_in(wrapped.span, span_of("use(flag);", "flag")));
    assert_eq!(wrapped.argument.span(), span_of("use(flag);", "flag"));
}

#[test]
fn nodes_that_start_together_get_starts_that_differ() {
    let source = "left.right();";
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);

    fill_spans(&mut parsed.program, &Origins::default());

    let statement = parsed.program.body[0].span();
    let call = outer_call(&parsed.program);
    let Expression::StaticMemberExpression(member) = &call.callee else {
        panic!("a member callee");
    };
    let starts = [
        statement.start,
        call.span.start,
        member.span.start,
        member.object.span().start,
    ];
    assert_eq!(member.object.span(), span_of(source, "left"));
    assert_eq!(member.property.span, span_of(source, "right"));
    for pair in starts.windows(2) {
        assert_ne!(pair[0], pair[1]);
    }
    for start in starts {
        assert!(start <= NUDGE);
    }
    assert_eq!(statement.end, span_of(source, source).end);
}

#[test]
fn a_leaf_without_a_span_takes_the_span_around_it() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, CALLS);
    let mut sem = build_scoping(&parsed.program);
    let mut edits = ExprEdits::new();
    let null = Expression::new_null_literal(SPAN, &builder);
    assert!(edits.insert(span_of(CALLS, "g(2)"), null));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    fill_spans(&mut parsed.program, &Origins::default());

    assert_eq!(
        argument(&parsed.program, 1).span(),
        span_of(CALLS, "console.log(f(1), g(2))")
    );
}

#[test]
fn an_inner_recording_hands_the_outer_one_back() {
    let outer = Recording::start();
    drop(Recording::start());
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, CALLS);
    let mut sem = build_scoping(&parsed.program);
    let mut edits = ExprEdits::new();
    let null = Expression::new_null_literal(SPAN, &builder);
    assert!(edits.insert(span_of(CALLS, "g(2)"), null));
    edits.apply(&mut parsed.program, &alloc, &mut sem.scoping);

    fill_spans(&mut parsed.program, &outer.finish());

    assert_eq!(argument(&parsed.program, 1).span(), span_of(CALLS, "g(2)"));
}
