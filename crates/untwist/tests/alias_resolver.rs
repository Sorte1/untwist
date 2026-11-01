use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{
    self, node_or_skip, patched_after_named_rotator, patched_after_rotator, Mode,
};

const F: &str = "function () { return '?'; }";
const R: &str = "{ fromCharCode: function () { return '?'; } }";
const SPECIES: &str = "class X extends Array { join() { return '?'; } } ";

const JOIN_TEST: &str = "[1, 2].join('') === '12'";
const CONCAT_TEST: &str = "[1].concat([2]).join('') === '12'";
const PROTO_TEST: &str = "typeof b.hasOwnProperty === 'function'";
const STRING_TEST: &str = "String.fromCharCode(33) === '!'";

const FOLD_JOIN: &str = "console.log([1, 2].join(''));";
const FOLD_CONCAT: &str = "console.log([1].concat([2]).join(''));";

struct Access {
    label: &'static str,
    decls: &'static str,
    expr: &'static str,
}

const fn access(label: &'static str, decls: &'static str, expr: &'static str) -> Access {
    Access { label, decls, expr }
}

const ARRAY_ACCESSES: &[Access] = &[
    access("direct", "", "Array"),
    access("global-this", "", "globalThis.Array"),
    access("global", "", "global.Array"),
    access("global-computed", "", "globalThis['Array']"),
    access("global-template", "", "globalThis[`Arr${'ay'}`]"),
    access("alias", "var A0 = Array;", "A0"),
    access("alias-2", "var A0 = Array; var A1 = A0;", "A1"),
    access("alias-3", "var A0 = Array; var A1 = A0; var A2 = A1;", "A2"),
    access("alias-of-global", "var A0 = globalThis.Array;", "A0"),
    access("global-alias", "var g0 = globalThis;", "g0.Array"),
    access(
        "global-alias-2",
        "var g0 = globalThis; var g1 = g0;",
        "g1.Array",
    ),
    access(
        "global-destructure",
        "var { Array: A0 } = globalThis;",
        "A0",
    ),
    access("alias-late", "var A0; A0 = Array;", "A0"),
    access("conditional", "var A0 = 1 ? Array : 0;", "A0"),
    access("global-of-global", "", "globalThis.globalThis.Array"),
    access("constructor-of-array", "", "[].constructor"),
    access("alias-of-constructor", "var A0 = [].constructor;", "A0"),
    access(
        "get-prototype-of-constructor",
        "var A0 = Object.getPrototypeOf([]).constructor;",
        "A0",
    ),
    access("constructor-of-binding", "var x0 = [1];", "x0.constructor"),
    access("constructor-of-from", "", "Array.from([1]).constructor"),
    access("constructor-of-new", "", "new Array().constructor"),
    access("constructor-of-split", "", "'a b'.split(' ').constructor"),
    access(
        "constructor-of-call",
        "function mk0() { return []; }",
        "mk0().constructor",
    ),
];

const FUNCTION_ACCESSES: &[Access] = &[
    access("direct", "", "Function"),
    access("global-this", "", "globalThis.Function"),
    access(
        "global-destructure",
        "var { Function: Fn0 } = globalThis;",
        "Fn0",
    ),
    access(
        "global-alias-chain",
        "var g0 = globalThis; var Fn0 = g0.Function;",
        "Fn0",
    ),
    access("global-of-global", "", "globalThis.globalThis.Function"),
];

const GLOBAL_ACCESSES: &[Access] = &[
    access("global-this", "", "globalThis"),
    access("global", "", "global"),
    access("alias", "var g0 = globalThis;", "g0"),
    access("alias-2", "var g0 = globalThis; var g1 = g0;", "g1"),
    access("global-of-global", "", "globalThis.globalThis"),
];

const PLACEMENTS: [&str; 2] = ["inline", "hoisted"];

fn placed(placement: &str, decls: &str, statement: &str) -> String {
    match placement {
        "hoisted" => format!("function hoisted0() {{ {statement} }} {decls} hoisted0();"),
        _ => format!("{decls} {statement}"),
    }
}

