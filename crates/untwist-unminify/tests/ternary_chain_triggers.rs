use untwist_core::pass::{Pass, PassGroup, PassOptions};
use untwist_testing::{transform_report_with, Allocator};
use untwist_unminify::{lexical_ctors, ternary_to_if, transpile_ctors, unminify_ctors};

struct Trigger {
    pass: &'static str,
    prelude: &'static str,
    atom: &'static str,
}

const fn trigger(pass: &'static str, prelude: &'static str, atom: &'static str) -> Trigger {
    Trigger {
        pass,
        prelude,
        atom,
    }
}

const TRIGGERS: &[Trigger] = &[
    trigger("block-statements", "", "function () { if (a) b(); }"),
    trigger("block-statements", "", "function () { while (a) b(); }"),
    trigger("block-statements", "", "() => (a(), b())"),
    trigger(
        "block-statements",
        "",
        "function () { if (a) b(); else if (c) d(); else e(); }",
    ),
    trigger("computed-properties", "", r#"require("foo")["default"]?.["_"]"#),
    trigger("computed-properties", "", r#"{ ["foo"]() {}, ["bar"]: 1 }"#),
    trigger("computed-properties", "", r#"console["1"]("hello")"#),
    trigger("computed-properties", "", r#"a["pos"]++"#),
    trigger("computed-properties", "", r#"a["class"] = b"#),
    trigger(
        "default-parameters",
        "",
        r#"function () { var e = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "foo"; }"#,
    ),
    trigger(
        "default-parameters",
        "",
        "function () { var x = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1; var y = arguments.length > 1 ? arguments[1] : undefined; }",
    ),
    trigger(
        "default-parameters",
        "",
        "function () { var x = arguments.length > 0 && arguments[0] !== undefined && arguments[0]; }",
    ),
    trigger(
        "default-parameters",
        "",
        "function (x, y) { if (x === undefined) { x = 1; } }",
    ),
    trigger("for-to-while", "", "function () { for (;;) b(); }"),
    trigger("for-to-while", "", "function () { for (; a(); ) b(); }"),
    trigger("for-to-while", "", "function () { for (;;) a(); }"),
    trigger("infinity", "", "1 / 0"),
    trigger("infinity", "", "-1 / 0"),
    trigger("infinity", "", "inf = 1 / 0"),
    trigger("invert-boolean-logic", "", "!(a == b)"),
    trigger("invert-boolean-logic", "", "!(a === b)"),
    trigger("invert-boolean-logic", "", "!(a != b)"),
    trigger("invert-boolean-logic", "", "!(a !== b)"),
    trigger("invert-boolean-logic", "", "!(a || b || c)"),
    trigger("invert-boolean-logic", "", "!(a && b && c)"),
    trigger("invert-boolean-logic", "", "!((a ?? b) || c)"),
    trigger("json-parse", "", r#"JSON.parse("[1,2,3]")"#),
    trigger("json-parse", "", r#"JSON.parse("1000000000000000000000")"#),
    trigger("json-parse", "", r#"JSON.parse("abc")"#),
    trigger("json-parse", "", r#"JSON.parse("null")"#),
    trigger("let-to-const", "", "function () { let a = 1; use(a); }"),
    trigger(
        "let-to-const",
        "",
        "function () { let a = 1, b = 2; b = 3; use(a, b); }",
    ),
    trigger("let-to-const", "", "function () { let y = 1; return y; }"),
    trigger("var-to-let", "", "function () { var a = g(); use(a); }"),
    trigger(
        "var-to-let",
        "",
        "function () { for (var i = 0; i < 3; i++) use(i); }",
    ),
    trigger("var-to-let", "", "function () { for (var k in o) use(k); }"),
    trigger(
        "function-to-arrow",
        "",
        "function () { var h = function () { return 1; }; return h(); }",
    ),
    trigger("function-to-arrow", "", "list.map(function (x) { return x; })"),
    trigger("function-to-arrow", "", "list.forEach(function () { use(); })"),
    trigger(
        "merge-member-declarations",
        "",
        "function (e) { var a = e.x; var b = e.y; use(a, b); }",
    ),
    trigger(
        "merge-member-declarations",
        "",
        "function (e) { const a = e.x; const b = e[\"y\"]; return a + b; }",
    ),
    trigger(
        "merge-member-declarations",
        "",
        "function (e) { if (e) { let a = e.x; let b = e.y; a++; b++; use(a, b); } }",
    ),
    trigger("logical-assignments", "", "x || (x = y)"),
    trigger("logical-assignments", "", "x && (x = y)"),
    trigger("logical-assignments", "", "x.y || (x.y = z)"),
    trigger("logical-assignments", "", "x[y] || (x[y] = z)"),
    trigger(
        "logical-assignments",
        "var _x$y;",
        "(_x$y = x.y()).z || (_x$y.z = b)",
    ),
    trigger("logical-to-if", "", "x && y && z()"),
    trigger("logical-to-if", "", "x || y || z()"),
    trigger("logical-to-if", "", "x ?? y()"),
    trigger("logical-to-if", "", r#""red" === color && paint()"#),
    trigger(
        "merge-else-if",
        "",
        "function () { if (x) {} else { if (y) {} } }",
    ),
    trigger(
        "merge-else-if",
        "",
        "function () { if (a) { p(); } else { if (b) { q(); } else { if (c) { r(); } } } }",
    ),
    trigger(
        "merge-else-if",
        "",
        "function () { if (!c) { if (d) { p(); } else { q(); } } else { r(); } }",
    ),
    trigger("merge-strings", "", r#""a" + "b" + "c""#),
    trigger("merge-strings", "", r#""a" + "b" + xyz + "c" + "d""#),
    trigger("merge-strings", "", r#"s = "\ud800" + "b""#),
    trigger("nullish-coalescing", "", "a != null ? a : b"),
    trigger(
        "nullish-coalescing",
        "",
        "a !== null && a !== undefined ? a : b",
    ),
    trigger(
        "nullish-coalescing",
        "var _a;",
        "(_a = a) !== null && _a !== undefined ? _a : b",
    ),
    trigger(
        "nullish-coalescing",
        "var _a$b;",
        "(_a$b = a.b) !== null && _a$b !== undefined ? _a$b : c",
    ),
    trigger(
        "nullish-coalescing",
        "var _a_b;",
        "((_a_b = a.b) !== null && _a_b !== undefined) || c",
    ),
    trigger(
        "nullish-coalescing",
        "var interp;",
        r#"(interp = hello) == null ? "" : interp"#,
    ),
    trigger("nullish-coalescing", "", "a != null"),
    trigger("nullish-coalescing", "", "a !== null && a !== undefined"),
    trigger(
        "nullish-coalescing",
        "var _a;",
        "(_a = a) !== null && _a !== undefined",
    ),
    trigger("nullish-coalescing", "var interp;", "(interp = hello) == null"),
    trigger("nullish-coalescing-assignment", "", "a ?? (a = b)"),
    trigger("nullish-coalescing-assignment", "", "a.b ?? (a.b = c)"),
    trigger(
        "nullish-coalescing-assignment",
        "var _a;",
        "(_a = a).b ?? (_a.b = c)",
    ),
    trigger(
        "nullish-coalescing-assignment",
        "var _a;",
        "obj[_a = incr()] ?? (obj[_a] = incr())",
    ),
    trigger(
        "number-expressions",
        "",
        "-0x1021e + -0x7eac8 + 0x17 * 0xac9c",
    ),
    trigger("number-expressions", "", r#"-"0xa6" - -331"#),
    trigger("number-expressions", "", "10 / 2"),
    trigger("number-expressions", "", r#"0x1021e + "test""#),
    trigger(
        "optional-chaining",
        "",
        "a === null || a === undefined ? undefined : a.b",
    ),
    trigger(
        "optional-chaining",
        "",
        "a === null || a === undefined ? undefined : a[b]",
    ),
    trigger(
        "optional-chaining",
        "",
        "a === null || a === undefined ? undefined : a()",
    ),
    trigger(
        "optional-chaining",
        "var _a;",
        "(_a = a) === null || _a === undefined ? undefined : _a.b",
    ),
    trigger(
        "optional-chaining",
        "var _a;",
        "(_a = a === null || a === undefined ? undefined : a.b) === null || _a === undefined ? undefined : _a.c",
    ),
    trigger("optional-chaining", "", "a === null || a === undefined"),
    trigger(
        "optional-chaining",
        "var _a;",
        "(_a = a) === null || _a === undefined",
    ),
    trigger("remove-double-not", "", "!!a ? b : c"),
    trigger("remove-double-not", "", "!!a"),
    trigger("remove-double-not", "", "[].filter(a => !!a)"),
    trigger("remove-double-not", "", "x = !!a"),
    trigger("remove-double-not", "", "function () { if (!!a) {} }"),
    trigger("remove-double-not", "", "function () { return !!!a; }"),
    trigger("sequence", "", "(a(), b())"),
    trigger("sequence", "", "a = (b(), c())"),
    trigger("sequence", "", "a.x = (b(), c())"),
    trigger("sequence", "", "console.log(a = (b(), c()))"),
    trigger("sequence", "", r#"Lr[("nnQB", "jcIgN")]()"#),
    trigger("sequence", "", "function () { if (a) b(), c(); }"),
    trigger("sequence", "", "function () { return a(), b(), c(); }"),
    trigger("split-for-loop-vars", "", "function () { for (var i = 0;;) {} }"),
    trigger(
        "split-for-loop-vars",
        "",
        "function () { for (var i = 0, j = 1, k = 2; k < 5; k++) {} }",
    ),
    trigger(
        "split-for-loop-vars",
        "",
        "function () { for (let i = 0, j = 1;;) { setTimeout(() => console.log(i)); if (++i > 3) break; } }",
    ),
    trigger(
        "split-variable-declarations",
        "",
        "function () { const a = 1, b = 2, c = 3; }",
    ),
    trigger(
        "split-variable-declarations",
        "",
        "function () { for (var i = 0, j = 1;;) {} }",
    ),
    trigger(
        "split-variable-declarations",
        "",
        "function () { for (var i = 0, j = 1; i < 10; i++, j++) var a, b; }",
    ),
    trigger("string-literal-in-template-literal", "", "`Hello ${'World'}!`"),
    trigger("string-literal-in-template-literal", "", "`Hello ${'World'}${'!'}`"),
    trigger("string-literal-in-template-literal", "", r#"`${"Hi!"}`"#),
    trigger("string-literal-in-template-literal", "", r#"`a${"\ud800"}b`"#),
    trigger(
        "template-literals",
        "",
        r#""'".concat(foo, "' \"").concat(bar, "\"")"#,
    ),
    trigger("template-literals", "", r#""".concat(1)"#),
    trigger(
        "template-literals",
        "",
        r#"1 + "".concat(foo).concat(bar).concat(baz)"#,
    ),
    trigger("template-literals", "", "`a${1}b` + 'c'"),
    trigger(
        "template-literals",
        "",
        r#""".concat(1, f, "oo", true).concat(b, "ar", 0).concat(baz)"#,
    ),
    trigger("ternary-to-if", "", "a ? b() : c()"),
    trigger("ternary-to-if", "", "function () { return a ? b() : c(); }"),
    trigger("ternary-to-if", "", "function () { const x = a ? b() : c(); }"),
    trigger("ternary-to-if", "", "function g() { return a ? b : c; }"),
    trigger("ternary-to-if", "", "a ? b : c"),
    trigger("truncate-number-literal", "", "!(a | 12.34)"),
    trigger("truncate-number-literal", "", "!(a ^ 0xfffffffff)"),
    trigger("truncate-number-literal", "", "!(a << 64)"),
    trigger("truncate-number-literal", "", "!(1000 << a)"),
    trigger("typeof-undefined", "", r#"typeof a > "u""#),
    trigger("typeof-undefined", "", r#"typeof a < "u""#),
    trigger("typeof-undefined", "", r#""u" > typeof x"#),
    trigger("unary-expressions", "", "void foo()"),
    trigger("unary-expressions", "", "typeof foo()"),
    trigger("unary-expressions", "", "!foo()"),
    trigger("unary-expressions", "", "function () { return void foo(); }"),
    trigger("unminify-booleans", "", "!0"),
    trigger("unminify-booleans", "", "!!1"),
    trigger("unminify-booleans", "", "!![]"),
    trigger("unminify-booleans", "", "!1"),
    trigger("unminify-booleans", "", "![]"),
    trigger("unwrap-comma-call", "", "(0, f)(a, b)"),
    trigger("unwrap-comma-call", "", "(void 0, f)(a)"),
    trigger("unwrap-comma-call", "", r#"("", f)(a)"#),
    trigger("unwrap-comma-call", "", "(null, f)(a)"),
    trigger("unwrap-comma-call", "", "(0, 1, f)(a)"),
    trigger("unwrap-comma-call", "", "(0, () => 1)(a)"),
    trigger("void-to-undefined", "", "void 0"),
    trigger("void-to-undefined", "", "void 0 === z"),
    trigger("void-to-undefined", "", "x = void 0"),
    trigger("yoda", "", r#""red" === color"#),
    trigger("yoda", "", "null == x"),
    trigger("yoda", "", r#""red" !== color"#),
    trigger("yoda", "", "null != x"),
    trigger("yoda", "", "0 < x"),
    trigger("yoda", "", "0 * x"),
    trigger("yoda", "", r#""str" == x"#),
    trigger("yoda", "", "-1 == x"),
    trigger("yoda", "", "true == x"),
    trigger("yoda", "", "undefined == x"),
    trigger("yoda", "", "NaN == x"),
    trigger("yoda", "", "2 + x"),
];

const WRAPPERS: [(&str, &str); 4] = [("", ""), ("!(", ")"), ("!!(", ")"), ("!!!!(", ")")];
const TEST_CONSEQUENTS: [&str; 5] = ["", "a", "x", "_a", "undefined"];
const TEST_ALTERNATES: [&str; 4] = ["", "a.p", "x.p", "_a.p"];
const DEEPEST: usize = 3;

#[derive(Clone, Copy)]
enum Slot {
    Test(&'static str, &'static str),
    Consequent,
    Tail,
}

fn stage_passes<'a>(one_level: bool) -> Vec<Box<dyn Pass<'a> + 'a>> {
    transpile_ctors()
        .into_iter()
        .chain(unminify_ctors())
        .chain(lexical_ctors())
        .map(|ctor| {
            let pass = ctor();
            if one_level && pass.name() == "ternary-to-if" {
                ternary_to_if::one_level_boxed_for_tests()
            } else {
                pass
            }
        })
        .collect()
}

fn unminified(source: &str, one_level: bool, extended: bool) -> String {
    let alloc = Allocator::default();
    let opts = PassOptions {
        unsafe_transforms: true,
        extended,
        max_iterations: None,
        fuel: None,
        ..PassOptions::default()
    };
    let group = PassGroup::new(stage_passes(one_level), opts);
    transform_report_with(&alloc, source, group).0
}

fn filler_link(level: usize) -> String {
    format!("t{level} ? c{level}() : ")
}

fn chain(depth: usize, target: usize, slot: Slot, atom: &str) -> String {
    let mut text = String::new();
    for level in 0..depth {
        if level != target {
            text.push_str(&filler_link(level));
            continue;
        }
        match slot {
            Slot::Test(consequent, alternate) => {
                let consequent = if consequent.is_empty() {
                    format!("c{level}()")
                } else {
                    consequent.to_owned()
                };
                text.push_str(&format!("{atom} ? {consequent} : "));
                if !alternate.is_empty() {
                    text.push_str(alternate);
                    return text;
                }
            }
            Slot::Consequent => text.push_str(&format!("t{level} ? {atom} : ")),
            Slot::Tail => text.push_str(&filler_link(level)),
        }
    }
    match slot {
        Slot::Tail => text.push_str(atom),
        _ => text.push_str("z()"),
    }
    text
}

fn placements() -> Vec<(usize, usize, Slot)> {
    let mut all = Vec::new();
    for depth in 1..=DEEPEST {
        for target in 0..depth {
            for consequent in TEST_CONSEQUENTS {
                for alternate in TEST_ALTERNATES {
                    all.push((depth, target, Slot::Test(consequent, alternate)));
                }
            }
            all.push((depth, target, Slot::Consequent));
        }
        all.push((depth, 0, Slot::Tail));
    }
    all
}

fn in_contexts(prelude: &str, chain: &str) -> [String; 3] {
    [
        format!("{prelude}\n{chain};"),
        format!("function f() {{\n{prelude}\nreturn {chain};\n}}"),
        format!("const g = () => {{\n{prelude}\nreturn {chain};\n}};"),
    ]
}

fn sources_for(trigger: &Trigger) -> Vec<String> {
    let mut sources = Vec::new();
    for (open, close) in WRAPPERS {
        let atom = format!("{open}({}){close}", trigger.atom);
        for (depth, target, slot) in placements() {
            let text = chain(depth, target, slot, &atom);
            sources.extend(in_contexts(trigger.prelude, &text));
        }
    }
    sources
}

fn mismatches_in(triggers: &[Trigger]) -> Vec<String> {
    let mut found = Vec::new();
    for trigger in triggers {
        for source in sources_for(trigger) {
            for extended in [false, true] {
                let whole = unminified(&source, false, extended);
                let peeled = unminified(&source, true, extended);
                if whole != peeled {
                    found.push(format!(
                        "{} extended {extended}\n{source}\n--- one visit\n{whole}\n--- one level\n{peeled}",
                        trigger.pass
                    ));
                }
            }
        }
    }
    found
}

#[test]
fn every_unminify_pass_has_at_least_three_triggers() {
    let mut names: Vec<&'static str> = stage_passes(false).iter().map(|pass| pass.name()).collect();
    names.sort_unstable();
    assert_eq!(names.len(), 33);
    for name in names {
        let count = TRIGGERS
            .iter()
            .filter(|trigger| trigger.pass == name)
            .count();
        assert!(count >= 3, "{name} has {count} triggers");
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "every trigger in every chain position; run with cargo test --release -p untwist-unminify --test ternary_chain_triggers"
)]
fn every_trigger_in_every_chain_position_unminifies_to_the_same_bytes_either_way() {
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    let chunk = TRIGGERS.len().div_ceil(workers);
    let found: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = TRIGGERS
            .chunks(chunk)
            .map(|part| scope.spawn(move || mismatches_in(part)))
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
            })
            .collect()
    });
    for mismatch in &found {
        eprintln!("=====\n{mismatch}");
    }
    assert!(found.is_empty(), "{} mismatches", found.len());
}
