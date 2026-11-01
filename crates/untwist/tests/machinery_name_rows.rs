use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, patched_after_rotator, Mode};

const ALWAYS: &str = "1 === 1";

const OBSERVING: [(&str, &str); 19] = [
    ("keys length", "console.log(Object.keys(this).length);"),
    ("globalThis keys", "console.log(Object.keys(globalThis).join());"),
    ("alias keys", "var g = this; console.log(Object.keys(g).length);"),
    ("own keys", "console.log(Reflect.ownKeys(this).length > 0 && Object.keys(this).length);"),
    ("returned this", "function G() { return this; } console.log(Object.keys(G()).length);"),
    ("filtered names", "console.log(Object.getOwnPropertyNames(this).filter(function (k) { return k.length === 1; }).join());"),
    ("name lookup", "console.log(Object.getOwnPropertyNames(this).indexOf('a') >= 0);"),
    ("in operator", "console.log('a' in this, 'b' in this);"),
    ("keys index", "console.log(Object.keys(this).indexOf('b'));"),
    ("for in", "for (var k in this) { if (k.length === 1) console.log(k); }"),
    ("entries", "console.log(Object.entries(this).length);"),
    ("literal reads", "console.log(typeof this.a, typeof this['b']);"),
    ("iife this", "var o = (function () { return this; })(); console.log(Object.keys(o).join());"),
    ("names includes", "console.log(Object.getOwnPropertyNames(globalThis).includes('a'));"),
    ("hasOwnProperty call", "console.log(Object.prototype.hasOwnProperty.call(this, 'a'));"),
    ("own keys index", "console.log(Reflect.ownKeys(globalThis).indexOf('b') >= 0);"),
    ("hasOwnProperty method", "console.log(this.hasOwnProperty('a'));"),
    ("boxed keys", "console.log(Object.keys(Object(this)).length);"),
    ("printed alias", "var q = [1]; q = this; console.log(Object.keys(q).length);"),
];

const LENGTHS: [(&str, &str); 6] = [
    ("array local", "var q = [1, [2]]; console.log(q.length);"),
    ("string local", "var q = 'abc'; console.log(q.length);"),
    (
        "template local",
        "var q = `x${typeof q}`; console.log(q.length);",
    ),
    ("array literal", "console.log([0, 1].length, 'ab'.length);"),
    (
        "written length",
        "var q = [1]; q.length = 0; console.log(q.length);",
    ),
    (
        "pushed local",
        "var q = []; q.push(1); console.log(q.length);",
    ),
];

const LENGTHS_KEPT: [(&str, &str); 5] = [
    (
        "printed local",
        "var q = [1]; console.log(q.length, Object.keys(q).length);",
    ),
    (
        "reassigned local",
        "var q = [1]; q = this; console.log(q.length, Object.keys(q).length);",
    ),
    (
        "escaping local",
        "var q = [1]; function f() { return q; } console.log(f().length);",
    ),
    ("global local", "var q = this; console.log(q.length);"),
    (
        "redeclared local",
        "var q = [1]; var q; console.log(q.length);",
    ),
];

const STAMP_SPELLED: [(&str, &str); 7] = [
    ("free array stamp", "console.log(typeof __STRING_ARRAY__);"),
    (
        "free decoder stamp beside declared ones",
        "function __DECODE_0__() { return 'user0'; } function __STRING_ARRAY_0__() { return 'u'; } console.log(__DECODE_0__(), __STRING_ARRAY_0__(), typeof __DECODE_1__);",
    ),
    ("free decoder stamp", "console.log(typeof __DECODE_0__);"),
    (
        "free stamps together",
        "console.log(typeof __STRING_ARRAY__, typeof __DECODE_0__, typeof __STRING_ARRAY_1__);",
    ),
    (
        "assigned free stamp",
        "__STRING_ARRAY_1__ = 5; console.log(__STRING_ARRAY_1__, typeof __STRING_ARRAY__);",
    ),
    ("declared array stamp", "var __STRING_ARRAY__ = 3; console.log(__STRING_ARRAY__);"),
    (
        "free stamp in a function",
        "function f() { return typeof __DECODE_0__ + typeof __STRING_ARRAY__; } console.log(f());",
    ),
];

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn programs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, patch)| (label.to_string(), patched_after_rotator(ALWAYS, patch)))
        .collect()
}

fn agrees_with_node(rows: &[(&str, &str)]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| {
        node_rows::with_machinery_restored(source, &transformed(&unsafe_transforms, source))
    };
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs(rows), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn machinery_left(rows: &[(&str, &str)]) -> Vec<(String, bool, bool)> {
    programs(rows)
        .into_iter()
        .map(|(label, source)| {
            let code = transformed(&Options::default(), &source);
            let kept = code.contains("function a()");
            let inlined = code.contains("console.log(\"log!\");");
            (label, kept, inlined)
        })
        .collect()
}

#[test]
fn decoder_machinery_observed_through_global_names_runs_like_node() {
    agrees_with_node(&OBSERVING);
}

#[test]
fn decoder_machinery_is_not_removed_under_name_observation() {
    let removed: Vec<String> = machinery_left(&OBSERVING)
        .into_iter()
        .filter(|(_, kept, _)| !kept)
        .map(|(label, _, _)| label)
        .collect();
    assert!(removed.is_empty(), "{}", removed.join("\n"));
}

#[test]
fn a_length_read_on_a_fresh_sequence_local_runs_like_node() {
    agrees_with_node(&LENGTHS);
    agrees_with_node(&LENGTHS_KEPT);
}

#[test]
fn a_length_read_on_a_fresh_sequence_local_does_not_observe_global_names() {
    let kept: Vec<String> = machinery_left(&LENGTHS)
        .into_iter()
        .filter(|(_, kept, inlined)| *kept || !inlined)
        .map(|(label, kept, inlined)| format!("{label}: kept {kept}, inlined {inlined}"))
        .collect();
    assert!(kept.is_empty(), "{}", kept.join("\n"));
}

#[test]
fn an_input_identifier_spelled_like_a_stamp_keeps_its_value() {
    agrees_with_node(&STAMP_SPELLED);
}