fn mismatches(node: &Path, programs: &[(String, String)]) -> Vec<String> {
    let conservative = Options {
        extended: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_default = |source: &str| {
        Pipeline::new(Options::default())
            .run(source)
            .code()
            .to_string()
    };
    let run_conservative = |source: &str| {
        Pipeline::new(conservative.clone())
            .run(source)
            .code()
            .to_string()
    };
    let runs: [Mode<'_>; 2] = [
        ("default", &run_default),
        ("conservative", &run_conservative),
    ];
    node_rows::mismatches(node, programs, &runs)
}

fn check(programs: Vec<(String, String)>) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let wrong = mismatches(&node, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn rows<'r>(
    accesses: &'r [Access],
    target: &'r str,
    patch: impl Fn(&Access) -> String + 'r,
) -> impl Iterator<Item = (String, String, String)> + 'r {
    accesses.iter().flat_map(move |a| {
        let statement = patch(a);
        PLACEMENTS.iter().map(move |placement| {
            let label = format!("{target}:{}:{placement}", a.label);
            (
                label,
                placement.to_string(),
                placed(placement, a.decls, &statement),
            )
        })
    })
}

fn decoded(
    rows: impl Iterator<Item = (String, String, String)>,
    test: &str,
    prefix: &str,
) -> Vec<(String, String)> {
    rows.map(|(label, _, patch)| {
        (
            label,
            patched_after_rotator(test, &format!("{prefix}{patch}")),
        )
    })
    .collect()
}

fn folded(
    rows: impl Iterator<Item = (String, String, String)>,
    prefix: &str,
    tail: &str,
) -> Vec<(String, String)> {
    rows.map(|(label, _, patch)| (format!("fold-{label}"), format!("{prefix}{patch} {tail}")))
        .collect()
}

fn join_write(a: &Access) -> String {
    format!("{}.prototype.join = {F};", a.expr)
}

fn species_write(a: &Access) -> String {
    format!(
        "Object.defineProperty({}, Symbol.species, {{ value: X }});",
        a.expr
    )
}

#[test]
fn a_prototype_method_patched_through_any_realm_access_prints_like_node() {
    check(decoded(
        rows(ARRAY_ACCESSES, "prototype", join_write),
        JOIN_TEST,
        "",
    ));
}

#[test]
fn a_prototype_chain_cut_through_any_realm_access_prints_like_node() {
    let cut = |a: &Access| format!("{}.prototype.__proto__ = null;", a.expr);
    check(decoded(rows(FUNCTION_ACCESSES, "cut", cut), PROTO_TEST, ""));
}

#[test]
fn a_species_written_on_a_constructor_reached_any_way_prints_like_node() {
    check(decoded(
        rows(ARRAY_ACCESSES, "species", species_write),
        CONCAT_TEST,
        SPECIES,
    ));
}

#[test]
fn a_definer_called_on_a_global_object_reached_any_way_prints_like_node() {
    let plain = |a: &Access| {
        format!(
            "Object.defineProperty({}, 'String', {{ value: {R} }});",
            a.expr
        )
    };
    let computed = |a: &Access| {
        format!(
            "Object['define' + 'Property']({}, 'String', {{ value: {R} }});",
            a.expr
        )
    };
    let mut all = decoded(rows(GLOBAL_ACCESSES, "definer", plain), STRING_TEST, "");
    all.extend(decoded(
        rows(GLOBAL_ACCESSES, "definer-computed", computed),
        STRING_TEST,
        "",
    ));
    check(all);
}

#[test]
fn a_literal_chain_beside_any_realm_access_patch_prints_like_node() {
    let mut all = folded(rows(ARRAY_ACCESSES, "join", join_write), "", FOLD_JOIN);
    all.extend(folded(
        rows(ARRAY_ACCESSES, "species", species_write),
        SPECIES,
        FOLD_CONCAT,
    ));
    check(all);
}

