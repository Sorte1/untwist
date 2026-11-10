use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{heap_budget, Budget};
use untwist_eval::heap::HeapLedger;
use untwist_eval::interp::{eval_script_decoding, eval_script_in, Interp};
use untwist_eval::test_hooks::{compile_census, frozen, record_units, CompileCensus, UnitRecord};

const UNSOUND_DEFAULT: &str = "unsupported construct: a parameter default that reads its own parameter, a later one or a binding of the function body";

struct Compared {
    default: Vec<UnitRecord>,
    census: CompileCensus,
}

fn compare(label: &str, source: &str, decoding: bool) -> Compared {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{label}: {:?}",
        parsed.diagnostics
    );
    let program = &parsed.program;
    let run = || match decoding {
        true => eval_script_decoding(program, Budget::default(), |_| ()),
        false => eval_script_in(program, Budget::default(), |_, _| ()),
    };
    let mut default = Vec::new();
    let census = compile_census(|| default = record_units(run));
    frozen(
        "compile_expressions",
        &format!("{label}\n{decoding}\n{source}"),
        &default,
    );
    Compared { default, census }
}

fn compiled(label: &str, source: &str, value: &str) -> CompileCensus {
    let compared = compare(label, source, false);
    assert!(
        compared.census.calls > 0,
        "{label}: no call entered compiled"
    );
    assert_eq!(compared.default.len(), 1, "{label}");
    assert_eq!(
        compared.default[0].values,
        [value],
        "{label}: {:?}",
        compared.default[0]
    );
    compared.census
}

fn bails(label: &str, source: &str, decoding: bool, bail: &str) -> CompileCensus {
    let compared = compare(label, source, decoding);
    assert!(
        compared.census.calls > 0,
        "{label}: no call entered compiled"
    );
    assert_eq!(compared.default[0].bail.as_deref(), Some(bail), "{label}");
    compared.census
}

#[test]
fn a_param_var_collision_and_its_closure_run_compiled() {
    compiled(
        "a param/var collision, node p",
        "var r; function f(a) { var a; r = (() => a)(); } f('p'); r;",
        "string:p",
    );
}

#[test]
fn a_closure_inside_a_catch_frame_runs_compiled() {
    compiled(
        "catch (e) { var e }, node v",
        "var r; try { throw 'c'; } catch (e) { var e = 'v'; r = (() => e)(); } r;",
        "string:v",
    );
}

#[test]
fn a_named_function_expressions_own_name_is_immutable() {
    compiled(
        "sloppy, node function,function,1",
        "var r; var f = function g(n) { r = typeof g + ',' + (g = 5, typeof g) + ',' + n; }; f(1); r;",
        "string:function,function,1",
    );
    compiled(
        "strict, node TypeError",
        "'use strict'; var f = function g() { g = 1; }; f();",
        "throw error:TypeError",
    );
}

#[test]
fn a_hoisted_function_runs_compiled_before_its_declaration() {
    compiled(
        "hoisting, node 6",
        "var r = 0; f(3); function f(x) { r = x * 2; } r;",
        "number:4018000000000000",
    );
}

#[test]
fn arguments_in_its_mapped_unmapped_and_arrow_forms() {
    compiled(
        "mapped, node 2,x,y",
        "var r; function f(a, b) { r = arguments.length + ',' + (arguments[1] = 'x', b) + ',' + (b = 'y', arguments[1]); } f(1, 2); r;",
        "string:2,x,y",
    );
    compiled(
        "strict, node 1",
        "'use strict'; var r; function f(a) { r = (arguments[0] = 9, a); } f(1); r;",
        "number:3ff0000000000000",
    );
    compiled(
        "an arrow reads its function's arguments, node 7",
        "var r; function f(a) { r = (() => arguments[0])(); } f(7); r;",
        "number:401c000000000000",
    );
}

#[test]
fn a_read_three_hops_out() {
    compiled(
        "three hops out, node 10",
        "var f = (a) => (b) => (c) => (d) => a + b + c + d; f(1)(2)(3)(4);",
        "number:4024000000000000",
    );
}

#[test]
fn global_reads_and_writes() {
    compiled(
        "sloppy, node 7",
        "var g = 1; function f() { g = g + 1; h = 5; } f(); g + h;",
        "number:401c000000000000",
    );
    compiled(
        "strict, node ReferenceError",
        "'use strict'; function f() { nope = 1; } f();",
        "throw error:ReferenceError",
    );
}

#[test]
fn typeof_of_bound_unbound_and_function_names() {
    let source = "var r; function f(x) { r = typeof x + typeof nope + typeof f; } f(1); r;";
    compiled(
        "typeof, node numberundefinedfunction",
        source,
        "string:numberundefinedfunction",
    );
    bails(
        "while decoding",
        source,
        true,
        "unresolved global binding: nope",
    );
}

