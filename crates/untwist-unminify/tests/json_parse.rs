use untwist_testing::assert_transform;
use untwist_unminify::json_parse::boxed;

#[test]
fn array() {
    assert_transform!(r#"JSON.parse("[1,2,3]")"#, [boxed], @"[1, 2, 3];");
}

#[test]
fn large_literal() {
    assert_transform!(r#"JSON.parse("1000000000000000000000")"#, [boxed], @"1e21;");
}

#[test]
fn ignore_invalid_json() {
    assert_transform!(
        r#"JSON.parse("abc")"#,
        [boxed],
        @r#"JSON.parse("abc");"#
    );
}

#[test]
fn ignore_when_json_is_declared_in_scope() {
    assert_transform!(r#"let JSON; JSON.parse("null")"#, [boxed], @r#"
    let JSON;
    JSON.parse("null");
    "#);
}

const PROTO_KEY: &str = r#"var t = Object.assign({}, JSON.parse('{"a": [{"__proto__": {"x": "P"}}]}').a[0]); console.log(t.x);"#;

#[test]
fn a_proto_key_is_left_to_json_parse_in_either_mode() {
    for extended in [true, false] {
        let alloc = untwist_testing::Allocator::default();
        let output = untwist_testing::transform_with(
            &alloc,
            PROTO_KEY,
            &[boxed],
            untwist_core::pass::PassOptions {
                extended,
                ..untwist_core::pass::PassOptions::default()
            },
        );
        assert!(output.contains("JSON.parse"), "{output}");
        untwist_testing::node::assert_node_agrees(PROTO_KEY, &output);
    }
}

fn nested_json_parse(depth: usize) -> String {
    format!(
        "x = JSON.parse(\"{}1{}\");",
        "[".repeat(depth),
        "]".repeat(depth)
    )
}

fn transformed_on_a_stack_of(stack_bytes: usize, source: String) -> bool {
    std::thread::Builder::new()
        .stack_size(stack_bytes)
        .spawn(move || {
            let alloc = untwist_testing::Allocator::default();
            !untwist_testing::transform(&alloc, &source, &[boxed]).contains("JSON.parse")
        })
        .expect("a thread for the row")
        .join()
        .expect("the row ran")
}

#[test]
fn a_json_text_the_module_scan_refuses_is_left_unchanged() {
    const SERDE_ACCEPTS: usize = 120;
    assert!(transformed_on_a_stack_of(
        64 << 20,
        nested_json_parse(SERDE_ACCEPTS)
    ));
    assert!(!transformed_on_a_stack_of(
        1536 << 10,
        nested_json_parse(SERDE_ACCEPTS)
    ));
}
