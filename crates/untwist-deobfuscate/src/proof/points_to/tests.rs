use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use rustc_hash::FxHashMap;

use super::{points_to_dump, points_to_sites};

fn with_program<T>(
    source: &str,
    run: impl FnOnce(&oxc_ast::ast::Program<'_>, &oxc_semantic::Scoping) -> T,
) -> T {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, SourceType::cjs()).parse();
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let scoping = semantic.into_scoping();
    run(&parsed.program, &scoping)
}

fn site_texts(source: &str) -> Vec<String> {
    with_program(source, |program, scoping| {
        points_to_sites(program, scoping, &FxHashMap::default(), &[])
            .iter()
            .map(|site| source[site.span().start as usize..site.span().end as usize].to_string())
            .collect()
    })
}

fn dump(source: &str, shuffle: Option<u64>) -> String {
    with_program(source, |program, scoping| {
        points_to_dump(
            program,
            scoping,
            &FxHashMap::default(),
            (Span::new(0, source.len() as u32), shuffle),
        )
    })
}

#[test]
fn a_store_through_an_alias_of_a_prototype_is_a_site() {
    let sites = site_texts("var P = Array.prototype; P.push = function () {};");
    assert_eq!(sites, ["P.push"]);
}

#[test]
fn a_store_into_a_program_object_is_not_a_site() {
    let sites = site_texts("var o = {}; var p = o; p.push = 1; p[k] = 2;");
    assert!(sites.is_empty(), "{sites:?}");
}

#[test]
fn a_prototype_passed_to_a_helper_reaches_its_parameter() {
    let sites = site_texts(
        "function put(o, k, v) { o[k] = v; } put(String.prototype, 'x', 1); put({}, 'y', 2);",
    );
    assert_eq!(sites, ["o[k]"]);
}

#[test]
fn a_map_from_a_helper_is_tracked_per_allocation() {
    let sites =
        site_texts("function mk() { return new Map(); } var m = mk(); m.set(a, b); m[c] = 1;");
    assert!(sites.is_empty(), "{sites:?}");
}

#[test]
fn a_define_property_through_a_bound_alias_is_a_site() {
    let sites =
        site_texts("var d = Object.defineProperty; d(Array.prototype, 'push', { value: 1 });");
    assert_eq!(sites.len(), 1, "{sites:?}");
}