#[test]
fn the_ledger_per_call_of_a_compiled_function_that_captures_its_frame() {
    let source =
        "var keep = []; function mk(i) { keep[i] = () => i; } mk(0); mk(1); mk(2); keep[2]();";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let program = &parsed.program;
    let session = || {
        let mut interp = Interp::new(program.source_text, Budget::default());
        let ledger = HeapLedger::new(heap_budget(source.len()));
        interp.set_heap(Some(ledger.clone()));
        let env = interp.global_env();
        interp.hoist(&program.body, &env).expect("hoists");
        let mut after = Vec::new();
        for statement in &program.body {
            interp.run_root_statement(statement).expect("runs");
            after.push((ledger.peak(), ledger.live(), interp.budget.steps_used()));
        }
        after
    };
    let mut default = Vec::new();
    let census = compile_census(|| {
        record_units(|| default = session());
    });
    frozen("compile_expressions", "captured frame", &default);
    assert_eq!(census.calls, 4, "mk three times and keep[2] once");
}

#[test]
fn a_default_that_reads_a_later_parameter_is_refused() {
    let compared = compare(
        "an arrow, node throws a ReferenceError",
        "var b = 'outer'; var f = (a = b, b) => a; f();",
        false,
    );

    assert_eq!(compared.default[0].bail.as_deref(), Some(UNSOUND_DEFAULT));
}

#[test]
fn a_closure_made_in_a_default_that_reads_a_later_parameter_is_refused() {
    let compared = compare(
        "a closure in a default, node throws a ReferenceError",
        "var r, b = 'outer'; function f(g = () => b, h = g(), b) { r = h + ',' + g(); } f(undefined, undefined, 'inner'); r;",
        false,
    );

    assert_eq!(compared.default[0].bail.as_deref(), Some(UNSOUND_DEFAULT));
}

#[test]
fn a_closure_made_before_a_let_reads_it_through_maybe() {
    let census = compiled(
        "Maybe, node inner",
        "var r, x = 'outer'; function f() { var g = () => x; let x = 'inner'; r = g(); } f(); r;",
        "string:inner",
    );
    assert!(census.maybe > 0, "Maybe: {census:?}");
}

#[test]
fn a_default_that_reads_arguments_reads_the_arguments_object_compiled() {
    compiled(
        "compiled, node object",
        "var r; function f(a = arguments) { r = typeof a; } f(); r;",
        "string:object",
    );
}

fn nested_arrows(depth: usize) -> String {
    format!("{}Math.PI{};", "(() => ".repeat(depth), ")()".repeat(depth))
}

#[test]
fn a_global_read_through_256_compiled_frames_bails_at_the_read() {
    compiled(
        "at 255 frames, node 3.141592653589793",
        &nested_arrows(255),
        "number:400921fb54442d18",
    );
    let census = bails(
        "at 256 frames",
        &nested_arrows(256),
        false,
        "scope chain too deep",
    );
    assert_eq!(census.calls, 256, "{census:?}");
}

fn within(node: (u32, u32), units: &[(u32, u32)]) -> bool {
    units
        .iter()
        .any(|unit| unit.0 <= node.0 && node.1 <= unit.1)
}

fn span_of(source: &str, from: &str, to: &str) -> (u32, u32) {
    let start = source.find(from).expect("the start marker");
    let end = start + source[start..].find(to).expect("the end marker") + to.len();
    (start as u32, end as u32)
}

#[test]
fn the_laziness_row_compiles_only_the_statements_that_ran() {
    let source = "var r = 0; function f(a = 1 + 2) { r = a; null.x; r = 'never' + (1 + 2 + 3); } try { f(5); } catch (e) {} r;";
    let compared = compare("laziness", source, false);
    let census = compared.census;
    assert_eq!(census.calls, 1);
    let body = span_of(source, "{ r = a;", "} try");
    let in_body =
        |spans: &[(u32, u32)]| spans.iter().filter(|span| within(**span, &[body])).count();
    assert_eq!(in_body(&census.unit_spans), 2, "{census:?}");
    assert_eq!(
        in_body(&census.node_spans),
        6,
        "r = a and null.x only, two statements and four nodes: {census:?}"
    );
    let never = span_of(source, "r = 'never'", ")");
    assert_eq!(in_body(&[never]), 1);
    assert!(census
        .node_spans
        .iter()
        .all(|node| !within(*node, &[never])));
    assert!(census
        .node_spans
        .iter()
        .all(|node| within(*node, &census.unit_spans)));
    assert_eq!(compared.default[0].values, ["number:4014000000000000"]);
}

#[test]
fn the_laziness_row_counts_an_untaken_arm_inside_a_statement_that_ran() {
    let census = compiled(
        "untaken arm, node 1",
        "var r; function f(c) { r = c ? 1 : 2 + 3; } f(true); r;",
        "number:3ff0000000000000",
    );
    let body = span_of(
        "var r; function f(c) { r = c ? 1 : 2 + 3; } f(true); r;",
        "r = c",
        ";",
    );
    let in_body = census
        .node_spans
        .iter()
        .filter(|span| within(**span, &[body]))
        .count();
    assert_eq!(in_body, 6, "one statement and five nodes: {census:?}");
    assert_eq!(
        census.constants, 1,
        "the arm `2 + 3` is one folded node: {census:?}"
    );
}