const UNMODELLED_FORMS: &[(&str, &str)] = &[
    ("for-of", "for (var A of [Array]); A.prototype.join = F0;"),
    (
        "array-destructuring",
        "var A; [A] = [Array]; A.prototype.join = F0;",
    ),
    (
        "object-destructuring",
        "var A; ({ A } = { A: Array }); A.prototype.join = F0;",
    ),
    (
        "declared-array-pattern",
        "var [A] = [Array]; A.prototype.join = F0;",
    ),
    ("or-assign", "var A; A ||= Array; A.prototype.join = F0;"),
    (
        "nullish-assign",
        "var A; A ??= Array; A.prototype.join = F0;",
    ),
    (
        "and-assign",
        "var A = 1; A &&= Array; A.prototype.join = F0;",
    ),
    (
        "update-then-assign",
        "var A = 0; A++; A = Array; A.prototype.join = F0;",
    ),
    (
        "catch",
        "try { throw Array; } catch (A) { A.prototype.join = F0; }",
    ),
    (
        "default-parameter",
        "function f0(A = Array) { A.prototype.join = F0; } f0();",
    ),
    ("implicit-global", "A = Array; A.prototype.join = F0;"),
    ("sequence", "(0, Array).prototype.join = F0;"),
    (
        "assignment-value",
        "var A; (A = Array).prototype.join = F0;",
    ),
    (
        "mixed-conditional",
        "var P = Math.random() < 2 ? Array.prototype : {}; P.join = F0;",
    ),
];

fn unmodelled(form: &str) -> String {
    form.replace("F0", F)
}

#[test]
fn an_assignment_form_the_resolver_does_not_model_prints_like_node() {
    let mut all: Vec<(String, String)> = UNMODELLED_FORMS
        .iter()
        .map(|(label, form)| {
            (
                label.to_string(),
                patched_after_rotator(JOIN_TEST, &unmodelled(form)),
            )
        })
        .collect();
    all.extend(UNMODELLED_FORMS.iter().map(|(label, form)| {
        (
            format!("fold-{label}"),
            format!("{} {FOLD_JOIN}", unmodelled(form)),
        )
    }));
    check(all);
}

const MAY_SOURCES: &[(&str, &str, &str)] = &[
    ("constructor", "Array", "X.prototype.join = F0;"),
    ("prototype", "Array.prototype", "X.join = F0;"),
    (
        "get-prototype-of",
        "Object.getPrototypeOf([])",
        "X.join = F0;",
    ),
];

const MAY_FORMS: &[(&str, &str)] = &[
    ("fn-default", "function f0(X = S0) { W0 } f0();"),
    ("arrow-default", "var f0 = (X = S0) => { W0 }; f0();"),
    (
        "param-destr-default",
        "function f0({ X = S0 }) { W0 } f0({});",
    ),
    (
        "param-arr-destr-default",
        "function f0([X = S0]) { W0 } f0([]);",
    ),
    ("destr-default", "var { X = S0 } = {}; W0"),
    ("arr-destr-default", "var [X = S0] = []; W0"),
    ("assign-destr-default", "var X; ({ X = S0 } = {}); W0"),
    ("assign-arr-destr-default", "var X; [X = S0] = []; W0"),
    ("param-reassign", "function f0(X) { X = S0; W0 } f0();"),
    (
        "realm-first",
        "var X = S0; if (Math.random() > 2) X = {}; W0",
    ),
    (
        "realm-last",
        "var X = {}; if (Math.random() > 2) X = S0; W0",
    ),
    (
        "let-realm-first",
        "let X = S0; if (Math.random() > 2) X = {}; W0",
    ),
    (
        "let-realm-last",
        "let X = {}; if (Math.random() > 2) X = S0; W0",
    ),
    (
        "hoisted-realm-first",
        "function g0() { X = {}; } var X = S0; if (Math.random() > 2) g0(); W0",
    ),
    (
        "hoisted-realm-last",
        "function g0() { X = S0; } var X = {}; if (Math.random() > 2) g0(); W0",
    ),
    (
        "default-then-reassign",
        "function f0(X = {}) { if (Math.random() > 2) X = S0; W0 } f0();",
    ),
    (
        "default-then-clear",
        "function f0(X = S0) { if (Math.random() > 2) X = {}; W0 } f0();",
    ),
    (
        "several-realm-first",
        "var X = S0; if (Math.random() > 2) X = Object.prototype; W0",
    ),
    (
        "several-realm-last",
        "var X = Object.prototype; if (Math.random() > 2) X = S0; W0",
    ),
    (
        "unmodelled-then-realm",
        "var X; [X] = [{}]; if (Math.random() > 2) X = S0; W0",
    ),
];

