use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn identifier_or_assignment() {
    assert_transform!("x || (x = y)", [untwist_unminify::logical_assignments::boxed], @"x ||= y;");
}

#[test]
fn identifier_and_assignment() {
    assert_transform!("x && (x = y)", [untwist_unminify::logical_assignments::boxed], @"x &&= y;");
}

#[test]
fn member_expression_or_assignment_babel() {
    assert_transform!(
        r#"var _x$y;
    (_x$y = x.y()).z || (_x$y.z = b);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x.y().z ||= b;"
    );
}

#[test]
fn computed_member_expression_or_assignment_babel() {
    assert_transform!(
        r#"var _x$y;
    (_x$y = x.y())[z] || (_x$y[z] = b);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x.y()[z] ||= b;"
    );
}

#[test]
fn member_expression_or_assignment_ts() {
    assert_transform!("x.y || (x.y = z)", [untwist_unminify::logical_assignments::boxed], @"x.y ||= z;");
}

#[test]
fn computed_member_expression_or_assignment_multi_temp() {
    assert_transform!(
        r#"var _x, _y;
    (_x = x)[_y = y] || (_x[_y] = z);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x[y] ||= z;"
    );
}

#[test]
fn computed_member_expression_or_assignment_ts() {
    assert_transform!(
        "x[y] || (x[y] = z);",
        [untwist_unminify::logical_assignments::boxed],
        @"x[y] ||= z;"
    );
}

#[test]
fn computed_member_expression_or_assignment_single_temp() {
    assert_transform!(
        r#"var _y;
    x[_y = y] || (x[_y] = z);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x[y] ||= z;"
    );
}

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "var _a;\n((_a = o).x) || (_a.x = 1);\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::logical_assignments::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

fn default_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_unminify::logical_assignments::boxed],
        PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::logical_assignments::boxed],
    )
}

const UPDATE_KEY: &str = r#"function m(a, i, z) { a[i++] || (a[i++] = z); return [i, a]; }
console.log(JSON.stringify(m([0, 0, 0], 0, 7)));"#;

const CALL_OBJECT: &str = r#"var n = 0;
var o = { y: 1 };
function f() { n++; return o; }
f().y && (f().y = 2);
console.log(n, o.y);"#;

const CALL_KEY: &str = r#"var n = 0;
var a = {};
function k() { return "k" + n++; }
a[k()] || (a[k()] = 1);
console.log(n, JSON.stringify(a));"#;

const TEMP_KEY_CALL_OBJECT: &str = r#"var n = 0;
var store = {};
function a() { n++; return store; }
function k() { return "k"; }
var t;
a()[t = k()] || (a()[t] = 1);
console.log(n, JSON.stringify(store));"#;

const TEMP_OBJECT_UPDATE_KEY: &str = r#"function m(o, i, z) { var t; (t = o)[i++] || (t[i++] = z); return [i, o]; }
console.log(JSON.stringify(m([0, 0, 0], 0, 7)));"#;

const GETTER_OBJECT: &str = r#"var reads = 0;
var inner = {};
var o = { get p() { reads++; return inner; } };
o.p.y || (o.p.y = 1);
console.log(reads, inner.y);"#;

const GLOBAL_ACCESSOR_OBJECT: &str = r#"var reads = 0;
var store = {};
Object.defineProperty(globalThis, "gobj", { get() { reads++; return store; }, configurable: true });
gobj.k || (gobj.k = 1);
console.log(reads, store.k);"#;

const GLOBAL_ACCESSOR_KEY: &str = r#"var reads = 0;
var store = {};
Object.defineProperty(globalThis, "gkey", { get() { reads++; return "k"; }, configurable: true });
store[gkey] || (store[gkey] = 1);
console.log(reads, store.k);"#;

const TEMP_KEY_GETTER_OBJECT: &str = r#"var reads = 0;
var inner = {};
var o = { get p() { reads++; return inner; } };
function k() { return "k"; }
var t;
o.p[t = k()] || (o.p[t] = 1);
console.log(reads, inner.k);"#;

const WITH_IDENTIFIER: &str = r#"var seen = 0;
var x = 0;
var scope = new Proxy({ x: 0 }, { has(target, key) { if (key === "x") seen++; return key in target; } });
with (scope) { x || (x = 5); }
console.log(seen, scope.x, x);"#;

const RUNS_CODE_IN_EITHER_LANE: [&str; 5] = [
    UPDATE_KEY,
    CALL_OBJECT,
    CALL_KEY,
    TEMP_KEY_CALL_OBJECT,
    TEMP_OBJECT_UPDATE_KEY,
];

const READS_TWICE: [&str; 5] = [
    GETTER_OBJECT,
    GLOBAL_ACCESSOR_OBJECT,
    GLOBAL_ACCESSOR_KEY,
    TEMP_KEY_GETTER_OBJECT,
    WITH_IDENTIFIER,
];

fn has_logical_assignment(output: &str) -> bool {
    output.contains("||=") || output.contains("&&=")
}