#[test]
fn a_definer_called_through_call_is_a_site() {
    let sites = site_texts(
        "var d = Object.defineProperty; d.call(null, Array.prototype, 'shift', { value: 1 });",
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
}

#[test]
fn an_escaped_function_parameter_may_hold_an_escaped_builtin() {
    let sites =
        site_texts("function f(o) { o.push = 1; } window.handler = f; host(Array.prototype);");
    assert_eq!(sites, ["o.push"]);
}

#[test]
fn an_escaped_function_parameter_holds_no_builtin_that_stayed_inside() {
    let sites = site_texts("function f(o) { o.push = 1; } window.handler = f;");
    assert!(sites.is_empty(), "{sites:?}");
}

#[test]
fn a_method_call_binds_this_to_its_receiver_only() {
    let sites = site_texts(
        "(function () { var o = { m: function (k) { this[k] = 1; } }; o.m(host()); })();",
    );
    assert!(sites.is_empty(), "{sites:?}");
}

#[test]
fn a_detached_method_call_binds_this_to_the_global_object() {
    let sites = site_texts(
        "(function () { var o = { m: function (k) { this[k] = 1; } }; var g = o.m; g(host()); })();",
    );
    assert_eq!(sites, ["this[k]"]);
}

#[test]
fn a_getter_runs_with_an_inheriting_receiver() {
    let sites = site_texts(
        "(function () { var p = { get g() { this.t.push = 1; } }; var a = Object.create(p); a.t = Array.prototype; a.g; })();",
    );
    assert_eq!(sites, ["this.t.push"]);
}

#[test]
fn a_parent_constructor_result_becomes_the_derived_this() {
    let sites = site_texts(
        "(function () { class A { constructor() { return Array.prototype; } } class B extends A { constructor() { super(); this.push = 1; } } new B(); })();",
    );
    assert_eq!(sites, ["this.push"]);
}

#[test]
fn a_primitive_receiver_reads_the_string_prototype() {
    let out = dump("var c = ''.constructor; c.prototype.x = 1;", None);
    assert!(out.contains("builtin StringCtor"), "{out}");
    let sites = site_texts("var c = ''.constructor; c.prototype.x = 1;");
    assert_eq!(sites, ["c.prototype.x"]);
}

#[test]
fn sloppy_this_and_function_this_reach_the_global_object() {
    let sites = site_texts(
        "var g = Function('return this')(); g.String = 1; (function () { this.Array = 2; })();",
    );
    assert_eq!(sites, ["g.String", "this.Array"]);
}

#[test]
fn concatenated_keys_resolve_to_names() {
    let out = dump("var P = Array.prototype; P['pu' + 'sh'] = 1;", None);
    assert!(out.contains("string \"push\""), "{out}");
}

#[test]
fn direct_eval_falls_back_for_the_whole_program() {
    with_program("eval('x'); var o = {};", |program, scoping| {
        let sites = points_to_sites(program, scoping, &FxHashMap::default(), &[]);
        assert_eq!(sites.len(), 1);
    });
}

const SHUFFLED: &str = "
var P = Array.prototype, S = String.prototype;
function put(o, k, v) { o[k] = v; return o; }
var helpers = { put: put, list: [P, S] };
helpers.list.forEach(function (proto) { put(proto, 'x' + 'y', function () {}); });
var b = put.bind(null, {});
b('a', 1);
var m = new Map(); m.set('k', S);
var w = m.get('k'); w.trim = null;
class C { constructor() { this.v = [1]; } get g() { return this.v; } }
var c = new C(); c.g.push(P);
for (var key in helpers) { helpers[key]; }
";

#[test]
fn shuffled_constraint_order_gives_identical_sets_and_sites() {
    let first = dump(SHUFFLED, None);
    assert!(first.contains("site"), "{first}");
    for seed in [1, 7, 2026, 99_991] {
        assert_eq!(dump(SHUFFLED, Some(seed)), first, "seed {seed}");
    }
}

#[test]
fn a_spent_budget_refuses_through_one_fallback_site() {
    with_program(SHUFFLED, |program, scoping| {
        let decoded = FxHashMap::default();
        let mut generator = super::generate::Gen::new(program, scoping, &decoded, &[]);
        generator.run(program);
        generator.g.cap_for_tests(3);
        generator.g.solve(None);
        assert!(generator.g.exhausted());
        let sites = generator.sites();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].span(), Span::default());
    });
}

#[test]
fn code_built_at_run_time_reaches_every_built_in() {
    let sites = site_texts("var g = Function('return this')(); var k = f(); g[k][k] = 1;");
    assert_eq!(sites, ["g[k][k]"]);
    with_program(
        "var g = Function('return this')(); var k = f(); g[k][k] = 1;",
        |program, scoping| {
            let sites = points_to_sites(program, scoping, &FxHashMap::default(), &[]);
            let used = crate::proof::UsedNames::default();
            assert!(crate::proof::alias_refusal(&sites, &used, program.source_text).is_some());
        },
    );
}

#[test]
fn joining_arrays_of_arrays_settles_within_the_budget() {
    let source = "var a = [[1], [[2]]]; var s = '' + a; for (var i = 0; i < a.length; i++) { console.log(a[i] + s); }";
    let out = dump(source, None);
    assert!(!out.contains("exhausted"), "{out}");
}

#[test]
fn a_prototype_chain_that_loops_settles_within_the_budget() {
    let source = "function mk(p) { return Object.create(p); } var o = mk(mk({ a: 1 })); for (var k in o) { o[k]; } var c = Object.assign({}, o); Object.keys(o);";
    let out = dump(source, None);
    assert!(!out.contains("exhausted"), "{out}");
}

const SMALL_INPUTS: &[&str] = &[
    "var s = [new H()].join();",
    "var s = '' + [x];",
    "var a = Array.from(iterable);",
    "var m = new Map(iterable);",
    "var s = String.raw`${x}`;",
    "var it = { [Symbol.iterator]: function () { return [][Symbol.iterator](); } }; for (var v of it) { v; }",
    "var p = Object.getPrototypeOf([][Symbol.iterator]());",
];

const SMALL_INPUT_STEPS: u64 = 20_000;

