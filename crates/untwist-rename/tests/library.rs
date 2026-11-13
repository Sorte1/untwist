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
        .filter(|record| record.kind == untwist_rename::library::shape::UnitKind::Function)
        .max_by_key(|record| (record.span.end - record.span.start, std::cmp::Reverse(record.span.start)))
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
    ] {
        assert!(
            names.contains(&(expected.0.to_string(), expected.1.to_string())),
            "{names:?}"
        );
    }
    assert!(
        !names.iter().any(|(old, _)| old == "requireGlobal"),
        "a function defined outside the wrapper is named only by its own match: {names:?}"
    );
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
    assert!(hit.shared, "{hit:?}");
    assert!(
        text.contains("@swc/helpers 0.5: possible_constructor_return.js")
            && text.contains("core-js 3.39: helpers/possibleConstructorReturn.js"),
        "{text}"
    );
    let names = names_after(&table, GET_BUILT_IN);
    assert!(!names.iter().any(|(old, _)| old == "wrapper"), "{names:?}");
    assert!(
        names.contains(&("g".to_string(), "globalThis".to_string())),
        "{names:?}"
    );
}

fn program_shape(source: &str) -> (u128, (u32, u64)) {
    records(source)
        .into_iter()
        .find(|record| record.kind == untwist_rename::library::shape::UnitKind::Program)
        .map(|record| (record.exact, record.loose))
        .expect("the program is a unit")
}

#[test]
fn an_arrow_that_reads_this_arguments_super_or_new_target_is_not_a_function() {
    for body in ["this", "arguments", "eval(\"this\")"] {
        assert!(
            !same_shape(
                &format!("const a = function () {{ return {body}; }};"),
                &format!("const a = () => {body};"),
            ),
            "{body}"
        );
    }
    assert!(!same_shape(
        "const a = function () { return () => this; };",
        "const a = () => () => this;",
    ));
    assert!(!same_shape(
        "class A extends B { m() { const f = function () { return new.target; }; } }",
        "class A extends B { m() { const f = () => new.target; } }",
    ));
    assert!(same_shape(
        "const a = function (b) { return b.c; };",
        "const a = (b) => b.c;"
    ));
}

#[test]
fn absent_for_slots_keep_their_position() {
    let shapes = [
        "function f(a, b) { for (; a; b) {} }",
        "function f(a, b) { for (a; b;) {} }",
        "function f(a, b) { for (a;; b) {} }",
    ];
    for (index, left) in shapes.iter().enumerate() {
        for right in &shapes[index + 1..] {
            assert!(!same_shape(left, right), "{left} = {right}");
        }
    }
}

#[test]
fn labels_are_told_apart_by_first_use_not_by_name() {
    let jump = |target: &str| {
        format!("function f(c) {{ outer: for (;;) {{ inner: for (;;) {{ if (c) continue {target}; c(); }} }} }}")
    };
    assert!(!same_shape(&jump("outer"), &jump("inner")));
    assert!(same_shape(
        &jump("outer"),
        &jump("outer").replace("outer", "x").replace("inner", "y")
    ));
}

#[test]
fn the_try_block_and_the_finally_block_stay_apart() {
    assert!(!same_shape(
        "function f(a, b) { try { a(); } finally { b(); } }",
        "function f(a, b) { try { a(); b(); } finally {} }",
    ));
}

