use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const DIR: &str = "fixtures/pure-calls";

const HELLO: &str = "console.log(\"hello\", \"worold\");";

const FOLDED: &[(&str, &str)] = &[
    ("arrow-block.js", HELLO),
    ("arrow-expression.js", HELLO),
    ("function-declaration.js", HELLO),
    ("hoisted-const-array.js", HELLO),
    ("offset-index.js", HELLO),
    ("object-literal.js", HELLO),
    ("ternary-concat.js", "console.log(\"xa\", \"yb!\");"),
    (
        "nested-scope.js",
        "function main() {\n  console.log(\"hello\", \"worold\");\n}\nmain();",
    ),
];

const KEPT: &[(&str, &[&str], &[&str])] = &[
    ("kept-reassigned.js", &["dec(0)"], &["dec(0)"]),
    ("kept-mutates.js", &["dec(0)"], &["dec(0)"]),
    ("kept-reads-global.js", &["dec(0)"], &["dec(0)"]),
    (
        "kept-non-literal-arg.js",
        &["dec(Date.now() % 1)"],
        &["dec(Date.now() % 1)"],
    ),
    (
        "kept-call-before-let.js",
        &["early = dec(0)", "console.log(early, \"b\")"],
        &["early = dec(0)", "console.log(early, \"b\")"],
    ),
    (
        "kept-call-in-hoisted-function.js",
        &["return dec(1)"],
        &["return dec(1)"],
    ),
    (
        "kept-passed-elsewhere.js",
        &["console.log(dec(0), table.dec(1))"],
        &["console.log(\"a\", table.dec(1))"],
    ),
    ("kept-array-passed.js", &["dec(0)"], &["dec(0)"]),
    (
        "kept-out-of-range.js",
        &["console.log(dec(5), \"a\")"],
        &["console.log(dec(5), \"a\")"],
    ),
    ("kept-switch-case.js", &["dec(0)"], &["dec(0)"]),
    (
        "kept-global-rebind.js",
        &["console.log(dec(0))"],
        &["console.log(\"a\")"],
    ),
];

const UNSAFE_ASSUMES_NO_GLOBAL_REBIND: &str = "kept-global-rebind.js";

fn run(source: &str, unsafe_transforms: bool) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn default_lane(source: &str) -> String {
    run(source, false)
}

fn unsafe_lane(source: &str) -> String {
    run(source, true)
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = js_files_in(DIR);
    assert_eq!(fixtures.len(), FOLDED.len() + KEPT.len());
    fixtures
}

fn fixture(name: &str) -> String {
    fixtures()
        .into_iter()
        .find(|(label, _)| label.ends_with(&format!("/{name}")))
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("missing fixture {DIR}/{name}"))
}

#[test]
fn every_folded_fixture_prints_the_folded_call_in_the_default_lane() {
    for (name, expected) in FOLDED {
        let code = default_lane(&fixture(name));
        assert_eq!(code.trim(), *expected, "{name}");
    }
}

#[test]
fn every_folded_fixture_prints_the_folded_call_in_the_unsafe_lane() {
    for (name, expected) in FOLDED {
        let code = unsafe_lane(&fixture(name));
        assert_eq!(code.trim(), *expected, "{name}");
    }
}

#[test]
fn every_kept_fixture_keeps_the_refused_calls_in_the_default_lane() {
    for (name, wanted, _) in KEPT {
        let code = default_lane(&fixture(name));
        for piece in *wanted {
            assert!(code.contains(piece), "{name} lacks {piece:?}:\n{code}");
        }
    }
}

#[test]
fn every_kept_fixture_keeps_the_refused_calls_in_the_unsafe_lane() {
    for (name, _, wanted) in KEPT {
        let code = unsafe_lane(&fixture(name));
        for piece in *wanted {
            assert!(code.contains(piece), "{name} lacks {piece:?}:\n{code}");
        }
    }
}

#[test]
fn every_fixture_prints_what_node_prints_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 1] = [("default", &default_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_unsafe_lane_differs_from_node_only_where_a_script_rebinds_the_function_globally() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    let unexplained: Vec<&String> = wrong
        .iter()
        .filter(|row| !row.contains(UNSAFE_ASSUMES_NO_GLOBAL_REBIND))
        .collect();
    assert!(unexplained.is_empty(), "{unexplained:?}");
    assert_eq!(wrong.len(), 1, "{wrong:?}");
}
