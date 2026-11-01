use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_core::ast_util::literal::{
    is_false_literal, is_pure_value, is_safe_literal, is_true_literal, is_truthy_literal,
    is_undefined_like, numeric_value,
};
use untwist_core::ast_util::scope::{binding_symbol, references_symbol};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

fn first_expr<'a>(p: &'a Program<'a>) -> &'a Expression<'a> {
    match &p.body[0] {
        Statement::ExpressionStatement(s) => &s.expression,
        _ => panic!("expected expression statement"),
    }
}

fn check(src: &str, pred: fn(&Expression<'_>) -> bool) -> bool {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, src);
    pred(first_expr(&p.program))
}

#[test]
fn literal_predicates() {
    assert!(check("1", is_safe_literal));
    assert!(check("`a`", is_safe_literal));
    assert!(!check("`a${b}`", is_safe_literal));
    assert!(check("-Infinity", is_pure_value));
    assert!(check("undefined", is_pure_value));
    assert!(!check("x", is_pure_value));
    assert!(check("void 0", is_undefined_like));
    assert!(check("!0", is_true_literal));
    assert!(check("!![]", is_true_literal));
    assert!(check("![]", is_false_literal));
    assert!(check("[]", is_truthy_literal));
    assert!(!check("[1]", is_truthy_literal));
}

#[test]
fn is_true_literal_pins_exact_double_negated_one() {
    assert!(check("!!1", is_true_literal));
    assert!(!check("!!2", is_true_literal));
}

#[test]
fn numeric_value_handles_negation() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "-3");
    assert_eq!(numeric_value(first_expr(&p.program)), Some(-3.0));
}

#[test]
fn references_symbol_finds_uses_inside_expression() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "var i = 0; for (var j = 1; i < 10; i++) {}");
    let sem = build_scoping(&p.program);
    let Statement::VariableDeclaration(decl) = &p.program.body[0] else {
        panic!()
    };
    let i = binding_symbol(&decl.declarations[0].id).unwrap();
    let Statement::ForStatement(f) = &p.program.body[1] else {
        panic!()
    };
    assert!(references_symbol(f.test.as_ref().unwrap(), &sem.scoping, i));
    let Some(ForStatementInit::VariableDeclaration(jd)) = &f.init else {
        panic!()
    };
    let j = binding_symbol(&jd.declarations[0].id).unwrap();
    assert!(!references_symbol(
        f.test.as_ref().unwrap(),
        &sem.scoping,
        j
    ));
}

#[test]
fn fresh_span_hands_out_increasing_disjoint_spans() {
    use untwist_core::ast_util::respan::{fresh_span_with, SpanCounter};
    let counter = SpanCounter::new();
    let first = fresh_span_with(&counter).expect("a span");
    let second = fresh_span_with(&counter).expect("a span");
    assert!(
        first.start >= 1 << 31,
        "{first:?} overlaps the source range"
    );
    assert!(first.start < first.end, "{first:?} is not well ordered");
    assert!(
        second.start >= first.end,
        "{first:?} and {second:?} overlap"
    );
}

#[test]
fn an_exhausted_counter_hands_out_nothing() {
    use untwist_core::ast_util::respan::{fresh_span_with, SpanCounter};
    let counter = SpanCounter::starting_at(u64::from(u32::MAX));
    assert_eq!(fresh_span_with(&counter), None);
}

#[test]
fn the_synthetic_counter_restarts_on_every_reset_on_every_thread() {
    use oxc_span::Span;
    use untwist_core::ast_util::respan::{fresh_span, reset_synthetic_spans, SYNTHETIC_BASE};
    reset_synthetic_spans();
    let first = fresh_span().expect("a span");
    assert_eq!(first, Span::new(SYNTHETIC_BASE, SYNTHETIC_BASE + 1));
    let _ = fresh_span().expect("a span");
    reset_synthetic_spans();
    assert_eq!(fresh_span(), Some(first));
    let advanced = fresh_span().expect("a span");
    let on_a_worker = std::thread::spawn(|| {
        reset_synthetic_spans();
        fresh_span()
    })
    .join()
    .expect("the worker joins");
    assert_eq!(on_a_worker, Some(first));
    let unaffected = fresh_span().expect("a span");
    assert_eq!(unaffected, Span::new(advanced.end + 1, advanced.end + 2));
}