fn may_alias_rows() -> Vec<(String, String)> {
    MAY_SOURCES
        .iter()
        .flat_map(|(source, value, write)| {
            MAY_FORMS.iter().map(move |(form, template)| {
                let program = template
                    .replace("W0", write)
                    .replace("S0", value)
                    .replace("F0", F);
                (format!("{form}:{source}"), program)
            })
        })
        .collect()
}

#[test]
fn a_binding_that_may_hold_a_realm_path_is_judged_as_writing_it() {
    let rows = may_alias_rows();
    let mut all: Vec<(String, String)> = rows
        .iter()
        .map(|(label, program)| (label.clone(), patched_after_rotator(JOIN_TEST, program)))
        .collect();
    all.extend(
        rows.iter()
            .map(|(label, program)| (format!("fold-{label}"), format!("{program} {FOLD_JOIN}"))),
    );
    check(all);
}

const HOP_FORMS: &[(&str, &str)] = &[
    ("await-constructor", "(await Array).prototype.join = F0;"),
    ("await-prototype", "(await Array.prototype).join = F0;"),
    ("await-alias", "var P = await Array.prototype; P.join = F0;"),
    ("sequence-prototype", "(0, Array.prototype).join = F0;"),
    (
        "conditional-prototype",
        "(Math.random() < 2 ? Array.prototype : {}).join = F0;",
    ),
    ("logical-prototype", "(0 || Array.prototype).join = F0;"),
    (
        "assignment-prototype",
        "var P; (P = Array.prototype).join = F0;",
    ),
];

#[test]
fn a_member_write_through_an_await_or_a_value_hop_is_judged() {
    let mut all: Vec<(String, String)> = HOP_FORMS
        .iter()
        .map(|(label, form)| {
            let body = format!("{} {FOLD_JOIN}", unmodelled(form));
            (
                format!("fold-{label}"),
                format!("(async function () {{ {body} }})();"),
            )
        })
        .collect();
    all.extend(
        HOP_FORMS
            .iter()
            .filter(|(_, form)| !form.contains("await"))
            .map(|(label, form)| {
                (
                    label.to_string(),
                    patched_after_rotator(JOIN_TEST, &unmodelled(form)),
                )
            }),
    );
    check(all);
}

const CHAR_CODE_TEST: &str = "'-'.charCodeAt(0) === 45";
const FOLD_CHAR_CODE: &str = "console.log('-'.charCodeAt(0));";

const CODE_TEXT_WRITES: &[(&str, &str, &str, &str)] = &[
    (
        "join-text",
        JOIN_TEST,
        "Function('Array.prototype.join = function () { return \"?\"; }')();",
        FOLD_JOIN,
    ),
    (
        "join-text-alias",
        JOIN_TEST,
        "Function('var AP = Array.prototype; AP.join = function () { return \"?\"; }')();",
        FOLD_JOIN,
    ),
    (
        "join-template-text",
        JOIN_TEST,
        "Function(`Array.prototype.join = function () { return '?'; }`)();",
        FOLD_JOIN,
    ),
    (
        "char-code-text",
        CHAR_CODE_TEST,
        "Function('String.prototype.charCodeAt = function () { return 1; }')();",
        FOLD_CHAR_CODE,
    ),
    (
        "join-in-decoder",
        "(Array.prototype.join = function () { return '?'; }, [1, 2].join('') === '12')",
        "",
        FOLD_JOIN,
    ),
    (
        "join-direct",
        JOIN_TEST,
        "Array.prototype.join = function () { return '?'; };",
        FOLD_JOIN,
    ),
];

