use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_testing::node::assert_node_agrees;

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/loose-for-of")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn run(source: &str, unsafe_transforms: bool) -> String {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn lanes(name: &str) -> (String, String, String) {
    let source = read(name);
    let safe = run(&source, false);
    let unsafe_ = run(&source, true);
    (source, safe, unsafe_)
}

fn assert_has(code: &str, parts: &[&str]) {
    for part in parts {
        assert!(code.contains(part), "missing {part:?} in:\n{code}");
    }
}

fn assert_lacks(code: &str, parts: &[&str]) {
    for part in parts {
        assert!(!code.contains(part), "unexpected {part:?} in:\n{code}");
    }
}

#[test]
fn loose_loops_over_arrays_strings_and_fresh_collections_become_for_of() {
    let (source, safe, unsafe_) = lanes("for-of-loose.js");
    assert_has(
        &unsafe_,
        &[
            "for (const key of Object.keys({",
            " of Object.entries({",
            "for (const ch of \"a-b-c\".split(\"-\"))",
            " of \"hé\")",
            "for (const pair of new Map([",
            "outer: for (const row of [[1, 2], [3, 4]])",
            "for (const g of grow)",
            "for (last of Array.from(new Set([5, 6])))",
            "for (const h of holes)",
            " of [7, 8])",
        ],
    );
    assert_has(
        &unsafe_,
        &[
            "_createForOfIteratorHelperLoose(counted(\"c1\"",
            "_createForOfIteratorHelperLoose(gen())",
            "_createForOfIteratorHelperLoose(row)",
            "_createForOfIteratorHelperLoose(items)",
            "_list = list",
        ],
    );
    assert_lacks(&unsafe_, &["_loop"]);
    assert_has(
        &safe,
        &[
            "_createForOfIteratorHelperLoose(\"hé\")",
            "_i < _list.length",
        ],
    );
    assert_node_agrees(&source, &safe);
    assert_node_agrees(&source, &unsafe_);
}

#[test]
fn babel_array_loops_of_the_strict_transform_become_for_of() {
    let (source, safe, unsafe_) = lanes("for-of-index.js");
    assert_has(
        &unsafe_,
        &[
            "for (const key of Object.keys({",
            "for (const g of grow)",
            "for (const h of holes)",
        ],
    );
    assert_has(&safe, &["_i2 < _Object$keys.length"]);
    assert_node_agrees(&source, &safe);
    assert_node_agrees(&source, &unsafe_);
}

#[test]
fn arrays_that_escape_or_change_keep_their_loops() {
    let (source, safe, unsafe_) = lanes("for-of-loose-guards.js");
    assert_has(
        &unsafe_,
        &[
            "_i < _a.length",
            "_i2 < _b.length",
            "_i3 < _c.length",
            "_createForOfIteratorHelperLoose(d)",
            "_createForOfIteratorHelperLoose(e)",
            " of f) {",
        ],
    );
    assert_node_agrees(&source, &safe);
    assert_node_agrees(&source, &unsafe_);
}

#[test]
fn math_pow_with_a_plain_operand_becomes_the_operator() {
    let (source, safe, unsafe_) = lanes("exponent-guards.js");
    for code in [&safe, &unsafe_] {
        assert_has(
            code,
            &[
                "String(Math.pow(a, b))",
                "sq = n ** 2;",
                "2 ** n",
                "__pow(n, n)",
                "return __pow(v, 2);",
                "x **= 2;",
                "o.v **= 3;",
                "(-x) ** 2",
                "x ** -1",
            ],
        );
        assert_lacks(code, &["Math.pow(x", "Math[\"pow\"]"]);
    }
    assert_node_agrees(&source, &safe);
    assert_node_agrees(&source, &unsafe_);
}

#[test]
fn the_default_lane_keeps_a_patched_math_pow() {
    let (source, safe, unsafe_) = lanes("exponent-patched.js");
    assert_has(&safe, &["Math.pow(2, 10)", "Math.pow(x, 2)"]);
    assert_lacks(&unsafe_, &["Math.pow(x, 2)"]);
    assert_node_agrees(&source, &safe);
}
