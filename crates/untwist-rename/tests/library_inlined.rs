use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::library::shape::UnitKind;
use untwist_rename::library::table::{exact_key, loose_key, Role, RowData, Table};
use untwist_rename::library::{match_with, unit_records, LibraryNames, UnitRecord};

fn records(source: &str) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    unit_records(&parsed.program, &sem.scoping)
}

fn unit_at(source: &str, marker: &str) -> UnitRecord {
    let start = source.find(marker).expect("the marker is in the source") as u32;
    records(source)
        .into_iter()
        .filter(|record| record.kind == UnitKind::Function && record.span.start <= start)
        .max_by_key(|record| record.span.start)
        .expect("a function holds the marker")
}

fn same_shape(left: (&str, &str), right: (&str, &str)) -> bool {
    let (left, right) = (unit_at(left.0, left.1), unit_at(right.0, right.1));
    left.exact == right.exact && left.loose == right.loose
}

const WRAPPER: &str = "var keys = {}; var hasKeys;
function requireKeys() { if (hasKeys) return keys; hasKeys = 1; keys.all = Object.keys; return keys; }
requireKeys();";

const INLINED: &str = "var keys = {}; var hasKeys;
(function () { if (hasKeys) return keys; hasKeys = 1; keys.all = Object.keys; })();";

#[test]
fn an_inlined_wrapper_without_its_last_return_shapes_as_the_wrapper() {
    assert!(same_shape((WRAPPER, "if (hasKeys)"), (INLINED, "if (hasKeys)")));
}

#[test]
fn an_inlined_wrapper_that_ends_in_its_exports_assignment_shapes_as_the_wrapper() {
    let wrapper = "var m; var h;
function r() { if (h) return m; h = 1; return m = Object.keys; }";
    let inlined = "var m; var h;
(function () { if (h) { return m; } else { h = 1; m = Object.keys; } })();";
    assert!(same_shape((wrapper, "if (h)"), (inlined, "if (h)")));
}

#[test]
fn a_guard_that_does_not_set_its_flag_is_not_an_inlined_wrapper() {
    let wrapper = "var m; var h;
function r() { if (h) return m; m.a = 1; return m; }";
    let inlined = "var m; var h;
(function () { if (h) return m; m.a = 1; })();";
    assert!(!same_shape((wrapper, "if (h)"), (inlined, "if (h)")));
}

const OUTER_PLAIN: &str = "var a = {}; var hasA; var b; var hasB;
function requireB() { if (hasB) return b; hasB = 1; return b = Object.getOwnPropertyNames(Object.prototype).length; }
function requireA() { if (hasA) return a; hasA = 1; var own = requireB(); a.count = own + Object.keys(Object).length; return a; }
requireA();";

const OUTER_INLINED: &str = "var a = {}; var hasA; var b; var hasB;
(function () { if (hasA) return a; hasA = 1; var own = function () { if (hasB) return b; hasB = 1; return b = Object.getOwnPropertyNames(Object.prototype).length; }(); a.count = own + Object.keys(Object).length; })();";

#[test]
fn a_wrapper_inlined_into_its_caller_shapes_as_a_call() {
    assert!(same_shape(
        (OUTER_PLAIN, "if (hasA)"),
        (OUTER_INLINED, "if (hasA)")
    ));
    assert!(same_shape(
        (OUTER_PLAIN, "if (hasB)"),
        (OUTER_INLINED, "if (hasB)")
    ));
}

fn row(source: &str, marker: &str, module: &str, own: &str, outer: &[&str]) -> RowData {
    let record = unit_at(source, marker);
    RowData {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label: "core-js".to_string(),
        versions: "3.39.0".to_string(),
        module: module.to_string(),
        role: Role::Wrapper,
        own: Some(own.to_string()),
        inner: record.inner.iter().map(|_| None).collect(),
        outer: outer.iter().map(|name| Some(name.to_string())).collect(),
        specific: record.specific.max(2),
    }
}

fn table() -> Table {
    Table::from_rows(&[
        row(
            OUTER_PLAIN,
            "if (hasB)",
            "internals/b",
            "requireB",
            &["hasB", "b"],
        ),
        row(
            OUTER_PLAIN,
            "if (hasA)",
            "internals/a",
            "requireA",
            &["hasA", "a", "requireB"],
        ),
    ])
}

fn matched(table: &Table, source: &str) -> (LibraryNames, Vec<(String, String)>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let found = match_with(table, &parsed.program, &sem.scoping);
    let mut names: Vec<(String, String)> = found
        .names
        .iter()
        .map(|(symbol, name)| (sem.scoping.symbol_name(*symbol).to_string(), name.clone()))
        .collect();
    names.sort();
    (found, names)
}

#[test]
fn an_inlined_caller_matches_and_its_stand_in_gets_no_name() {
    let (found, names) = matched(&table(), &OUTER_INLINED.replace("hasA", "x").replace("hasB", "y"));
    let mut modules: Vec<String> = found.hits.iter().flat_map(|hit| hit.modules.clone()).collect();
    modules.sort();
    assert_eq!(modules, ["internals/a", "internals/b"], "{:?}", found.hits);
    assert!(names.contains(&("x".to_string(), "hasA".to_string())), "{names:?}");
    assert!(names.contains(&("y".to_string(), "hasB".to_string())), "{names:?}");
    assert!(!names.iter().any(|(_, name)| name == "requireB"), "{names:?}");
}

#[test]
fn an_inlined_caller_whose_inlined_wrapper_differs_is_not_a_match() {
    let changed = OUTER_INLINED.replace("Object.prototype", "Array.prototype");
    let (found, _) = matched(&table(), &changed);
    assert!(found.hits.is_empty(), "{:?}", found.hits);
}

#[test]
fn the_comment_for_an_inlined_wrapper_sits_above_its_statement() {
    let alloc = Allocator::default();
    let source = OUTER_INLINED.replace("hasA", "x").replace("hasB", "y");
    let mut parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let found = match_with(&table(), &parsed.program, &sem.scoping);
    let hits: Vec<&untwist_rename::library::Hit> = found.hits.iter().collect();
    untwist_rename::library::annotate::annotate(&mut parsed.program, &hits, &alloc);
    let printed = untwist_core::print::print(&parsed.program, untwist_core::print::PrintStyle::Pretty);
    let comment = printed
        .find("// core-js 3.39: internals/a.js")
        .expect("the inlined caller is commented");
    let call = printed.find("(function").expect("the call is printed");
    assert!(comment < call, "{printed}");
    assert!(!printed[comment..call].contains("if (x)"), "{printed}");
}