const OTHER_UNIT_TESTS: &[(&str, &str)] = &[("reads-nothing", "true"), ("reads-join", JOIN_TEST)];

fn two_unit_rows(write: &(&str, &str, &str, &str)) -> Vec<(String, String)> {
    let (label, test, patch, tail) = *write;
    OTHER_UNIT_TESTS
        .iter()
        .flat_map(|(other, other_test)| {
            let writer_first = patched_after_named_rotator("", test, patch)
                + &patched_after_named_rotator("2", other_test, "");
            let writer_second = patched_after_named_rotator("", other_test, "")
                + &patched_after_named_rotator("2", test, patch);
            [
                (format!("{label}:writer-first:{other}"), writer_first),
                (format!("{label}:writer-second:{other}"), writer_second),
            ]
        })
        .flat_map(move |(row, program)| {
            [
                (row.clone(), program.clone()),
                (format!("fold-{row}"), format!("{program}{tail}")),
            ]
        })
        .collect()
}

#[test]
fn a_later_decoder_round_keeps_every_write_an_earlier_round_recorded() {
    check(CODE_TEXT_WRITES.iter().flat_map(two_unit_rows).collect());
}

const RUNNER_TEXT: &str = "'Array.prototype.join = function () { return \"?\"; }'";

const RUNNERS_IN_DECODER: &[(&str, &str)] = &[
    ("function", "Function(T0)()"),
    ("new-function", "new Function(T0)()"),
    ("alias", "(function () { var F = Function; F(T0)(); })()"),
    (
        "template",
        "Function(`Array.prototype.join = function () { return '?'; }`)()",
    ),
    ("indirect-eval", "(0, eval)(T0)"),
    ("direct-eval", "eval(T0)"),
    ("global-peel", "globalThis['Func' + 'tion'](T0)()"),
    ("constructor", "[].constructor.constructor(T0)()"),
];

fn runner_test(runner: &str) -> String {
    format!(
        "({}, [1, 2].join('') === '12')",
        runner.replace("T0", RUNNER_TEXT)
    )
}

#[test]
fn a_code_runner_inside_a_decoder_leaves_it_undecoded_and_stops_the_fold() {
    let mut all: Vec<(String, String)> = Vec::new();
    for (label, runner) in RUNNERS_IN_DECODER {
        let program = patched_after_rotator(&runner_test(runner), "");
        all.push((format!("fold-{label}"), format!("{program}{FOLD_JOIN}")));
        all.push((label.to_string(), program));
    }
    check(all);
}

fn decodes(program: &str) -> bool {
    let code = Pipeline::new(Options::default())
        .run(program)
        .code()
        .to_string();
    code.contains("console.log(\"log!\")")
}

#[test]
fn a_code_runner_named_but_not_called_or_called_after_decoding_keeps_the_decode() {
    let named = patched_after_rotator(
        "({ Function: 'eval' }).Function === 'eval' && [1, 2].join('') === '12'",
        "",
    );
    let after = format!(
        "{}Function({RUNNER_TEXT})();\n",
        patched_after_rotator("true", "")
    );
    assert!(decodes(&named), "named");
    assert!(decodes(&after), "after");
    check(vec![
        ("named-not-called".to_string(), named),
        ("called-after-decoding".to_string(), after),
    ]);
}

const HERITAGES: &[(&str, &str)] = &[
    ("class", "class K extends Array {}"),
    ("class-expression", "var K = class extends Array {};"),
    ("class-global", "class K extends globalThis.Array {}"),
    ("class-sequence", "class K extends (0, Array) {}"),
    ("function-proto", "function K() {} K.__proto__ = Array;"),
    (
        "set-prototype-of",
        "function K() {} Object.setPrototypeOf(K, Array);",
    ),
];

