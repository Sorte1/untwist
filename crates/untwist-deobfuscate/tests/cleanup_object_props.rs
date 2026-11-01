mod common;

const MERGED_THEN_READ: &str =
    "function hi() {\n  var o = {};\n  o.a = 1;\n  o.b = 2;\n  console.log(o.a);\n}";

#[test]
fn the_default_resolves_a_property_read_of_the_merged_literal() {
    let (code, _notes) = common::run_all_stages_extended(MERGED_THEN_READ);
    assert_eq!(code.trim(), "function hi() {\n  console.log(1);\n}");
}

#[test]
fn conservative_keeps_the_references_shape() {
    let (code, _notes) = common::run_all_stages(MERGED_THEN_READ);
    assert!(code.contains("a: 1"), "{code}");
    assert!(!code.contains("console.log(1)"), "{code}");
}
