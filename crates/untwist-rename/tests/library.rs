use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::library::table::{exact_key, loose_key, Role, RowData, Table};
use untwist_rename::library::{match_with, unit_records, UnitRecord};

fn records(source: &str) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    unit_records(&parsed.program, &sem.scoping)
}

fn first_function(source: &str) -> UnitRecord {
    records(source)
        .into_iter()
        .find(|record| record.kind == untwist_rename::library::shape::UnitKind::Function)
        .expect("the source has a function")
}

fn same_shape(left: &str, right: &str) -> bool {
    let (left, right) = (first_function(left), first_function(right));
    left.exact == right.exact && left.loose == right.loose
}

#[test]
fn identifiers_and_declaration_kinds_do_not_change_the_shape() {
    assert!(same_shape(
        "function a(b) { var c = b.x; return c(\"y\"); }",
        "function q(w) { const e = w.x; return e(\"y\"); }",
    ));
    assert!(same_shape(
        "function a(b) { let c = b; return c; }",
        "function a(b) { var c = b; return c; }",
    ));
}

#[test]
fn constants_keys_globals_and_operators_change_the_shape() {
    let base = "function a(b) { return b.x + \"y\"; }";
    assert!(!same_shape(base, "function a(b) { return b.z + \"y\"; }"));
    assert!(!same_shape(base, "function a(b) { return b.x + \"z\"; }"));
    assert!(!same_shape(base, "function a(b) { return b.x - \"y\"; }"));
    assert!(!same_shape(
        "function a() { return Object; }",
        "function a() { return Array; }"
    ));
    assert!(!same_shape(
        "function a(b, c) { return b(c); }",
        "function a(b, c) { return c(b); }"
    ));
}

#[test]
fn minifier_spellings_share_one_shape() {
    assert!(same_shape(
        "function a(b) { if (b) { return 1; } else { return 2; } }",
        "function a(b) { if (b) return 1; return 2; }",
    ));
    assert!(same_shape(
        "function a(b) { return b[\"key\"]; }",
        "function a(b) { return b.key; }"
    ));
    assert!(same_shape(
        "function a(b) { return \"function\" == typeof b; }",
        "function a(b) { return typeof b === \"function\"; }",
    ));
    assert!(same_shape(
        "function a(b) { return b === void 0; }",
        "function a(b) { return b === undefined; }"
    ));
    assert!(same_shape(
        "function a() { return !0; }",
        "function a() { return true; }"
    ));
    assert!(same_shape(
        "const a = (b) => b + 1;",
        "const a = function (b) { return b + 1; };"
    ));
    assert!(same_shape(
        "var x; function a(b) { x = b; return x; }",
        "var x; function a(b) { return x = b; }",
    ));
    assert!(same_shape(
        "function a(b) { b(), b(); }",
        "function a(b) { b(); b(); }"
    ));
    assert!(same_shape(
        "function a(b) { return c; var c; }",
        "function a(b) { var c; return c; }"
    ));
}

#[test]
fn an_else_that_does_not_follow_an_exit_stays_an_else() {
    assert!(!same_shape(
        "function a(b, c) { if (b) { c(); } else { b(); } }",
        "function a(b, c) { if (b) { c(); } b(); }",
    ));
}

#[test]
fn the_program_prefix_key_agrees_with_the_unit_key() {
    let source = "function outer(a) { return function inner(b) { return a(b, \"k\"); }; }";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let shaped = untwist_rename::library::shape::shape_program(&parsed.program, &sem.scoping);
    for unit in &shaped.units {
        let exact = untwist_rename::library::shape::exact_shape(unit, &sem.scoping);
        assert_eq!(unit.loose_key(&shaped.prefix), exact.loose);
    }
}

