use untwist_testing::assert_transform;

#[test]
fn if_statement() {
    assert_transform!(
        "if (!!a) {}",
        [untwist_unminify::remove_double_not::boxed],
        @"if (a) {}"
    );
}

#[test]
fn ternary() {
    assert_transform!(
        "!!a ? b : c;",
        [untwist_unminify::remove_double_not::boxed],
        @"a ? b : c;"
    );
}

#[test]
fn triple_not() {
    assert_transform!(
        "function f() {\n  return !!!a;\n}",
        [untwist_unminify::remove_double_not::boxed],
        @r"
    function f() {
      return !a;
    }
    "
    );
}

#[test]
fn array_filter() {
    assert_transform!(
        "[].filter(a => !!a);",
        [untwist_unminify::remove_double_not::boxed],
        @"[].filter((a) => a);"
    );
}

#[test]
fn keeps_double_not_used_as_a_value() {
    assert_transform!(
        "x = !!a;",
        [untwist_unminify::remove_double_not::boxed],
        @"x = !!a;"
    );
}

fn default_lane(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_unminify::remove_double_not::boxed],
        untwist_core::pass::PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::remove_double_not::boxed],
    )
}

const PREDICATE_METHODS: [&str; 7] = [
    "filter",
    "find",
    "findLast",
    "findIndex",
    "findLastIndex",
    "some",
    "every",
];

fn assert_callback_keeps_its_double_not(source: &str) {
    let output = default_lane(source);
    assert!(output.contains("(v) => !!v"), "{source}\n{output}");
    untwist_testing::node::assert_node_agrees(source, &output);
}

#[test]
fn a_replaced_array_method_keeps_the_double_not_in_its_callback_by_default() {
    for method in PREDICATE_METHODS {
        let source = format!(
            "Array.prototype.{method} = function (f) {{ return f(0); }}; console.log([0].{method}((v) => !!v));"
        );
        assert_callback_keeps_its_double_not(&source);
    }
}

const FILTER_REDEFINED: &str = r#"Object.defineProperty(Array.prototype, "filter", { value: function (f) { return f(0); } }); console.log([0].filter((v) => !!v));"#;

const FIND_REPLACED_THROUGH_AN_ALIAS: &str = "var p = Array.prototype; p.find = function (f) { return f(0); }; console.log([0].find((v) => !!v));";

const SOME_REPLACED_THROUGH_THE_GLOBAL_OBJECT: &str = "globalThis.Array.prototype.some = function (f) { return f(0); }; console.log([0].some((v) => !!v));";

const EVERY_REPLACED_UNDER_A_COMPUTED_KEY: &str = r#"var k = "eve" + "ry"; Array.prototype[k] = function (f) { return f(0); }; console.log([0].every((v) => !!v));"#;

const FIND_INDEX_REPLACED_BY_ASSIGN: &str = "Object.assign(Array.prototype, { findIndex: function (f) { return f(0); } }); console.log([0].findIndex((v) => !!v));";

const FILTER_REPLACED_INSIDE_A_FUNCTION: &str = "function patch() { Array.prototype.filter = function (f) { return f(0); }; } function read() { return [0].filter((v) => !!v); } patch(); console.log(read());";

#[test]
fn an_array_method_patched_indirectly_keeps_the_double_not_by_default() {
    for source in [
        FILTER_REDEFINED,
        FIND_REPLACED_THROUGH_AN_ALIAS,
        SOME_REPLACED_THROUGH_THE_GLOBAL_OBJECT,
        EVERY_REPLACED_UNDER_A_COMPUTED_KEY,
        FIND_INDEX_REPLACED_BY_ASSIGN,
        FILTER_REPLACED_INSIDE_A_FUNCTION,
    ] {
        assert_callback_keeps_its_double_not(source);
    }
}

#[test]
fn a_replaced_array_method_is_assumed_away_under_unsafe() {
    let output = unsafe_lane(
        "Array.prototype.filter = function (f) { return f(0); }; console.log([0].filter((v) => !!v));",
    );
    assert!(output.contains("[0].filter((v) => v)"), "{output}");
}

const UNPATCHED_PREDICATES: [(&str, &str); 5] = [
    (
        "console.log([0, 1, 2].filter((v) => !!v));",
        "[0, 1, 2].filter((v) => v)",
    ),
    (
        r#"console.log(["a", ""].some((v) => !!v), [1, 0].every((v) => !!v));"#,
        r#"["a", ""].some((v) => v), [1, 0].every((v) => v)"#,
    ),
    (
        "Array.prototype.last = function () { return this[this.length - 1]; }; console.log([0, 3].filter((v) => !!v).last());",
        "[0, 3].filter((v) => v)",
    ),
    (
        "var list = Array.from({ length: 2 }, function (_, i) { return i; }); console.log([0, 1].find((v) => !!v), list.length);",
        "[0, 1].find((v) => v)",
    ),
    (
        r#"function Bag() {} Bag.prototype.filter = function () { return "own"; }; console.log([0, 1].findIndex((v) => !!v), new Bag().filter());"#,
        "[0, 1].findIndex((v) => v)",
    ),
];

#[test]
fn a_program_that_leaves_array_methods_alone_still_loses_the_double_not_by_default() {
    for (source, expected) in UNPATCHED_PREDICATES {
        let output = default_lane(source);
        assert!(output.contains(expected), "{source}\n{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_double_not_in_a_test_position_goes_whatever_the_program_patches() {
    let source = "Array.prototype.filter = function (f) { return f(0); }; if (!!globalThis) { console.log(!!1 ? 1 : 2); }";
    let output = default_lane(source);
    assert!(output.contains("if (globalThis)"), "{output}");
    untwist_testing::node::assert_node_agrees(source, &output);
}

const FILTER_REACHED_THROUGH_VALUES: [&str; 8] = [
    "[].__proto__.filter = function (f) { return f(0); }; console.log([0].filter((v) => !!v));",
    "[].constructor.prototype.filter = function (f) { return f(0); }; console.log([0].filter((v) => !!v));",
    "Object.getPrototypeOf([]).filter = function (f) { return f(0); }; console.log([0].filter((v) => !!v));",
    "(function (A) { A.prototype.filter = function (f) { return f(0); }; })([].constructor); console.log([0].filter((v) => !!v));",
    r#"Object.defineProperty([].__proto__, "filter", { value: function (f) { return f(0); } }); console.log([0].filter((v) => !!v));"#,
    r#"[]["__proto__"]["filter"] = function (f) { return f(0); }; console.log([0].filter((v) => !!v));"#,
    "var p = Reflect.getPrototypeOf([]); p.filter = function (f) { return f(0); }; console.log([0].filter((v) => !!v));",
    r#"Function("Array.prototype.filter = function (f) { return f(0); }")(); console.log([0].filter((v) => !!v));"#,
];

#[test]
fn an_array_method_patched_through_a_value_keeps_the_double_not_by_default() {
    for source in FILTER_REACHED_THROUGH_VALUES {
        assert_callback_keeps_its_double_not(source);
    }
}
