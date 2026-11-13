use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

fn lane(unsafe_transforms: bool) -> impl Fn(&str) -> String {
    move |source: &str| {
        Pipeline::new(Options {
            unsafe_transforms,
            unpack: false,
            ..Options::default()
        })
        .run(source)
        .code()
        .to_string()
    }
}

fn safe(source: &str) -> String {
    lane(false)(source)
}

const NAMED: &[(&str, &str, &[&str])] = &[
    (
        "early-return",
        "(function () { var a; var b; function c() { if (b) return a; b = 1; console.log('eval'); a = { v: 1 }; return a; } console.log('start'); console.log(c().v, c().v); })();",
        &["function requireModule1()", "hasRequiredModule1 = 1", "return module1;"],
    ),
    (
        "else-branch",
        "(function () { var a; var b; function c() { if (b) { return a; } else { b = 1; return a = 7; } } console.log(c() + c()); })();",
        &["function requireModule1()", "hasRequiredModule1"],
    ),
    (
        "negated-guard",
        "(function () { var b; var a = {}; function c() { if (!b) { b = 1; a.x = 2; } return a; } console.log(c().x); })();",
        &["function requireModule1()", "var module1 = {}"],
    ),
    (
        "short-circuit",
        "(function () { var b; var a = {}; function c() { return b || (b = 1, a.x = 3), a; } console.log(c().x); })();",
        &["function requireModule1()", "hasRequiredModule1"],
    ),
    (
        "module-object",
        "(function () { var m = { exports: {} }; var h; function r() { if (h) return m.exports; h = 1; (function (q, w) { w.a = 1; q.exports.b = 2; })(m, m.exports); return m.exports; } console.log(JSON.stringify(r())); })();",
        &["var module1 = {", "(function (module, exports)", "function requireModule1()"],
    ),
    (
        "cycle",
        "(function () { var a = {}; var b = {}; var c; var d; function e() { if (c) return a; c = 1; a.n = 'a'; a.o = f().n; return a; } function f() { if (d) return b; d = 1; b.n = 'b'; b.seen = String(e().n); return b; } console.log(e().o, f().seen); })();",
        &["function requireModule1()", "function requireModule2()"],
    ),
    (
        "kept-human-names",
        "(function () { var thing = {}; var hasRequiredThing; function requireThing() { if (hasRequiredThing) return thing; hasRequiredThing = 1; thing.v = 1; return thing; } console.log(requireThing().v); })();",
        &["function requireThing()", "var thing = {}"],
    ),
    (
        "stem-from-human-holder",
        "(function () { var getBuiltIn; var x; function y() { if (x) return getBuiltIn; x = 1; getBuiltIn = 5; return getBuiltIn; } console.log(y()); })();",
        &["function requireGetBuiltIn()", "hasRequiredGetBuiltIn"],
    ),
];

const FUNCTION_KEPT: &[(&str, &str, &str)] = &[
    (
        "own-arguments",
        "(function () { var a; var b; function c() { if (b) return a; b = 1; a = arguments.callee.name; return a; } console.log(c()); })();",
        "function c()",
    ),
    (
        "escapes-as-value",
        "(function () { var a; var b; function c() { if (b) return a; b = 1; a = 1; return a; } console.log(c.name, c()); })();",
        "function c()",
    ),
    (
        "sloppy-caller-read",
        "(function () { var a; var b; function peek() { return peek.caller.name; } function c() { if (b) return a; b = 1; a = peek(); return a; } console.log(c()); })();",
        "function c()",
    ),
];

const NOTHING_NAMED: &[(&str, &str)] = &[
    (
        "direct-eval",
        "(function () { var a; var b; function c() { if (b) return a; b = 1; a = eval('typeof b'); return a; } console.log(c()); })();",
    ),
    (
        "method-text",
        "(function () { var a; var b; var o = { m() { function c() { if (b) return a; b = 1; a = 1; return a; } return c(); } }; console.log(o.m(), String(o.m).length); })();",
    ),
    (
        "flag-read-twice",
        "(function () { var a; var b; function c() { if (b) return a; b = 1; a = 1; return a; } console.log(c(), b); })();",
    ),
];

fn assert_contains(label: &str, code: &str, wanted: &str) {
    assert!(
        code.contains(wanted),
        "{label}: missing {wanted:?} in\n{code}"
    );
}

#[test]
fn every_wrapper_shape_is_named_from_evidence_or_a_stable_number() {
    for (label, source, wanted) in NAMED {
        let code = safe(source);
        for wanted in *wanted {
            assert_contains(label, &code, wanted);
        }
    }
}

#[test]
fn a_wrapper_whose_function_name_can_be_observed_keeps_it() {
    for (label, source, kept) in FUNCTION_KEPT {
        let code = safe(source);
        assert_contains(label, &code, kept);
        assert!(!code.contains("requireModule"), "{label}:\n{code}");
    }
}

#[test]
fn a_wrapper_whose_text_or_scope_can_be_observed_is_left_alone() {
    for (label, source) in NOTHING_NAMED {
        for unsafe_transforms in [false, true] {
            let code = lane(unsafe_transforms)(source);
            assert!(!code.contains("Module1"), "{label}:\n{code}");
        }
    }
}

#[test]
fn every_row_prints_the_same_in_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let printed_text = |label: &str| label == "method-text";
    let programs: Vec<(String, String)> = NAMED
        .iter()
        .map(|(label, source, _)| (label.to_string(), source.to_string()))
        .chain(
            FUNCTION_KEPT
                .iter()
                .map(|(label, source, _)| (label.to_string(), source.to_string())),
        )
        .chain(
            NOTHING_NAMED
                .iter()
                .filter(|(label, _)| !printed_text(label))
                .map(|(label, source)| (label.to_string(), source.to_string())),
        )
        .collect();
    let default_lane = lane(false);
    let unsafe_lane = lane(true);
    let modes: [Mode<'_>; 2] = [("default", &default_lane), ("unsafe", &unsafe_lane)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