const HERITAGE_SPELLINGS: &[(&str, &str)] = &[
    ("proto", "K.__proto__"),
    ("proto-computed", "K['__proto__']"),
    ("get-prototype-of", "Object.getPrototypeOf(K)"),
    ("reflect", "Reflect.getPrototypeOf(K)"),
    ("alias", "A"),
];

const HERITAGE_TARGETS: &[(&str, &str)] = &[
    ("method", "X0.prototype.join = F0;"),
    (
        "definer",
        "Object.defineProperty(X0.prototype, 'join', { value: F0 });",
    ),
];

fn heritage_rows() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (heritage, decl) in HERITAGES {
        for (spelling, base) in HERITAGE_SPELLINGS {
            for (target, write) in HERITAGE_TARGETS {
                let alias = if *spelling == "alias" {
                    "var A = Object.getPrototypeOf(K); "
                } else {
                    ""
                };
                let write = write.replace("X0", base).replace("F0", F);
                let patch = format!("{decl} {alias}{write}");
                rows.push((format!("{heritage}:{spelling}:{target}"), patch));
            }
        }
    }
    for (target, write) in HERITAGE_TARGETS {
        let body = write.replace("X0", "super").replace("F0", F);
        let patch = format!("class K extends Array {{ static p() {{ {body} }} }} K.p();");
        rows.push((format!("class:super:{target}"), patch));
    }
    rows.push((
        "class:proto:string".to_string(),
        "class K extends String {} K.__proto__.prototype.charCodeAt = function () { return 7; };"
            .to_string(),
    ));
    rows
}

#[test]
fn a_realm_reached_through_class_heritage_or_a_written_proto_is_judged() {
    let rows = heritage_rows();
    let mut all: Vec<(String, String)> = rows
        .iter()
        .map(|(label, patch)| {
            let test = if label.ends_with(":string") {
                "'a'.charCodeAt(0) === 97"
            } else {
                JOIN_TEST
            };
            (label.clone(), patched_after_rotator(test, patch))
        })
        .collect();
    all.extend(rows.iter().map(|(label, patch)| {
        let tail = if label.ends_with(":string") {
            "console.log('a'.charCodeAt(0));"
        } else {
            FOLD_JOIN
        };
        (format!("fold-{label}"), format!("{patch} {tail}"))
    }));
    check(all);
}

const MAKERS: &[(&str, &str)] = &[
    ("create", "var o = Object.create(P0);"),
    ("literal-proto", "var o = { __proto__: P0 };"),
    (
        "set-prototype-of",
        "var o = {}; Object.setPrototypeOf(o, P0);",
    ),
    (
        "reflect-set-prototype-of",
        "var o = {}; Reflect.setPrototypeOf(o, P0);",
    ),
    ("assigned-proto", "var o = {}; o.__proto__ = P0;"),
    ("new", "function F() {} F.prototype = P0; var o = new F();"),
];

const MADE_PROTOTYPES: &[(&str, &str, &str)] = &[
    ("constructor", "", "Array"),
    ("alias", "var A0 = Array; ", "A0"),
    ("global", "", "globalThis.Array"),
    ("data", "var D0 = [Array][0]; ", "D0"),
];

fn made_rows() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (maker, make) in MAKERS {
        for (spelling, decl, value) in MADE_PROTOTYPES {
            for (target, write) in HERITAGE_TARGETS {
                let write = write.replace("X0", "o").replace("F0", F);
                let patch = format!("{decl}{} {write}", make.replace("P0", value));
                rows.push((format!("{maker}:{spelling}:{target}"), patch));
            }
        }
    }
    rows
}

#[test]
fn a_value_made_with_a_realm_prototype_is_judged_as_that_realm_path() {
    let rows = made_rows();
    let mut all: Vec<(String, String)> = rows
        .iter()
        .map(|(label, patch)| (label.clone(), patched_after_rotator(JOIN_TEST, patch)))
        .collect();
    all.extend(
        rows.iter()
            .map(|(label, patch)| (format!("fold-{label}"), format!("{patch} {FOLD_JOIN}"))),
    );
    check(all);
}