const GET_BUILT_IN: &str = "function wrapper() {
  if (flag) { return value; }
  flag = 1;
  var g = requireGlobal();
  var c = requireCallable();
  value = function (n, m) {
    return arguments.length < 2 ? (c(g[n]) ? g[n] : undefined) : g[n] && g[n][m];
  };
  return value;
}
var flag, value;
function requireGlobal() { return globalThis; }
function requireCallable() { return function (it) { return typeof it == \"function\"; }; }
";

fn row_for(source: &str, module: &str, own: &str, inner: &[&str], outer: &[&str]) -> RowData {
    let record = records(source)
        .into_iter()
        .find(|record| record.binding.as_deref() == Some("wrapper"))
        .expect("the wrapper is a unit");
    let names = |list: &[&str]| list.iter().map(|name| Some(name.to_string())).collect();
    RowData {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label: "core-js".to_string(),
        versions: "3.39.0".to_string(),
        module: module.to_string(),
        role: Role::Wrapper,
        own: Some(own.to_string()),
        inner: names(inner),
        outer: names(outer),
        specific: record.specific,
    }
}

fn names_after(table: &Table, source: &str) -> Vec<(String, String)> {
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
    names
}

const INNER: [&str; 5] = [
    "globalThis",
    "isCallable",
    "namespace",
    "method",
    "argument",
];
const OUTER: [&str; 4] = [
    "hasRequiredGetBuiltIn",
    "getBuiltIn",
    "requireGlobalThis",
    "requireIsCallable",
];

#[test]
fn an_exact_wrapper_match_names_the_wrapper_its_locals_and_its_dependencies() {
    let row = row_for(
        GET_BUILT_IN,
        "internals/get-built-in",
        "requireGetBuiltIn",
        &INNER[..4],
        &OUTER,
    );
    let table = Table::from_rows(&[row]);
    let renamed = GET_BUILT_IN
        .replace("wrapper", "a")
        .replace("flag", "b")
        .replace("value", "d");
    let names = names_after(&table, &renamed);
    for expected in [
        ("a", "requireGetBuiltIn"),
        ("b", "hasRequiredGetBuiltIn"),
        ("d", "getBuiltIn"),
        ("g", "globalThis"),
        ("n", "namespace"),
        ("requireGlobal", "requireGlobalThis"),
    ] {
        assert!(
            names.contains(&(expected.0.to_string(), expected.1.to_string())),
            "{names:?}"
        );
    }
}

#[test]
fn a_changed_constant_is_not_a_match() {
    let row = row_for(
        GET_BUILT_IN,
        "internals/get-built-in",
        "requireGetBuiltIn",
        &INNER[..4],
        &OUTER,
    );
    let table = Table::from_rows(&[row]);
    let changed = GET_BUILT_IN.replace("arguments.length < 2", "arguments.length < 3");
    assert!(names_after(&table, &changed).is_empty());
}

#[test]
fn rows_that_disagree_on_the_name_of_the_same_shape_name_nothing() {
    let first = row_for(
        GET_BUILT_IN,
        "internals/get-built-in",
        "requireGetBuiltIn",
        &INNER[..4],
        &OUTER,
    );
    let second = row_for(
        GET_BUILT_IN,
        "internals/other",
        "requireOther",
        &INNER[..4],
        &OUTER,
    );
    let table = Table::from_rows(&[first, second]);
    assert!(names_after(&table, GET_BUILT_IN).is_empty());
}

#[test]
fn rows_that_agree_on_the_name_keep_only_the_shared_names() {
    let first = row_for(
        GET_BUILT_IN,
        "internals/get-built-in",
        "requireGetBuiltIn",
        &INNER[..4],
        &OUTER,
    );
    let mut other = INNER[..4].to_vec();
    other[0] = "global";
    let second = row_for(
        GET_BUILT_IN,
        "internals/get-built-in-copy",
        "requireGetBuiltIn",
        &other,
        &OUTER,
    );
    let table = Table::from_rows(&[first, second]);
    let names = names_after(&table, GET_BUILT_IN);
    assert!(names.contains(&("wrapper".to_string(), "requireGetBuiltIn".to_string())));
    assert!(!names.iter().any(|(old, _)| old == "g"), "{names:?}");
}

#[test]
fn a_small_function_is_named_only_when_a_matched_unit_vouches_for_it() {
    let small = "function wrapper(it) { return it == null; }";
    let record = records(small)
        .into_iter()
        .find(|r| r.binding.as_deref() == Some("wrapper"))
        .expect("unit");
    let row = RowData {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label: "core-js".to_string(),
        versions: "3.39.0".to_string(),
        module: "internals/is-null-or-undefined".to_string(),
        role: Role::Function,
        own: Some("isNullOrUndefined".to_string()),
        inner: vec![Some("it".to_string())],
        outer: Vec::new(),
        specific: record.specific,
    };
    let table = Table::from_rows(&[row]);
    assert!(names_after(&table, small).is_empty());
}

#[test]
fn the_table_text_round_trips() {
    let row = row_for(
        GET_BUILT_IN,
        "internals/get-built-in",
        "requireGetBuiltIn",
        &INNER[..4],
        &OUTER,
    );
    let table = Table::from_rows(std::slice::from_ref(&row));
    let parsed = Table::parse(&table.write("core-js")).expect("the written table parses");
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.data(&parsed.rows[0]), row);
}

#[test]
fn a_split_initialiser_and_a_declared_one_share_a_shape() {
    assert!(same_shape(
        "function a(b) { var c; c = b.x; return c; }",
        "function a(b) { const c = b.x; return c; }",
    ));
    assert!(same_shape(
        "function a(b) { let c; c = b.x; c(); }",
        "function a(b) { var c = b.x; c(); }",
    ));
}

const BEFORE_PLACEMENT: &str = "function wrapper() {
  if (flag) { return value; }
  flag = 1;
  var g = requireGlobal();
  var c = requireCallable();
  value = function (n, m) {
    if (arguments.length < 2) {
      d = g[n];
      if (c(d)) { return d; } else { return undefined; }
    } else {
      return g[n] && g[n][m];
    }
    var d;
  };
  return value;
}";

