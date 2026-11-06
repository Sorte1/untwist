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

const MEMOIZE: &str = "function memoize(fn) {\nconst cache = new Map();\nreturn (key) => {\nif (!cache.has(key)) {\ncache.set(key, fn(key));\n}\nreturn cache.get(key);\n};\n}\nconst sq = memoize((n) => n * n);\nconsole.log(sq(4), sq(4));";

const FRESH_SHAPES: [(&str, &str); 3] = [
    ("a memoize cache built from new Map", MEMOIZE),
    (
        "a WeakMap side table",
        "const seen = new WeakMap();\nfunction mark(o, f) {\nseen.set(o, f(o));\nreturn seen.get(o);\n}\nconsole.log(mark(globalThis, (g) => typeof g));",
    ),
    (
        "a set call in a short-circuit statement",
        "const m = new Map();\nfunction put(k, f) {\nm.has(k) || m.set(k, f(k));\nreturn m.get(k);\n}\nconsole.log(put(globalThis, (g) => typeof g));",
    ),
];

const PUT_GLOBAL: &str =
    "function put(k, v) {\nm.set(k, v);\n}\nput(globalThis, String.fromCharCode(98));";

fn replaced_shapes() -> [(&'static str, String); 7] {
    [
        (
            "a replaced Map constructor",
            format!("Map = function () {{ return Reflect; }};\n{MEMOIZE}"),
        ),
        (
            "a replaced Map.prototype.set",
            format!("Map.prototype.set = Reflect.set;\n{MEMOIZE}"),
        ),
        (
            "an own set written on the map",
            format!("const m = new Map();\nm.set = Reflect.set;\n{PUT_GLOBAL}"),
        ),
        (
            "a global written by a key that may be Map",
            format!(
                "globalThis[String.fromCharCode(77, 97, 112)] = function () {{ return Reflect; }};\n{MEMOIZE}"
            ),
        ),
        (
            "a map passed to unseen code",
            format!("const m = new Map();\nhook(m);\n{PUT_GLOBAL}"),
        ),
        (
            "a map that leaves through the value of set",
            format!("const m = new Map();\nhook(m.set(\"x\", 1));\n{PUT_GLOBAL}"),
        ),
        (
            "a replaced Map.prototype.get",
            "const m = new Map();\nMap.prototype.get = Reflect.get;\nfunction look(o, f) {\nreturn m.get(o, f(o));\n}\nconsole.log(typeof look(globalThis, () => String.fromCharCode(98)));".to_string(),
        ),
    ]
}

#[test]
fn a_set_call_on_a_fresh_native_map_decodes_in_the_safe_lane() {
    for (label, body) in FRESH_SHAPES {
        let source = patched_after_rotator("true", body);
        assert!(decoded(&source), "{label}:\n{}", safe_lane(&source).code());
    }
}

#[test]
fn a_set_call_on_a_map_that_may_not_be_native_still_refuses() {
    for (label, body) in replaced_shapes() {
        let source = patched_after_rotator("true", &body);
        assert!(!decoded(&source), "{label}:\n{}", safe_lane(&source).code());
    }
}

#[test]
fn every_fresh_map_shape_prints_what_node_prints_after_decoding() {
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
