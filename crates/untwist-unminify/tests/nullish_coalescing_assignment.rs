use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "a ?? (a = b);\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::nullish_coalescing_assignment::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

#[test]
fn identifier_assignment_babel() {
    assert_transform!(
        "a ?? (a = b);",
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a ??= b;"
    );
}

#[test]
fn member_expression_assignment_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a).b ?? (_a.b = c);"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a.b ??= c;"
    );
}

#[test]
fn computed_member_expression_assignment_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a)[b] ?? (_a[b] = c);"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a[b] ??= c;"
    );
}

#[test]
fn member_expression_assignment_ts() {
    assert_transform!(
        "a.b ?? (a.b = c);",
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a.b ??= c;"
    );
}

#[test]
#[ignore = "upstream test.skip: nested object temp"]
fn nested_object_temp_ts() {
    assert_transform!(
        r#"var _c;
    (_c = c.foo).baz ?? (_c.baz = result.foo.baz);"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"c.foo.baz ??= result.foo.baz;"
    );
}

#[test]
#[ignore = "upstream test.skip: side-effecting computed key"]
fn side_effecting_computed_key_ts() {
    assert_transform!(
        r#"var _a;
    obj[_a = incr()] ?? (obj[_a] = incr());"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"obj[incr()] ??= incr();"
    );
}

#[test]
#[ignore = "upstream test.skip: two temps with side-effecting computed key"]
fn two_temps_ts() {
    assert_transform!(
        r#"var _b;
var _c;
(_b = oobj.obj)[_c = incr()] ?? (_b[_c] = incr());"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @r#"oobj["obj"][incr()] ??= incr();"#
    );
}

fn default_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_unminify::nullish_coalescing_assignment::boxed],
        PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::nullish_coalescing_assignment::boxed],
    )
}

const UPDATE_KEY: &str = r#"function m(a, i, z) { a[i++] ?? (a[i++] = z); return [i, a]; }
console.log(JSON.stringify(m([null, null, null], 0, 7)));"#;

const CALL_OBJECT: &str = r#"var n = 0;
var o = { y: null };
function f() { n++; return o; }
f().y ?? (f().y = 2);
console.log(n, o.y);"#;

const GETTER_OBJECT: &str = r#"var reads = 0;
var inner = {};
var o = { get p() { reads++; return inner; } };
o.p.y ?? (o.p.y = 1);
console.log(reads, inner.y);"#;

const GLOBAL_ACCESSOR_OBJECT: &str = r#"var reads = 0;
var store = {};
Object.defineProperty(globalThis, "gobj", { get() { reads++; return store; }, configurable: true });
gobj.k ?? (gobj.k = 1);
console.log(reads, store.k);"#;

const TEMP_OBJECT_GLOBAL_ACCESSOR_KEY: &str = r#"var reads = 0;
var store = {};
Object.defineProperty(globalThis, "gkey", { get() { reads++; return "k"; }, configurable: true });
var t;
(t = store)[gkey] ?? (t[gkey] = 1);
console.log(reads, store.k);"#;

#[test]
fn a_repeated_operand_that_can_run_code_keeps_its_pair_by_default() {
    for source in [
        UPDATE_KEY,
        CALL_OBJECT,
        GETTER_OBJECT,
        GLOBAL_ACCESSOR_OBJECT,
        TEMP_OBJECT_GLOBAL_ACCESSOR_KEY,
    ] {
        let output = default_lane(source);
        assert!(!output.contains("??="), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_repeated_call_or_update_keeps_its_pair_under_unsafe() {
    for source in [UPDATE_KEY, CALL_OBJECT] {
        let output = unsafe_lane(source);
        assert!(!output.contains("??="), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_call_free_member_chain_still_converts_under_unsafe() {
    assert!(unsafe_lane(GETTER_OBJECT).contains("o.p.y ??= 1"));
    assert!(unsafe_lane(GLOBAL_ACCESSOR_OBJECT).contains("gobj.k ??= 1"));
    assert!(unsafe_lane(TEMP_OBJECT_GLOBAL_ACCESSOR_KEY).contains("store[gkey] ??= 1"));
}

#[test]
fn transpiler_output_still_converts_by_default() {
    let rows = [
        ("function h(a, b) { a ?? (a = b); return a; }", "a ??= b;"),
        ("function h(a, c) { a.b ?? (a.b = c); }", "a.b ??= c;"),
        ("function h(a, k, c) { a[k] ?? (a[k] = c); }", "a[k] ??= c;"),
        (
            "function h(a, c) { var _a; (_a = a).b ?? (_a.b = c); }",
            "a.b ??= c;",
        ),
        (
            "function h(a, k, c) { var _a; (_a = a)[k] ?? (_a[k] = c); }",
            "a[k] ??= c;",
        ),
        ("function h(c) { this.b ?? (this.b = c); }", "this.b ??= c;"),
    ];
    for (source, expected) in rows {
        let output = default_lane(source);
        assert!(output.contains(expected), "{output}");
    }
}

const GETTER_REBINDS_OBJECT: &str = r#"function t() { var other = {}; var a = { get k() { a = other; return null; } }; a.k ?? (a.k = 5); return [other.k]; }
console.log(JSON.stringify(t()));"#;

const GETTER_REBINDS_KEY: &str = r#"function t() { var i = 0; var a = { get 0() { i = 1; return null; } }; a[i] ?? (a[i] = 5); return [a[1], i]; }
console.log(JSON.stringify(t()));"#;

const GETTER_REBINDS_KEY_OF_TEMP_OBJECT: &str = r#"function t() { var i = 0; var a = { get 0() { i = 1; return null; } }; var _a; (_a = a)[i] ?? (_a[i] = 5); return [a[1], i]; }
console.log(JSON.stringify(t()));"#;

#[test]
fn an_operand_the_first_read_can_rebind_keeps_its_pair_by_default() {
    for source in [
        GETTER_REBINDS_OBJECT,
        GETTER_REBINDS_KEY,
        GETTER_REBINDS_KEY_OF_TEMP_OBJECT,
    ] {
        let output = default_lane(source);
        assert!(!output.contains("??="), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}