const AFTER_PLACEMENT: &str = "function wrapper() {
  if (flag) { return value; }
  flag = 1;
  var g = requireGlobal();
  var c = requireCallable();
  value = function (n, m) {
    if (arguments.length < 2) {
      const d = g[n];
      if (c(d)) { return d; } else { return undefined; }
    } else {
      return g[n] && g[n][m];
    }
  };
  return value;
}";

#[test]
fn get_built_in_matches_before_and_after_declaration_placement() {
    let shape = |source: &str| {
        records(source)
            .into_iter()
            .find(|record| record.binding.as_deref() == Some("wrapper"))
            .map(|record| (record.exact, record.loose))
            .expect("the wrapper is a unit")
    };
    assert_eq!(shape(BEFORE_PLACEMENT), shape(AFTER_PLACEMENT));
}

fn gzip(text: &str) -> Vec<u8> {
    let mut data = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 2, 255];
    data.extend(miniz_oxide::deflate::compress_to_vec(text.as_bytes(), 6));
    data.extend([0u8; 8]);
    data
}

#[test]
fn a_table_loaded_at_run_time_is_matched_and_loads_once() {
    use untwist_rename::library::table::{load, loaded_ids, LibraryTables};
    let source = GET_BUILT_IN.replace("arguments.length < 2", "arguments.length < 7");
    let mut row = row_for(
        &source,
        "internals/runtime-only",
        "requireRuntimeOnly",
        &INNER[..4],
        &OUTER,
    );
    row.label = "runtime-demo".to_string();
    let text = Table::from_rows(&[row]).write("runtime-demo");
    let tables = LibraryTables::from_gz_bytes(&gzip(&text)).expect("the gzip table parses");
    assert_eq!(tables.ids(), ["runtime-demo".to_string()]);
    assert!(load(tables));
    assert!(loaded_ids().contains(&"runtime-demo".to_string()));
    let again = LibraryTables::from_gz_bytes(&gzip(&text)).expect("the gzip table parses");
    assert!(!load(again));
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let found = untwist_rename::library::match_program(&parsed.program, &sem.scoping);
    assert!(
        found.hits.iter().any(|hit| hit.label == "runtime-demo"),
        "{:?}",
        found.hits
    );
}

#[test]
fn a_shape_two_libraries_share_names_both_and_keeps_only_agreed_names() {
    let first = row_for(
        GET_BUILT_IN,
        "helpers/possibleConstructorReturn",
        "requireGetBuiltIn",
        &INNER[..4],
        &OUTER,
    );
    let mut second = row_for(
        GET_BUILT_IN,
        "possible_constructor_return",
        "requireOther",
        &INNER[..4],
        &OUTER,
    );
    second.label = "@swc/helpers".to_string();
    second.versions = "0.5.24".to_string();
    let table = Table::from_rows(&[first, second]);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, GET_BUILT_IN);
    let sem = build_scoping(&parsed.program);
    let found = match_with(&table, &parsed.program, &sem.scoping);
    let hit = found
        .hits
        .iter()
        .find(|hit| hit.outermost)
        .expect("the wrapper is a hit");
    let text = untwist_rename::library::annotate::comment_text(hit);
    assert!(
        text.contains("same code in @swc/helpers 0.5: possible_constructor_return.js"),
        "{text}"
    );
    let names = names_after(&table, GET_BUILT_IN);
    assert!(!names.iter().any(|(old, _)| old == "wrapper"), "{names:?}");
    assert!(
        names.contains(&("g".to_string(), "globalThis".to_string())),
        "{names:?}"
    );
}