#[test]
fn a_repeated_operand_that_can_run_code_keeps_its_pair_by_default() {
    for source in RUNS_CODE_IN_EITHER_LANE.iter().chain(&READS_TWICE) {
        let output = default_lane(source);
        assert!(!has_logical_assignment(&output), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_repeated_call_or_update_keeps_its_pair_under_unsafe() {
    for source in RUNS_CODE_IN_EITHER_LANE {
        let output = unsafe_lane(source);
        assert!(!has_logical_assignment(&output), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_call_free_member_chain_still_converts_under_unsafe() {
    assert!(unsafe_lane(GETTER_OBJECT).contains("o.p.y ||= 1"));
    assert!(unsafe_lane(GLOBAL_ACCESSOR_OBJECT).contains("gobj.k ||= 1"));
    assert!(unsafe_lane(GLOBAL_ACCESSOR_KEY).contains("store[gkey] ||= 1"));
    assert!(unsafe_lane(TEMP_KEY_GETTER_OBJECT).contains("o.p[k()] ||= 1"));
    assert!(unsafe_lane("a[i + 1] || (a[i + 1] = z);").contains("a[i + 1] ||= z"));
}

#[test]
fn transpiler_output_still_converts_by_default() {
    let rows = [
        ("function h(a) { a.x || (a.x = 1); }", "a.x ||= 1;"),
        ("function h(a, k) { a[k] || (a[k] = 2); }", "a[k] ||= 2;"),
        (
            r#"function h(a) { a["s"] && (a["s"] = 3); }"#,
            r#"a["s"] &&= 3;"#,
        ),
        ("function h(a) { a[0] || (a[0] = 3); }", "a[0] ||= 3;"),
        ("function h(x) { x || (x = 4); return x; }", "x ||= 4;"),
        ("function h() { this.q || (this.q = 5); }", "this.q ||= 5;"),
        (
            "function h(f, b) { var _o; (_o = f()).z || (_o.z = b); }",
            "f().z ||= b;",
        ),
        (
            "function h(f, k, b) { var _o; (_o = f())[k] || (_o[k] = b); }",
            "f()[k] ||= b;",
        ),
        (
            "function h(a, k, z) { var _k; a[_k = k()] || (a[_k] = z); }",
            "a[k()] ||= z;",
        ),
        (
            "function h(x, y, z) { var _x, _y; (_x = x.p)[_y = y()] || (_x[_y] = z); }",
            "x.p[y()] ||= z;",
        ),
    ];
    for (source, expected) in rows {
        let output = default_lane(source);
        assert!(output.contains(expected), "{output}");
    }
}

const GETTER_REBINDS_OBJECT: &str = r#"function t() { var other = {}; var a = { get k() { a = other; return 0; } }; var first = a; a.k || (a.k = 5); return [other.k, first === a]; }
console.log(JSON.stringify(t()));
function t3() { var other = {}; var a = { get k() { a = other; return 1; } }; a.k && (a.k = 5); return [other.k]; }
console.log(JSON.stringify(t3()));"#;

const GETTER_REBINDS_KEY: &str = r#"function t() { var i = 0; var a = { get 0() { i = 1; return 0; } }; a[i] || (a[i] = 5); return [a[1], i]; }
console.log(JSON.stringify(t()));"#;

const GETTER_REBINDS_TEMP_KEY_OBJECT: &str = r#"function t() { var other = {}; var a = { get k() { a = other; return 0; } }; var _k; a[_k = "k"] || (a[_k] = 5); return [other.k]; }
console.log(JSON.stringify(t()));"#;

const ARROW_ARGUMENTS_REBINDS_PARAMETER: &str = r#"function t(a) { var other = {}; Object.defineProperty(a, "k", { get: () => { arguments[0] = other; return 0; } }); a.k || (a.k = 5); return [other.k]; }
console.log(JSON.stringify(t({})));"#;

const GLOBAL_OBJECT_REBINDS_SCRIPT_VAR: &str = r#"var other = {};
var a = { get k() { globalThis.a = other; return 0; } };
a.k || (a.k = 5);
console.log(JSON.stringify([other.k]));"#;

const INDIRECT_EVAL_REBINDS_SCRIPT_LET: &str = r#"let other = {};
let a = { get k() { (0, eval)("a = other"); return 0; } };
a.k || (a.k = 5);
console.log(JSON.stringify([other.k]));"#;

const REBINDABLE_OPERANDS: [&str; 6] = [
    GETTER_REBINDS_OBJECT,
    GETTER_REBINDS_KEY,
    GETTER_REBINDS_TEMP_KEY_OBJECT,
    ARROW_ARGUMENTS_REBINDS_PARAMETER,
    GLOBAL_OBJECT_REBINDS_SCRIPT_VAR,
    INDIRECT_EVAL_REBINDS_SCRIPT_LET,
];

#[test]
fn an_operand_the_first_read_can_rebind_keeps_its_pair_by_default() {
    for source in REBINDABLE_OPERANDS {
        let output = default_lane(source);
        assert!(!has_logical_assignment(&output), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn an_imported_object_keeps_its_pair_by_default() {
    let alloc = Allocator::default();
    let output = untwist_testing::transform_with(
        &alloc,
        "import { a } from \"m\"; a.k || (a.k = 5);",
        &[untwist_unminify::logical_assignments::boxed],
        PassOptions::default(),
    );
    assert!(!has_logical_assignment(&output), "{output}");
}

#[test]
fn a_settled_operand_still_converts_by_default() {
    let rows = [
        ("const a = {}; a.k || (a.k = 5);", "a.k ||= 5;"),
        ("const a = {}, i = 0; a[i] || (a[i] = 5);", "a[i] ||= 5;"),
        (
            "function h() { var a = {}; a.k || (a.k = 5); }",
            "a.k ||= 5;",
        ),
        (
            "function h(a) { \"use strict\"; a.k || (a.k = 5); }",
            "a.k ||= 5;",
        ),
        ("{ let a = {}; a.k || (a.k = 5); }", "a.k ||= 5;"),
    ];
    for (source, expected) in rows {
        let output = default_lane(source);
        assert!(output.contains(expected), "{output}");
    }
}