#[test]
fn declaration_kinds_stay_apart_where_they_change_behaviour() {
    let pairs = [
        (
            "function f(a, s) { for (const i of a) { var x = i; s.push(() => x); } }",
            "function f(a, s) { for (const i of a) { let x = i; s.push(() => x); } }",
        ),
        (
            "function f(a, s) { for (const i of a) { var x; if (i) x = i; s(x); } }",
            "function f(a, s) { for (const i of a) { let x; if (i) x = i; s(x); } }",
        ),
        (
            "function f(a) { var x = a; x = 2; return x; }",
            "function f(a) { const x = a; x = 2; return x; }",
        ),
        (
            "function f(a) { g(); var x = a; function g() { return x; } return x; }",
            "function f(a) { g(); const x = a; function g() { return x; } return x; }",
        ),
        (
            "function f(a, k) { switch (k) { case 1: var x = a; case 2: return x; } }",
            "function f(a, k) { switch (k) { case 1: let x = a; case 2: return x; } }",
        ),
        (
            "function f(a) { var x = x + a; return x; }",
            "function f(a) { const x = x + a; return x; }",
        ),
        (
            "function f(a) { for (var i = 0; i < a.length; i++) a[i] = () => i; }",
            "function f(a) { for (let i = 0; i < a.length; i++) a[i] = () => i; }",
        ),
    ];
    for (left, right) in pairs {
        assert!(!same_shape(left, right), "{left} = {right}");
    }
}

#[test]
fn a_single_definition_before_every_use_is_one_shape_whatever_its_kind() {
    assert!(same_shape(
        "function f(a) { if (a) { var x; x = a.b; return x.c; } }",
        "function f(a) { if (a) { const x = a.b; return x.c; } }",
    ));
    assert!(same_shape(
        "function f(a) { let x; x = a.b; x = x.c; return x; }",
        "function f(a) { var x = a.b; x = x.c; return x; }",
    ));
}

#[test]
fn a_computed_or_shorthand_proto_key_is_not_the_prototype_setter() {
    assert!(!same_shape(
        "function f() { return { [\"__proto__\"]: [] }; }",
        "function f() { return { __proto__: [] }; }",
    ));
    assert!(!same_shape(
        "function f(__proto__) { return { __proto__ }; }",
        "function f(__proto__) { return { __proto__: __proto__ }; }",
    ));
    assert!(same_shape(
        "function f() { return { \"__proto__\": [] }; }",
        "function f() { return { __proto__: [] }; }",
    ));
    assert!(same_shape(
        "function f() { return { [\"a\"]: [] }; }",
        "function f() { return { a: [] }; }",
    ));
}

#[test]
fn typeof_equality_is_loosened_only_against_a_string_literal() {
    assert!(!same_shape(
        "function f(a, b) { return typeof a === b; }",
        "function f(a, b) { return typeof a == b; }",
    ));
    assert!(same_shape(
        "function f(a) { return typeof a === \"object\"; }",
        "function f(a) { return typeof a == \"object\"; }",
    ));
}

#[test]
fn the_whole_program_shape_tells_the_canonical_forms_apart_too() {
    assert_ne!(
        program_shape("for (; a; b) {}"),
        program_shape("for (a; b;) {}")
    );
}

