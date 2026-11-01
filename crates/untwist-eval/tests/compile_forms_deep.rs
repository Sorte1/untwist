use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::eval_script_in;
use untwist_eval::test_hooks::{compile_census, frozen, record_units, CompileCensus, UnitRecord};

const PARSE_STACK: usize = 1 << 30;
const RUNS: usize = 99_990;
const BAILS: usize = 150_000;
const NESTING: &str = "nesting depth cap exceeded";

fn records(source: &str) -> (Vec<UnitRecord>, CompileCensus) {
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, source));
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics.first()
    );
    let program = &parsed.program;
    let run = || eval_script_in(program, Budget::new(50_000_000), |_, _| ());
    let mut units = Vec::new();
    let census = compile_census(|| units = record_units(run));
    (units, census)
}

fn on_a_2_mib_thread<R: Send + 'static>(body: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(body)
        .expect("a thread")
        .join()
        .expect("the row's thread finished")
}

fn both(shape: fn(usize) -> String, levels: usize) -> (UnitRecord, u64) {
    on_a_2_mib_thread(move || {
        let source = shape(levels);
        let (mut compiled, census) = records(&source);
        frozen("compile_forms_deep", &source, &compiled);
        (compiled.remove(0), census.calls)
    })
}

fn deep_row(shape: fn(usize) -> String, value: &str) {
    let (record, calls) = both(shape, RUNS);
    assert_eq!(record.bail, None, "{record:?}");
    assert_eq!(record.values, [value]);
    assert!(calls > 0);
    let (record, calls) = both(shape, BAILS);
    assert_eq!(record.bail.as_deref(), Some(NESTING));
    assert!(calls > 0);
}

fn objects(levels: usize) -> String {
    format!(
        "(() => ({}1{}))();",
        "{a: ".repeat(levels),
        "}".repeat(levels)
    )
}

fn templates(levels: usize) -> String {
    format!(
        "(() => {}1{})();",
        "`${".repeat(levels),
        "}`".repeat(levels)
    )
}

fn news(levels: usize) -> String {
    format!(
        "var F = function () {{ return F; }}; (() => {}F)() === F;",
        "new ".repeat(levels)
    )
}

fn coalescing(levels: usize) -> String {
    format!("var x; (() => {}1)();", "x ??= ".repeat(levels))
}

fn arrays(levels: usize) -> String {
    format!(
        "(() => {}1{})().length;",
        "[".repeat(levels),
        "]".repeat(levels)
    )
}

fn deletes(levels: usize) -> String {
    format!(
        "var o = {{}}; (() => {}'k'{})();",
        "delete o[".repeat(levels),
        "]".repeat(levels)
    )
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_object_literal_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(objects, "object");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_template_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(templates, "string:1");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_new_chain_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(news, "bool:true");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_logical_assignment_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(coalescing, "number:3ff0000000000000");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_array_nest_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(arrays, "number:3ff0000000000000");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_delete_nest_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(deletes, "bool:true");
}

fn optional_chain(links: usize) -> String {
    format!(
        "var o = {{}}; o.a = o; o.f = () => 7; (() => o?.a{}.f())();",
        ".a".repeat(links)
    )
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the walker's chain links overflow a debug 2 MiB thread"
)]
fn a_15000_link_optional_chain_runs_compiled_as_far_as_the_walker() {
    let (record, calls) = both(optional_chain, 15_000);
    assert_eq!(record.values, ["number:401c000000000000"]);
    assert_eq!(calls, 2);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 100,000 links on a 2 MiB thread"
)]
fn a_100000_link_optional_chain_compiles_runs_and_drops_on_a_2_mib_thread() {
    let (units, census) = on_a_2_mib_thread(|| records(&optional_chain(100_000)));
    assert_eq!(units[0].values, ["number:401c000000000000"]);
    assert_eq!(census.calls, 2);
}

fn assignments(levels: usize) -> String {
    format!("var x; (() => {}1)();", "x = ".repeat(levels))
}

fn additions(levels: usize) -> String {
    format!("var x = 0; (() => {}1)();", "x += ".repeat(levels))
}

fn member_targets(levels: usize) -> String {
    format!(
        "var o = {{}}; (() => {}'k'{})();",
        "o[".repeat(levels),
        "] = 'k'".repeat(levels)
    )
}

fn computed_keys(levels: usize) -> String {
    format!(
        "(() => ({}1{}))();",
        "{[".repeat(levels),
        "]: 1}".repeat(levels)
    )
}

fn new_arguments(levels: usize) -> String {
    format!(
        "var F = function (x) {{ this.x = x; }}; (() => {}1{})();",
        "new F(".repeat(levels),
        ")".repeat(levels)
    )
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_assignment_chain_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(assignments, "number:3ff0000000000000");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_compound_assignment_chain_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(additions, "number:3ff0000000000000");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_member_target_nest_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(member_targets, "string:k");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_computed_key_nest_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(computed_keys, "object");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_new_argument_nest_compiles_runs_and_drops_on_a_2_mib_thread() {
    deep_row(new_arguments, "object");
}

fn call_chain(links: usize) -> String {
    format!(
        "var o = {{}}; o.f = () => o; o.v = 7; (() => o{}.v)();",
        "?.f()".repeat(links)
    )
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 100,000 links on a 2 MiB thread"
)]
fn a_100000_link_call_chain_compiles_runs_and_drops_on_a_2_mib_thread() {
    let (units, census) = on_a_2_mib_thread(|| records(&call_chain(100_000)));
    assert_eq!(units[0].values, ["number:401c000000000000"]);
    assert_eq!(census.calls, 100_001);
}
