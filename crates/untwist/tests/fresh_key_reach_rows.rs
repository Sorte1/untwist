use untwist::{Options, Output, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, patched_after_rotator, Mode};

const DECODER_CALL: &str = "console.log(b(";

fn safe_lane(source: &str) -> Output {
    Pipeline::new(Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn decoded(source: &str) -> bool {
    !safe_lane(source).code().contains(DECODER_CALL)
}

const FRESH_SHAPES: [(&str, &str); 5] = [
    (
        "a rest of a local literal",
        "const e = { c: 9, p: 1 };\nconst { c, ...rest } = e;\nconsole.log(c, rest.p);",
    ),
    (
        "a rest of a literal that spreads another literal",
        "const i = { p: 1 };\nconst e = { c: 9, ...i };\nconst { c, ...rest } = e;\nconsole.log(c, rest.p, Object.keys(rest).length);",
    ),
    (
        "a rest assigned from a local literal",
        "const e = { c: 9, p: 1 };\nlet c, rest;\n({ c, ...rest } = e);\nconsole.log(c, rest.p);",
    ),
    (
        "a key bound to a fresh Symbol",
        "const t = Symbol(\"tag\");\nconst g = { v: \"s\" };\ng[t] = 1;\nconsole.log(g[t], g.v);",
    ),
    (
        "a computed property keyed by a fresh Symbol",
        "const t = Symbol();\nconst g = { [t]: 1, v: \"s\" };\nconsole.log(g[t], g.v);",
    ),
];

const GLOBAL_SHAPES: [(&str, &str); 7] = [
    (
        "a rest of the global object",
        "const { console: c, ...rest } = globalThis;\nconsole.log(typeof rest.b);",
    ),
    (
        "a rest of a top-level this alias",
        "var g = this;\nconst { ...rest } = g;\nconsole.log(typeof rest.b);",
    ),
    (
        "a rest of a literal that spreads the global object",
        "const e = { c: 1, ...globalThis };\nconst { c, ...rest } = e;\nconsole.log(typeof rest.b);",
    ),
    (
        "a rest of a literal written by an unknown key",
        "const e = { c: 1 };\ne[String.fromCharCode(98)] = 2;\nconst { c, ...rest } = e;\nconsole.log(typeof rest.b);",
    ),
    (
        "a key from a shadowed Symbol",
        "const Symbol = String;\nconst t = Symbol(\"b\");\nconsole.log(typeof globalThis[t]);",
    ),
    (
        "a key from a replaced Symbol.for",
        "Symbol.for = String;\nconst t = Symbol.for(\"b\");\nconsole.log(typeof globalThis[t]);",
    ),
    (
        "a key from a replaced Symbol",
        "globalThis.Symbol = String;\nconst t = Symbol(\"b\");\nconsole.log(typeof globalThis[t]);",
    ),
];

#[test]
fn a_rest_or_symbol_key_on_a_fresh_object_decodes_in_the_safe_lane() {
    for (label, body) in FRESH_SHAPES {
        let source = patched_after_rotator("true", body);
        assert!(decoded(&source), "{label}:\n{}", safe_lane(&source).code());
    }
}

#[test]
fn a_rest_or_key_that_may_reach_the_global_object_still_refuses() {
    for (label, body) in GLOBAL_SHAPES {
        let source = patched_after_rotator("true", body);
        assert!(!decoded(&source), "{label}:\n{}", safe_lane(&source).code());
    }
}

#[test]
fn every_fresh_shape_prints_what_node_prints_after_decoding() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let fixtures: Vec<(String, String)> = FRESH_SHAPES
        .iter()
        .map(|(label, body)| (label.to_string(), patched_after_rotator("true", body)))
        .collect();
    let lane = |source: &str| safe_lane(source).code().to_string();
    let runs: [Mode<'_>; 1] = [("safe", &lane)];
    let wrong = mismatches(&node, &fixtures, &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
