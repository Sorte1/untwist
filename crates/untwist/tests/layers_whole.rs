use std::path::{Path, PathBuf};

use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::layers::notes::{
    LAYER_BAILED, LAYER_CAP, LAYER_CYCLE, LAYER_FUEL, LAYER_INJECTION, LAYER_NEEDS_UNSAFE,
    LAYER_NOT_A_STRING, LAYER_PEELED, LAYER_SHAPE, LAYER_TOO_DEEP, LAYER_UNPARSABLE,
};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const KINDS: &[&str] = &["eval/", "function/"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/layers")
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn disabled(options: Options) -> Options {
    Options {
        disabled_passes: vec!["layers"],
        ..options
    }
}

fn reason_text(name: &str) -> &'static str {
    match name {
        "LAYER_BAILED" => LAYER_BAILED,
        "LAYER_CAP" => LAYER_CAP,
        "LAYER_CYCLE" => LAYER_CYCLE,
        "LAYER_FUEL" => LAYER_FUEL,
        "LAYER_INJECTION" => LAYER_INJECTION,
        "LAYER_NEEDS_UNSAFE" => LAYER_NEEDS_UNSAFE,
        "LAYER_NOT_A_STRING" => LAYER_NOT_A_STRING,
        "LAYER_SHAPE" => LAYER_SHAPE,
        "LAYER_TOO_DEEP" => LAYER_TOO_DEEP,
        "LAYER_UNPARSABLE" => LAYER_UNPARSABLE,
        other => panic!("unknown reason {other}"),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    peels: usize,
    refused: Option<&'static str>,
}

fn expected(expect: &str, unsafe_transforms: bool) -> Outcome {
    let base = expect.split(':').collect::<Vec<_>>();
    if base[0] == "needs-unsafe" {
        return match unsafe_transforms {
            true => Outcome {
                peels: 1,
                refused: None,
            },
            false => Outcome {
                peels: 0,
                refused: Some(LAYER_NEEDS_UNSAFE),
            },
        };
    }
    let mut outcome = Outcome {
        peels: 0,
        refused: None,
    };
    for part in expect.split(',') {
        match part.split_once(':') {
            Some(("refuse", reason)) => outcome.refused = Some(reason_text(reason)),
            _ if part == "peel" => outcome.peels += 1,
            _ => {
                let count = part
                    .strip_prefix("peel*")
                    .and_then(|n| n.parse::<usize>().ok());
                outcome.peels += count.unwrap_or_else(|| panic!("outcome {part}"));
            }
        }
    }
    outcome
}

fn observed(out: &Output) -> Outcome {
    let peels = out
        .stages
        .iter()
        .filter(|stage| stage.stage == "layers")
        .count();
    let refused = out
        .notes
        .iter()
        .filter(|note| note.pass == "layers" && !note.message.starts_with(LAYER_PEELED))
        .map(|note| note.message.split("; layer ").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert!(refused.len() <= 1, "{refused:?}");
    let refused = refused.first().map(|text| {
        [
            LAYER_BAILED,
            LAYER_CAP,
            LAYER_CYCLE,
            LAYER_FUEL,
            LAYER_INJECTION,
            LAYER_NEEDS_UNSAFE,
            LAYER_NOT_A_STRING,
            LAYER_SHAPE,
            LAYER_TOO_DEEP,
            LAYER_UNPARSABLE,
        ]
        .into_iter()
        .find(|known| known == text)
        .unwrap_or_else(|| panic!("uncatalogued note {text}"))
    });
    Outcome { peels, refused }
}

fn rows() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(root().join("MANIFEST")).unwrap_or_default();
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [rel, _, _, expect, _] = fields[..] else {
                return None;
            };
            KINDS
                .iter()
                .any(|kind| rel.starts_with(kind))
                .then(|| (rel.to_string(), expect.to_string()))
        })
        .collect()
}