const CALLER: &str = "function inherit(d, b) {
  record(d, b);
  function __() { this.constructor = d; }
  d.prototype = b === null ? Object.create(b) : ((__.prototype = b.prototype), new __());
}
var log = [];
function record(d, b) { log.push(typeof d + \"<\" + typeof b); }
";

fn caller_row(outer: &[&str]) -> RowData {
    let record = records(CALLER)
        .into_iter()
        .find(|record| record.binding.as_deref() == Some("inherit"))
        .expect("the caller is a unit");
    RowData {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label: "tslib".to_string(),
        versions: "2.0.0".to_string(),
        module: "tslib".to_string(),
        role: Role::Function,
        own: Some("__extends".to_string()),
        inner: Vec::new(),
        outer: outer.iter().map(|name| Some(name.to_string())).collect(),
        specific: record.specific.max(2),
    }
}

#[test]
fn an_outer_function_is_named_only_when_its_own_code_matched() {
    let table = Table::from_rows(&[caller_row(&["extendStatics"])]);
    let names = names_after(&table, CALLER);
    assert!(
        names.contains(&("inherit".to_string(), "__extends".to_string())),
        "{names:?}"
    );
    assert!(!names.iter().any(|(old, _)| old == "record"), "{names:?}");
}

#[test]
fn an_outer_function_whose_own_code_matched_takes_the_callers_name() {
    let callee = records(CALLER)
        .into_iter()
        .find(|record| record.binding.as_deref() == Some("record"))
        .expect("the callee is a unit");
    let callee_row = RowData {
        loose: loose_key(callee.loose),
        exact: exact_key(callee.exact),
        label: "tslib".to_string(),
        versions: "2.0.0".to_string(),
        module: "tslib".to_string(),
        role: Role::Function,
        own: Some("extendStatics".to_string()),
        inner: Vec::new(),
        outer: Vec::new(),
        specific: callee.specific,
    };
    let table = Table::from_rows(&[caller_row(&["extendStatics"]), callee_row]);
    let names = names_after(&table, CALLER);
    assert!(
        names.contains(&("record".to_string(), "extendStatics".to_string())),
        "{names:?}"
    );
}

#[test]
fn an_outer_plain_variable_keeps_its_library_name() {
    let source = "var store = {};
function wrapper(key) {
  if (typeof key != \"string\") { throw new TypeError(\"shared-key wants a string key, got \" + typeof key); }
  return store[key] || (store[key] = { id: key, kind: \"shared-key\", made: Date.now(), tag: Symbol(key) });
}";
    let record = records(source)
        .into_iter()
        .find(|record| record.binding.as_deref() == Some("wrapper"))
        .expect("unit");
    let row = RowData {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label: "core-js".to_string(),
        versions: "3.39.0".to_string(),
        module: "internals/shared-key".to_string(),
        role: Role::Function,
        own: Some("sharedKey".to_string()),
        inner: vec![Some("key".to_string())],
        outer: vec![Some("keys".to_string())],
        specific: record.specific.max(2),
    };
    let names = names_after(&Table::from_rows(&[row]), source);
    assert!(
        names.contains(&("store".to_string(), "keys".to_string())),
        "{names:?}"
    );
}

fn shared_rows(source: &str, labels: &[(&str, &str)]) -> Vec<RowData> {
    labels
        .iter()
        .map(|(label, module)| {
            let mut row = row_for(source, module, "requireGetBuiltIn", &INNER[..4], &OUTER);
            row.label = label.to_string();
            row
        })
        .collect()
}

#[test]
fn a_shared_shape_takes_the_label_the_rest_of_the_program_agrees_with() {
    let other = GET_BUILT_IN.replace("arguments.length < 2", "arguments.length < 5");
    let mut rows = shared_rows(
        GET_BUILT_IN,
        &[("aaa-helpers", "helpers/x"), ("core-js", "internals/get-built-in")],
    );
    rows.extend(shared_rows(&other, &[("core-js", "internals/other")]));
    let table = Table::from_rows(&rows);
    let source = format!("{GET_BUILT_IN}\n{}", other.replace("wrapper", "wrapper2"));
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let found = match_with(&table, &parsed.program, &sem.scoping);
    let shared = found
        .hits
        .iter()
        .find(|hit| !hit.also.is_empty())
        .expect("the shared shape is a hit");
    assert_eq!(shared.label, "core-js");
    assert!(!shared.shared);
    assert_eq!(shared.also[0].label, "aaa-helpers");
}

#[test]
fn a_shared_shape_without_context_names_both_and_counts_for_neither() {
    let rows = shared_rows(
        GET_BUILT_IN,
        &[("zzz-helpers", "helpers/x"), ("core-js", "internals/get-built-in")],
    );
    let table = Table::from_rows(&rows);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, GET_BUILT_IN);
    let sem = build_scoping(&parsed.program);
    let found = match_with(&table, &parsed.program, &sem.scoping);
    let hit = found.hits.iter().find(|hit| hit.outermost).expect("hit");
    assert!(hit.shared);
    let reported: Vec<&untwist_rename::library::Hit> = found.hits.iter().collect();
    assert!(untwist_rename::library::pass::summary_notes(&reported).is_empty());
    let text = untwist_rename::library::annotate::comment_text(hit);
    assert!(text.contains("core-js") && text.contains("zzz-helpers"), "{text}");
}

#[test]
fn a_unit_inside_a_hit_of_any_of_its_labels_gets_no_comment_of_its_own() {
    let inner_source = "function (n, m) {
    return arguments.length < 2 ? (c(g[n]) ? g[n] : undefined) : g[n] && g[n][m];
  }";
    let inner = records(GET_BUILT_IN)
        .into_iter()
        .find(|record| record.binding.as_deref() == Some("value"))
        .expect("the inner function is a unit");
    assert!(GET_BUILT_IN.contains(inner_source));
    let mut rows = shared_rows(GET_BUILT_IN, &[("core-js", "internals/get-built-in")]);
    for label in ["aaa-helpers", "core-js"] {
        rows.push(RowData {
            loose: loose_key(inner.loose),
            exact: exact_key(inner.exact),
            label: label.to_string(),
            versions: "3.39.0".to_string(),
            module: "helpers/get".to_string(),
            role: Role::Export,
            own: Some("getBuiltIn".to_string()),
            inner: Vec::new(),
            outer: Vec::new(),
            specific: inner.specific,
        });
    }
    let table = Table::from_rows(&rows);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, GET_BUILT_IN);
    let sem = build_scoping(&parsed.program);
    let found = match_with(&table, &parsed.program, &sem.scoping);
    let outermost: Vec<&str> = found
        .hits
        .iter()
        .filter(|hit| hit.outermost)
        .flat_map(|hit| hit.modules.iter().map(String::as_str))
        .collect();
    assert_eq!(outermost, ["internals/get-built-in"]);
}

mod summaries {
    use untwist_rename::library::pass::{summaries, summary_notes};
    use untwist_rename::library::shape::UnitKind;
    use untwist_rename::library::table::Role;
    use untwist_rename::library::{intersect_ranges, parse_ranges, ranges_text, Hit};

    fn hit(modules: &[&str], versions: &str, start: u32) -> Hit {
        let ranges = parse_ranges(versions);
        Hit {
            label: "tslib".to_string(),
            versions: (ranges[0].0.clone(), ranges[ranges.len() - 1].1.clone()),
            ranges,
            modules: modules.iter().map(|module| module.to_string()).collect(),
            role: Role::Function,
            span: oxc_span::Span::new(start, start + 10),
            kind: UnitKind::Function,
            outermost: true,
            reported: true,
            shared: false,
            also: Vec::new(),
        }
    }

    #[test]
    fn a_unit_found_in_several_modules_counts_once() {
        let one = hit(&["tslib", "tslib.es6"], "1.10.0~2.9.0", 0);
        let notes = summary_notes(&[&one]);
        assert_eq!(notes, ["library: tslib@1.10\u{2013}2.9, 1 module recognised"]);
        let twice = hit(&["tslib", "tslib.es6"], "1.10.0~2.9.0", 20);
        assert!(summary_notes(&[&one, &twice])[0].ends_with(", 1 module recognised"));
    }

    #[test]
    fn the_range_is_the_intersection_of_the_version_sets() {
        let gap = hit(&["a"], "3.0.0~3.5.0,3.30.0~3.39.0", 0);
        let middle = hit(&["b"], "3.10.0~3.20.0", 20);
        assert_eq!(
            summary_notes(&[&gap, &middle]),
            ["library: tslib@versions disagree, 2 modules recognised"]
        );
        let late = hit(&["c"], "3.35.0~3.50.0", 40);
        assert_eq!(
            summary_notes(&[&gap, &late]),
            ["library: tslib@3.35\u{2013}3.39, 2 modules recognised"]
        );
        assert_eq!(ranges_text(&parse_ranges("3.0.0~3.5.0,3.30.0~3.39.0")), "3.0\u{2013}3.5, 3.30\u{2013}3.39");
        assert!(intersect_ranges(&parse_ranges("1.0.0~1.2.0"), &parse_ranges("1.3.0~1.4.0")).is_empty());
    }

    #[test]
    fn programs_that_disagree_on_versions_say_so() {
        let merged = summaries([
            "library: tslib@1.10\u{2013}1.14, 2 modules recognised",
            "library: tslib@2.0\u{2013}2.9, 3 modules recognised",
        ]);
        assert_eq!(merged[0].range, "versions disagree");
        assert_eq!(merged[0].modules, 5);
        let gap = summaries(["library: tslib@1.0\u{2013}1.5, 1.8\u{2013}2.1, 1 module recognised"]);
        assert_eq!(gap[0].range, "1.0\u{2013}1.5, 1.8\u{2013}2.1");
    }
}

#[test]
fn a_label_with_a_line_terminator_stays_inside_its_comment() {
    let mut row = row_for(GET_BUILT_IN, "internals/a\u{2028}b\nc", "requireGetBuiltIn", &INNER[..4], &OUTER);
    row.label = "evil\u{2029}lib\r".to_string();
    let table = Table::from_rows(&[row]);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, GET_BUILT_IN);
    let sem = build_scoping(&parsed.program);
    let found = match_with(&table, &parsed.program, &sem.scoping);
    let hit = found.hits.iter().find(|hit| hit.outermost).expect("hit");
    let text = untwist_rename::library::annotate::comment_text(hit);
    assert!(
        !text.contains(['\n', '\r', '\u{2028}', '\u{2029}']),
        "{text:?}"
    );
    assert!(text.contains("evil\\u2029lib\\r") && text.contains("a\\u2028b\\nc"), "{text}");
}

#[test]
fn a_table_that_claims_too_many_rows_is_refused_without_allocating_them() {
    let text = "#untwist-library-table\tv2\tx\n#strings\t1\n-\n#lists\t0\n#rows\t18446744073709551615\n";
    assert!(Table::parse(text).is_some_and(|table| table.rows.is_empty()));
}

#[test]
fn an_inflated_table_over_the_limit_is_refused() {
    let big = "a".repeat(4096);
    let data = gzip(&big);
    assert!(untwist_rename::library::table::inflate_with_limit(&data, 1024).is_none());
    assert_eq!(
        untwist_rename::library::table::inflate_with_limit(&data, 8192).as_deref(),
        Some(big.as_str())
    );
}

#[test]
fn matching_grows_linearly_with_the_number_of_matched_copies() {
    let row = row_for(GET_BUILT_IN, "internals/get-built-in", "requireGetBuiltIn", &INNER[..4], &OUTER);
    let table = Table::from_rows(&[row]);
    let copies = |count: usize| {
        (0..count)
            .map(|index| GET_BUILT_IN.replace("wrapper", &format!("wrapper{index}")))
            .collect::<String>()
    };
    let time = |source: &str| {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        let sem = build_scoping(&parsed.program);
        (0..3)
            .map(|_| {
                let started = std::time::Instant::now();
                let found = match_with(&table, &parsed.program, &sem.scoping);
                assert!(!found.hits.is_empty());
                started.elapsed()
            })
            .min()
            .expect("three runs")
    };
    let (small, large) = (copies(500), copies(4000));
    let ratio = time(&large).as_secs_f64() / time(&small).as_secs_f64().max(1e-6);
    assert!(ratio < 24.0, "8x the copies took {ratio:.1}x the time");
}

#[test]
fn a_program_is_matched_once_while_only_names_change() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function __rest(s, e) { var t = {}; for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p) && e.indexOf(p) < 0) t[p] = s[p]; return t; }");
    let sem = build_scoping(&parsed.program);
    let row = row_for(GET_BUILT_IN, "internals/get-built-in", "requireGetBuiltIn", &INNER[..4], &OUTER);
    let table = std::sync::Arc::new(Table::from_rows(&[row]));
    let first = untwist_rename::library::match_cached(&table, &parsed.program, &sem.scoping);
    let again = untwist_rename::library::match_cached(&table, &parsed.program, &sem.scoping);
    assert!(std::sync::Arc::ptr_eq(&first, &again));
    let other = parse_program(&alloc, "function __rest(s, e) { return s; }");
    let other_sem = build_scoping(&other.program);
    let different = untwist_rename::library::match_cached(&table, &other.program, &other_sem.scoping);
    assert!(!std::sync::Arc::ptr_eq(&first, &different));
}