#[test]
fn small_inputs_settle_in_few_steps() {
    for input in SMALL_INPUTS {
        let source = format!("function H() {{}} var x = new H(); var iterable = [[x, x]]; {input}");
        with_program(&source, |program, scoping| {
            let decoded = FxHashMap::default();
            let mut generator = super::generate::Gen::new(program, scoping, &decoded, &[]);
            generator.run(program);
            generator.g.solve(None);
            assert!(!generator.g.exhausted(), "{input}");
            let steps = generator.g.steps();
            assert!(steps < SMALL_INPUT_STEPS, "{input}: {steps} steps");
        });
    }
}

const SELF_REACHING_BOUND_CALLS: &[&str] = &[
    "document.defaultView.Object.defineProperty(String.prototype, m, { value: 1 });",
    "function d(c) { return String.fromCharCode(0x68) + c; } document.defaultView.Object.defineProperty(String.prototype, m, { value: 1 }); d(1);",
];

const HELD_OVERSHOOT: u64 = 64;

#[test]
fn a_bound_call_that_reaches_itself_ends_within_the_budget() {
    for input in SELF_REACHING_BOUND_CALLS {
        with_program(input, |program, scoping| {
            let decoded = FxHashMap::default();
            let mut generator = super::generate::Gen::new(program, scoping, &decoded, &[]);
            generator.run(program);
            generator.g.solve(None);
            let (held, cap) = generator.g.held_and_cap();
            assert!(
                held <= cap + HELD_OVERSHOOT,
                "{input}: held {held} of {cap}"
            );
            let sites = generator.sites();
            if generator.g.exhausted() {
                assert_eq!(sites.len(), 1, "{input}");
                assert_eq!(sites[0].span(), Span::default(), "{input}");
            }
        });
    }
}

#[test]
fn a_spent_budget_stops_the_work_inside_one_handler() {
    let args = vec!["1"; 399].join(", ");
    let source = format!("var a = []; var p = a.push; p.call(a, {args});");
    with_program(&source, |program, scoping| {
        let decoded = FxHashMap::default();
        let mut generator = super::generate::Gen::new(program, scoping, &decoded, &[]);
        generator.run(program);
        generator.g.cap_elements_for_tests(300);
        generator.g.solve(None);
        assert!(generator.g.exhausted());
        let stores = generator.g.stores_issued();
        assert!(
            stores < 16,
            "{stores} stores issued after the budget ran out"
        );
    });
}

const ESCAPED_BEFORE_A_WORLD_SET: &[(&str, &[&str])] = &[
    (
        "var P = Object.create(Object); host(P); var q = host2(); if (c) { q = P; } q.defineProperty(String.prototype, 'x', { value: 1 });",
        &["q.defineProperty(String.prototype, 'x', { value: 1 })"],
    ),
    (
        "var o = {}; host(o); var q = host2(); if (c) { q = o; } Object.setPrototypeOf(q, Object); o.defineProperty(String.prototype, 'x', { value: 1 });",
        &[
            "Object.setPrototypeOf(q, Object)",
            "o.defineProperty(String.prototype, 'x', { value: 1 })",
        ],
    ),
    (
        "var o = {}; host(o); var q = host2(); if (c) { q = o; } q.__proto__ = Object; o.defineProperty(String.prototype, 'x', { value: 1 });",
        &[
            "q.__proto__",
            "o.defineProperty(String.prototype, 'x', { value: 1 })",
        ],
    ),
];

fn shuffled_site_texts(source: &str, shuffle: Option<u64>) -> Vec<String> {
    with_program(source, |program, scoping| {
        let decoded = FxHashMap::default();
        let mut generator = super::generate::Gen::new(program, scoping, &decoded, &[]);
        generator.run(program);
        generator.g.solve(shuffle);
        generator
            .sites()
            .iter()
            .map(|site| source[site.span().start as usize..site.span().end as usize].to_string())
            .collect()
    })
}

#[test]
fn an_object_that_escapes_before_it_reaches_a_world_set_keeps_its_prototype_definers() {
    for (source, site) in ESCAPED_BEFORE_A_WORLD_SET {
        for shuffle in [None, Some(1), Some(7), Some(2026), Some(99_991)] {
            let sites = shuffled_site_texts(source, shuffle);
            assert_eq!(sites, *site, "{source} {shuffle:?}");
        }
    }
}
