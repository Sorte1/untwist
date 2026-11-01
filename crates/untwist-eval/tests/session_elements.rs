mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::decode::InterpEvaluator;
use untwist_eval::value::Value;

fn big_array_source(len: usize) -> String {
    let mut out = String::from("function a() { var h = [");
    for index in 0..len {
        out.push_str(&format!("'s{index}',"));
    }
    out.push_str("]; a = function () { return h; }; return a(); }");
    out
}

#[test]
fn element_reads_by_index_and_reports_out_of_range() {
    let alloc = Allocator::default();
    let source = big_array_source(3);
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "a", &[], None);
    let session = InterpEvaluator::default()
        .instantiate(&program)
        .expect("instantiates");

    assert_eq!(session.array_len(), 3);
    let Some(Value::Str(first)) = session.element(0) else {
        panic!("element 0 is a string");
    };
    assert_eq!(first.to_lossy_string(), "s0");
    let Some(Value::Str(last)) = session.element(2) else {
        panic!("element 2 is a string");
    };
    assert_eq!(last.to_lossy_string(), "s2");
    assert!(session.element(3).is_none());

    let Some(Value::Str(head)) = session.array_first() else {
        panic!("array_first is a string");
    };
    assert_eq!(head.to_lossy_string(), first.to_lossy_string());
}

#[test]
fn a_hole_in_the_string_array_is_refused_while_instantiating() {
    let alloc = Allocator::default();
    let source =
        "function a() { var h = ['s0', , 's2']; a = function () { return h; }; return a(); }";
    let parsed = parse_program(&alloc, source);
    let program = common::decoder_program(&parsed.program, "a", &[], None);
    let refused = InterpEvaluator::default()
        .instantiate(&program)
        .err()
        .map(|bail| bail.to_string());
    assert!(
        refused.as_deref().is_some_and(|text| text.contains("hole")),
        "{refused:?}"
    );
}