#[test]
fn every_eval_and_function_fixture_has_its_manifest_outcome_in_both_modes() {
    let rows = rows();
    assert!(rows.len() >= 33, "{} rows", rows.len());
    let mut wrong = Vec::new();
    for (rel, expect) in &rows {
        let source = std::fs::read_to_string(root().join(rel)).unwrap_or_default();
        for unsafe_transforms in [false, true] {
            let out = Pipeline::new(options(unsafe_transforms)).run(&source);
            let want = expected(expect, unsafe_transforms);
            let got = observed(&out);
            if got != want {
                wrong.push(format!(
                    "{rel} unsafe={unsafe_transforms}: {got:?}, want {want:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_fixture_that_peels_nothing_prints_as_the_run_with_layers_disabled() {
    for (rel, _) in rows() {
        let source = std::fs::read_to_string(root().join(&rel)).unwrap_or_default();
        for unsafe_transforms in [false, true] {
            let out = Pipeline::new(options(unsafe_transforms)).run(&source);
            if observed(&out).peels > 0 {
                continue;
            }
            let kept = Pipeline::new(disabled(options(unsafe_transforms))).run(&source);
            assert_eq!(out.code(), kept.code(), "{rel} unsafe={unsafe_transforms}");
        }
    }
}

fn peeled(source: &str, unsafe_transforms: bool) -> Outcome {
    observed(&Pipeline::new(options(unsafe_transforms)).run(source))
}

fn refused(reason: &'static str) -> Outcome {
    Outcome {
        peels: 0,
        refused: Some(reason),
    }
}

#[test]
fn each_whole_program_eval_condition_refuses_its_layer() {
    let rows = [
        (
            "eval((function () { var eval; return 'console.log(1)'; })());",
            Some(refused(LAYER_SHAPE)),
        ),
        (
            "eval(atob((function (atob) {}, 'Y29uc29sZS5sb2coMSk=')));",
            Some(refused(LAYER_SHAPE)),
        ),
        (
            "eval(atob('Y29uc29sZS5sb2coMSk='));",
            Some(Outcome {
                peels: 1,
                refused: None,
            }),
        ),
        ("eval('console.log(1)', f());", Some(refused(LAYER_SHAPE))),
        (
            "'use strict'; eval('console.log(1)');",
            Some(refused(LAYER_SHAPE)),
        ),
        ("eval(...['console.log(1)']);", Some(refused(LAYER_SHAPE))),
        (
            "eval((atob = String, 'console.log(1)'));",
            Some(refused(LAYER_SHAPE)),
        ),
        ("eval(location.hash);", Some(refused(LAYER_BAILED))),
        ("eval(Date());", Some(refused(LAYER_BAILED))),
        ("eval(f());", Some(refused(LAYER_BAILED))),
        ("eval(1);", Some(refused(LAYER_NOT_A_STRING))),
        ("eval('return 1');", Some(refused(LAYER_UNPARSABLE))),
        (
            "eval('let a = 1; console.log(a)');",
            Some(refused(LAYER_NEEDS_UNSAFE)),
        ),
        (
            "eval('class A {} console.log(A)');",
            Some(refused(LAYER_NEEDS_UNSAFE)),
        ),
        (
            "eval('\"use strict\"; var a = 1;');",
            Some(refused(LAYER_NEEDS_UNSAFE)),
        ),
    ];
    for (source, want) in rows {
        let want = want.unwrap_or(Outcome {
            peels: 0,
            refused: None,
        });
        assert_eq!(peeled(source, false), want, "{source}");
    }
}

#[test]
fn each_scanned_form_keeps_a_declaring_payload_for_unsafe() {
    let observers = [
        "delete x",
        "delete globalThis.x",
        "delete window['x']",
        "delete self.x",
        "delete this.x",
        "var g = globalThis; delete g.x",
        "delete [globalThis][0].x",
        "delete o?.x",
        "var o = {}; Object.defineProperty(o, 'x', {}); delete o.x;",
        "var g = globalThis; Object.defineProperty(g, 'x', {})",
        "Object.getOwnPropertyDescriptor(globalThis, 'x')",
        "Object.getOwnPropertyDescriptors(o)",
        "Reflect.getOwnPropertyDescriptor(o, 'x')",
        "Reflect.deleteProperty(globalThis, 'x')",
        "Object.defineProperty(globalThis, 'x', {})",
        "Object['freeze'](window)",
        "Object.seal(this)",
        "Object.preventExtensions(globalThis)",
        "with (o) {}",
        "eval('delete x')",
        "Function('delete x')()",
        "var O = Object; O.defineProperty(globalThis, 'x', {})",
        "var R = Reflect; R.deleteProperty(globalThis, 'x')",
        "var d = Object.defineProperty; d(globalThis, 'x', {})",
        "Object.defineProperty.call(null, globalThis, 'x', {})",
        "(0, Object.defineProperty)(globalThis, 'x', {})",
        "var { defineProperty: d } = Object; d(globalThis, 'x', {})",
        "var O = window.Object; O.freeze(this)",
        "var C = ({}).constructor; C['define' + 'Property'](globalThis, 'x', {})",
        "Object['define' + 'Property'](globalThis, 'x', {})",
        "var k = 'defineProperty'; globalThis.Object[k](globalThis, 'x', {})",
        "var g = globalThis; g[k](g, 'x', {})",
        "var { constructor: C } = {}; C[k](globalThis, 'x', {})",
        "o.getOwnPropertyDescriptor(globalThis, 'x')",
        "var g = this; g[k](g, 'x', {})",
        "var g = (function () { return this; })(); g[k](g, 'x', {})",
        "var { [k]: C } = {}; C[j](globalThis, 'x', {})",
        "setTimeout('delete x')",
        "Object.defineProperty.length",
    ];
    for observer in observers {
        let source = format!("eval({:?});", format!("var x = 1; {observer};"));
        assert_eq!(
            peeled(&source, false),
            refused(LAYER_NEEDS_UNSAFE),
            "{source}"
        );
        assert_eq!(peeled(&source, true).peels, 1, "{source}");
    }
    let clean = [
        "var x = 1; console.log(x);",
        "function f() {} f();",
        "if (a) { var x; }",
        "for (var i = 0; i < 1; i++) {}",
        "{ function g() {} }",
        "Object.defineProperty({}, 'x', {});",
        "delete f();",
        "var o = Object.keys({}); console.log(o.length, Object.prototype.toString.call(o));",
        "if (typeof window !== 'undefined' && 'x' in window) { window.y = 1; }",
        "var e = new Error('a'); console.log(e.constructor.name);",
        "function C() { this.a = 1; } C.prototype.constructor = C; new C();",
        "setTimeout(function () {}, 0);",
        "var o = {}; if (o.constructor === Object && o instanceof Object) {}",
        "var t = typeof globalThis;",
    ];
    for payload in clean {
        let source = format!("eval({payload:?});");
        assert_eq!(peeled(&source, false).peels, 1, "{source}");
    }
}

#[test]
fn a_function_layer_is_the_create_dynamic_function_source_called_with_the_arguments() {
    let out = Pipeline::new(options(false)).run("new Function('a', 'b', 'return a + b')(1, 2);");
    assert_eq!(observed(&out).peels, 1, "{:?}", out.notes);
    assert!(out.code().contains("anonymous"), "{}", out.code());
    let rows = [
        (
            "Function('}); f(); (function () {')();",
            refused(LAYER_INJECTION),
        ),
        (
            "Function('a) { f(); }, function (b', 'return 1')();",
            refused(LAYER_INJECTION),
        ),
        (
            "Function('a /*', '*/ return 1')();",
            refused(LAYER_INJECTION),
        ),
        (
            "Function('a = 1', '\"use strict\"; return a')();",
            refused(LAYER_INJECTION),
        ),
        (
            "Function('a', 'a', '\"use strict\"; return a')();",
            refused(LAYER_INJECTION),
        ),
        (
            "Function('return typeof anonymous')();",
            refused(LAYER_INJECTION),
        ),
        ("Function(f())();", refused(LAYER_BAILED)),
        (
            "Function?.('return 1')();",
            Outcome {
                peels: 0,
                refused: None,
            },
        ),
        (
            "Function('return 1')?.();",
            Outcome {
                peels: 0,
                refused: None,
            },
        ),
        (
            "var Function = 1; Function('x')();",
            Outcome {
                peels: 0,
                refused: None,
            },
        ),
    ];
    for (source, want) in rows {
        assert_eq!(peeled(source, false), want, "{source}");
    }
}

const GLOBAL_OBSERVERS: [(&str, &str); 7] = [
    (
        "a delete through an alias of the global object",
        "eval(\"var x = 1; var g = globalThis; console.log(delete g.x, typeof x);\");",
    ),
    (
        "a delete through an array holding the global object",
        "eval(\"var x = 1; console.log(delete [globalThis][0].x, typeof x);\");",
    ),
    (
        "a delete of a declared function through the global object",
        "eval(\"function f() { return 1; } console.log(f(), delete [globalThis][0].f, typeof f);\");",
    ),
    (
        "a redefinition through an alias of the global object",
        "eval(\"var x = 1; var g = globalThis; try { Object.defineProperty(g, 'x', { get: function () { return 2; } }); console.log(x); } catch (e) { console.log('threw ' + e.constructor.name); }\");",
    ),
    (
        "a redefinition through an alias of Object",
        "eval(\"var x = 1; var O = Object; try { O.defineProperty(globalThis, 'x', { get: function () { return 2; } }); console.log(x); } catch (e) { console.log('threw ' + e.constructor.name); }\");",
    ),
    (
        "a redefinition through a stored defineProperty",
        "eval(\"var x = 1; var d = Object.defineProperty; try { d(globalThis, 'x', { get: function () { return 2; } }); console.log(x); } catch (e) { console.log('threw ' + e.constructor.name); }\");",
    ),
    (
        "a redefinition through a computed key on the this of a plain call",
        "eval(\"var x = 1; var g = (function () { return this; })(); var k = 'define' + 'Property'; try { g.Object[k](g, 'x', { get: function () { return 2; } }); console.log(x); } catch (e) { console.log('threw ' + e.constructor.name); }\");",
    ),
];

#[test]
fn a_payload_that_can_tell_its_vars_from_script_vars_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = GLOBAL_OBSERVERS
        .iter()
        .map(|(label, source)| (label.to_string(), source.to_string()))
        .collect();
    let run_safe = |source: &str| Pipeline::new(options(false)).run(source).code().to_string();
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
