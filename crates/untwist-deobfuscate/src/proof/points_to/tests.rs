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
fn an_escaped_function_parameter_may_hold_a_builtin() {
    let sites = site_texts("function f(o) { o.push = 1; } window.handler = f;");
    assert_eq!(sites, ["o.push"]);
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
    with_program("var g = Function('return this')(); var k = f(); g[k][k] = 1;", |program, scoping| {
        let sites = points_to_sites(program, scoping, &FxHashMap::default(), &[]);
        let used = crate::proof::UsedNames::default();
        assert!(crate::proof::alias_refusal(&sites, &used, program.source_text).is_some());
    });
}
