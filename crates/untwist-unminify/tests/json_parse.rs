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

fn default_lane(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[boxed],
        untwist_core::pass::PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(&alloc, source, &[boxed])
}

const PARSE_REPLACED: &str =
    r#"JSON.parse = function (s) { return s.length; }; console.log(JSON.parse("[1,2]"));"#;

const PARSE_REDEFINED: &str = r#"Object.defineProperty(JSON, "parse", { value: function () { return "patched"; } }); console.log(JSON.parse("[1,2]"));"#;

const PARSE_REPLACED_THROUGH_THE_GLOBAL_OBJECT: &str =
    r#"globalThis.JSON.parse = function () { return "patched"; }; console.log(JSON.parse("1"));"#;

const JSON_REPLACED: &str =
    r#"JSON = { parse: function () { return "patched"; } }; console.log(JSON.parse("1"));"#;

const JSON_REPLACED_THROUGH_THE_GLOBAL_OBJECT: &str = r#"globalThis.JSON = { parse: function () { return "patched"; } }; console.log(JSON.parse("1"));"#;

const PARSE_REPLACED_THROUGH_AN_ALIAS: &str =
    r#"var J = JSON; J.parse = function () { return "patched"; }; console.log(JSON.parse("1"));"#;

const JSON_REPLACED_THROUGH_A_GLOBAL_ALIAS: &str = r#"var g = globalThis; g.JSON = { parse: function () { return "patched"; } }; console.log(JSON.parse("1"));"#;

const PARSE_REPLACED_INSIDE_A_FUNCTION: &str = r#"function patch() { JSON.parse = function () { return "patched"; }; } function read() { return JSON.parse("1"); } patch(); console.log(read());"#;

const PARSE_REPLACED_UNDER_A_COMPUTED_KEY: &str = r#"var k = "par" + "se"; JSON[k] = function () { return "patched"; }; console.log(JSON.parse("1"));"#;

const PARSE_REPLACED_BY_ASSIGN: &str = r#"Object.assign(JSON, { parse: function () { return "patched"; } }); console.log(JSON.parse("1"));"#;

const PARSE_DELETED: &str = r#"delete JSON.parse; try { console.log(JSON.parse("1")); } catch (e) { console.log("threw"); }"#;

const PATCHED_PARSE: [&str; 11] = [
    PARSE_REPLACED,
    PARSE_REDEFINED,
    PARSE_REPLACED_THROUGH_THE_GLOBAL_OBJECT,
    JSON_REPLACED,
    JSON_REPLACED_THROUGH_THE_GLOBAL_OBJECT,
    PARSE_REPLACED_THROUGH_AN_ALIAS,
    JSON_REPLACED_THROUGH_A_GLOBAL_ALIAS,
    PARSE_REPLACED_INSIDE_A_FUNCTION,
    PARSE_REPLACED_UNDER_A_COMPUTED_KEY,
    PARSE_REPLACED_BY_ASSIGN,
    PARSE_DELETED,
];

#[test]
fn a_program_that_patches_json_parse_keeps_its_calls_by_default() {
    for source in PATCHED_PARSE {
        let output = default_lane(source);
        assert!(output.contains("JSON.parse(\""), "{source}\n{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_patched_json_parse_is_assumed_away_under_unsafe() {
    let output = unsafe_lane(PARSE_REPLACED);
    assert!(output.contains("console.log([1, 2]);"), "{output}");
}

const UNPATCHED_PARSE: [(&str, &str); 6] = [
    (
        r#"console.log(JSON.parse("[1,2,3]"));"#,
        "console.log([1, 2, 3]);",
    ),
    (
        r#"var config = JSON.parse('{"a":1,"b":[true,null]}'); console.log(config.a, config.b.length);"#,
        "var config = {",
    ),
    (
        r#"var text = JSON.stringify({ a: 1 }); console.log(JSON.parse('{"a":1}').a, text);"#,
        "console.log({ \"a\": 1 }.a, text);",
    ),
    (
        r#"function A() {} A.prototype.parse = function () { return 0; }; console.log(JSON.parse("null"), new A().parse());"#,
        "console.log(null, new A().parse());",
    ),
    (
        r#"Array.prototype.last = function () { return this[this.length - 1]; }; console.log(JSON.parse("[1,2]").last());"#,
        "console.log([1, 2].last());",
    ),
    (
        r#"var isBrowser = typeof window !== "undefined" && typeof window.document !== "undefined"; console.log(JSON.parse("true"), isBrowser);"#,
        "console.log(true, isBrowser);",
    ),
];

#[test]
fn a_program_that_leaves_json_alone_still_gets_literals_by_default() {
    for (source, expected) in UNPATCHED_PARSE {
        let output = default_lane(source);
        assert!(output.contains(expected), "{source}\n{output}");
        assert!(!output.contains("JSON.parse"), "{source}\n{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}
