use untwist_core::pass::{PassCtor, PassOptions};
use untwist_testing::{transform_with, Allocator};
use untwist_unminify::{lexical_ctors, prepare_ctors, transpile_ctors, unminify_ctors};

fn full<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors
}

fn run(src: &str) -> String {
    let opts = PassOptions {
        unsafe_transforms: true,
        ..PassOptions::default()
    };
    let alloc = Allocator::default();
    let prepared = transform_with(&alloc, src, &prepare_ctors(), opts.clone());
    let alloc2 = Allocator::default();
    transform_with(&alloc2, &prepared, &full(), opts)
}

#[test]
fn split_for_loop_vars_then_optional_chaining() {
    let out = run("for (var v, d = [1], b = 0x0; b < d.length; b++) {}\nif ((null === (v = null) || void 0x0 === v ? void 0x0 : v['length'])) {}");
    assert_eq!(
        out,
        "for (var d = [1], b = 0; b < d.length; b++) {}\nif (null?.length) {}"
    );
}

#[test]
fn yoda_then_typeof_undefined_converge_without_requeue() {
    assert_eq!(
        run("if (\"u\" > typeof x) y();"),
        "if (typeof x !== \"undefined\") {\n  y();\n}"
    );
}

#[test]
fn prepare_then_unminify_matches_reference_ordering() {
    assert_eq!(
        run("var a=!0,b=!1;x(),y();console[\"log\"](a,b)"),
        "var a = true;\nvar b = false;\nx();\ny();\nconsole.log(a, b);"
    );
}

const UNMINIFY_ORDER: &[&str] = &[
    "block-statements",
    "computed-properties",
    "for-to-while",
    "infinity",
    "invert-boolean-logic",
    "json-parse",
    "logical-to-if",
    "merge-else-if",
    "merge-strings",
    "number-expressions",
    "remove-double-not",
    "sequence",
    "split-for-loop-vars",
    "split-variable-declarations",
    "string-literal-in-template-literal",
    "ternary-to-if",
    "truncate-number-literal",
    "typeof-undefined",
    "unary-expressions",
    "unminify-booleans",
    "void-to-undefined",
    "yoda",
    "unwrap-comma-call",
];

const LEXICAL_ORDER: &[&str] = &[
    "let-to-const",
    "var-to-let",
    "function-to-arrow",
    "merge-member-declarations",
];

const TRANSPILE_ORDER: &[&str] = &[
    "default-parameters",
    "logical-assignments",
    "nullish-coalescing",
    "nullish-coalescing-assignment",
    "optional-chaining",
    "template-literals",
];

const PREPARE_ORDER: &[&str] = &[
    "block-statements",
    "sequence",
    "split-variable-declarations",
];

#[test]
fn unminify_order_is_reference_barrel_order() {
    let names: Vec<&str> = unminify_ctors().iter().map(|c| c().name()).collect();
    assert_eq!(names, UNMINIFY_ORDER);
}

#[test]
fn the_lexical_group_runs_let_to_const_then_the_untranspile_passes() {
    let names: Vec<&str> = lexical_ctors().iter().map(|c| c().name()).collect();
    assert_eq!(names, LEXICAL_ORDER);
}

#[test]
fn transpile_order_is_reference_barrel_order() {
    let names: Vec<&str> = transpile_ctors().iter().map(|c| c().name()).collect();
    assert_eq!(names, TRANSPILE_ORDER);
}

#[test]
fn prepare_order_is_reference_barrel_order() {
    let names: Vec<&str> = prepare_ctors().iter().map(|c| c().name()).collect();
    assert_eq!(names, PREPARE_ORDER);
}

#[test]
fn block_statements_and_merge_else_if_converge() {
    assert_eq!(
        run("if (x) {} else { if (y) {} }"),
        "if (x) {} else if (y) {}"
    );
}

const FIXTURE_INPUTS: &[&str] = &[
    "if (x) {\n} else {\n  if (y) {}\n}",
    "if (a) b(), c();",
    "for (var i = 0, j = 1, k = 2; k < 5; k++) {}",
    "const a = 1, b = 2, c = 3;",
    "a ? b() : c();",
    "x && y && z();",
    "a === null || a === undefined ? undefined : a.b;",
    "a !== null && a !== undefined ? a : b;",
    "x || (x = y);",
    "function f() {\n  var e = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : \"foo\";\n}",
    "\"\".concat(1);",
    "\"red\" === color;",
];

#[test]
fn full_pipeline_converges_on_all_pass_fixture_inputs() {
    for input in FIXTURE_INPUTS {
        run(input);
    }
}
