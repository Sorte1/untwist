mod common;

use common::ZeroSpans;
use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, PassCtor, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

fn run_with_notes<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    common::run_ctors(
        alloc,
        source,
        &[untwist_deobfuscate::merge_object_assignments::boxed],
    )
}

fn run_superset<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    let (code, report) = untwist_testing::transform_report(
        alloc,
        source,
        &[untwist_deobfuscate::merge_object_assignments::boxed],
        PassOptions::default(),
    );
    assert!(
        report.converged(),
        "passes did not converge: {:?}",
        report.stop
    );
    (
        code,
        report.notes.into_iter().map(|note| note.message).collect(),
    )
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    print(&parsed.program, PrintStyle::Pretty)
}

#[test]
fn inline_properties_without_inlining_object() {
    assert_transform!(
        "function f() {\n  const obj = {};\n  obj.foo = foo;\n  obj.bar = 1;\n  foo++;\n  return obj;\n}",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @r#"
    function f() {
      const obj = {
        foo,
        bar: 1
      };
      foo++;
      return obj;
    }
    "#
    );
}

#[test]
fn inline_properties_and_object() {
    assert_transform!(
        "function f() {\n  const obj = {};\n  obj.foo = 'foo';\n  return obj;\n}",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @r#"
    function f() {
      return { foo: "foo" };
    }
    "#
    );
}

#[test]
fn computed_properties_are_normalised() {
    assert_transform!(
        "function f() {\n  const obj = {};\n  obj[\"a b c\"] = 1;\n  obj[1] = 2;\n  return obj;\n}",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @r#"
    function f() {
      return {
        "a b c": 1,
        1: 2
      };
    }
    "#
    );
}

#[test]
fn a_computed_key_that_is_not_a_literal_refuses_the_fold() {
    for (source, kept) in [
        (
            "function f() {\n  const obj = {};\n  obj[key] = 1;\n  return obj;\n}",
            "obj[key] = 1",
        ),
        (
            "function f() {\n  const obj = {};\n  obj.a = 1;\n  obj[key] = 2;\n  obj.b = 3;\n  return obj;\n}",
            "obj[key] = 2",
        ),
        (
            "function f() {\n  const obj = {};\n  obj[-1] = 2;\n  return obj;\n}",
            "obj[-1] = 2",
        ),
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert!(out.contains(kept), "{source} -> {out}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn a_proto_key_refuses_the_fold() {
    for source in [
        "const obj = {};\nobj.__proto__ = a;\nobj.__proto__ = b;",
        "const obj = {};\nobj[\"__proto__\"] = a;\nobj[\"__proto__\"] = b;",
        "const obj = {};\nobj.__proto__ = a;\nlog(obj);",
        "const obj = { __proto__: x };\nobj.k = 1;\nlog(obj);",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert_eq!(out, printed(source), "{source}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn the_fold_keeps_the_assignments_before_a_refused_key() {
    assert_transform!(
        "function f() {\n  const obj = {};\n  obj.a = 1;\n  obj.__proto__ = p;\n  return obj;\n}",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @r#"
    function f() {
      const obj = { a: 1 };
      obj.__proto__ = p;
      return obj;
    }
    "#
    );
}

#[test]
fn a_value_that_can_run_code_refuses_the_fold() {
    for source in [
        "const obj = {};\nobj.a = new X();\nlog(obj);",
        "const obj = {};\nobj.a = t`x`;\nlog(obj);",
        "const obj = {};\nobj.a = await p;\nlog(obj);",
        "const obj = {};\nobj.a = import(\"m\");\nlog(obj);",
        "function* g() {\n  const obj = {};\n  obj.a = yield 1;\n  log(obj);\n}",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert_eq!(out, printed(source), "{source}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn a_plain_member_read_still_folds() {
    assert_transform!(
        "function f() {\n  const obj = {};\n  obj.a = other.b;\n  return obj;\n}",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @r#"
    function f() {
      const obj = { a: other.b };
      return obj;
    }
    "#
    );
}

#[test]
fn a_read_that_may_run_before_the_declaration_refuses_the_inline() {
    for source in [
        "console.log(o);\nvar o = { a: 1 };",
        "log(o);\nconst o = { a: 1 };",
        "log(o);\nconst o = {};\no.a = 1;",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert!(out.contains("log(o)"), "{source} -> {out}");
        assert!(
            notes
                .iter()
                .any(|note| note.contains("merge_object_use_before_init")),
            "{source} -> {notes:?}"
        );
    }
}

#[test]
fn the_folded_literal_is_also_refused_before_the_declaration_under_conservative() {
    let source = "log(o);\nconst o = {};\no.a = 1;";
    let alloc = Allocator::default();
    let (out, notes) = run_with_notes(&alloc, source);
    assert_eq!(out.trim(), "log(o);\nconst o = { a: 1 };");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("merge_object_use_before_init")),
        "{notes:?}"
    );
}

#[test]
fn a_scope_hazard_refuses_the_inline() {
    for source in [
        "const o = { a: 1 };\neval(\"o\");\nuse(o);",
        "with (x) {\n  y;\n}\nconst o = { a: 1 };\nuse(o);",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert!(out.contains("const o = {"), "{source} -> {out}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn a_scope_hazard_refuses_the_fold_as_well() {
    for source in [
        "with (o) {\n  var obj = {};\n  obj.a = p;\n}",
        "eval(\"x\");\nvar obj = {};\nobj.a = p;",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_with_notes(&alloc, source);
        assert_eq!(out, printed(source), "conservative: {source}");
        assert!(notes.is_empty(), "conservative: {source} -> {notes:?}");
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert_eq!(out, printed(source), "superset: {source}");
        assert!(notes.is_empty(), "superset: {source} -> {notes:?}");
    }
}

#[test]
fn a_circular_or_call_valued_assignment_is_refused() {
    for source in [
        "const obj = {};\nobj.foo = obj;",
        "const obj = {};\nobj.foo = fn();",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_with_notes(&alloc, source);
        assert_eq!(out, printed(source), "{source}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn the_object_is_not_inlined_into_a_repeated_call_context() {
    for source in [
        "const obj = {};\nobj.foo = 1;\nfunction f() { return obj; }",
        "const obj = {};\nobj.foo = 1;\nconst f = () => obj;",
        "const obj = {};\nobj.foo = 1;\nconst obj2 = { f() { return obj; } };",
        "const obj = {};\nobj.foo = 1;\nclass C { f = obj; }",
        "const obj = {};\nobj.foo = 1;\nwhile (i < 2) { arr.push(obj); }",
        "const obj = {};\nobj.foo = 1;\ndo { arr.push(obj); } while (i < 2);",
        "const obj = {};\nobj.foo = 1;\nfor (let i = 0; i < 2; i++) { arr.push(obj); }",
        "const obj = {};\nobj.foo = 1;\nfor (const item of items) { arr.push(obj); }",
        "const obj = {};\nobj.foo = 1;\nfor (const key in [1, 2]) { arr.push(obj); }",
    ] {
        let alloc = Allocator::default();
        let out = run_with_notes(&alloc, source).0;
        assert!(
            out.contains("const obj = {"),
            "conservative: {source} -> {out}"
        );
        assert!(out.contains("foo: 1"), "conservative: {source} -> {out}");
        let alloc = Allocator::default();
        let out = run_superset(&alloc, source).0;
        assert!(out.contains("const obj = {"), "superset: {source} -> {out}");
        assert!(out.contains("foo: 1"), "superset: {source} -> {out}");
    }
}

#[test]
fn a_shape_the_fold_refuses_leaves_the_statement_list_byte_identical_and_silent() {
    for source in [
        "const obj = {};\nlog(obj);\nobj.foo = 1;",
        "const obj = { a: 1 };\nobj.foo = 2;",
        "let obj = {};\nobj = other;\nobj.foo = 1;",
        "const obj = {};\nother.foo = 1;\nlog(obj);",
        "const obj = {};\nobj.foo ||= 1;",
        "while (x) var obj = { a: 1 };\nlog(obj);",
        "const obj = { a: 1 };\nexport { obj };",
        "const obj = { a: fn() };\nlog(obj);",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_with_notes(&alloc, source);
        assert_eq!(out, printed(source), "conservative: {source}");
        assert!(notes.is_empty(), "conservative: {source} -> {notes:?}");
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert_eq!(out, printed(source), "superset: {source}");
        assert!(notes.is_empty(), "superset: {source} -> {notes:?}");
    }
}

#[test]
fn conservative_mode_inlines_only_a_literal_the_fold_produced() {
    let kept = "const o = { a: 1 };\nuse(o);";
    let alloc = Allocator::default();
    assert_eq!(run_with_notes(&alloc, kept).0, printed(kept));

    let merged = "const o = {};\no.a = 1;\nuse(o);";
    let alloc = Allocator::default();
    assert_eq!(run_with_notes(&alloc, merged).0.trim(), "use({ a: 1 });");
}

#[test]
fn the_superset_inlines_a_pre_populated_literal_as_well() {
    for source in [
        "const o = { a: 1 };\nuse(o);",
        "const o = {};\no.a = 1;\nuse(o);",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert_eq!(out.trim(), "use({ a: 1 });", "{source}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn the_fold_stops_at_the_first_statement_that_is_not_an_assignment() {
    assert_transform!(
        "function f() {\n  const obj = {};\n  obj.foo = 1;\n  log(obj);\n  obj.bar = 2;\n  return obj;\n}",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @r#"
    function f() {
      const obj = { foo: 1 };
      log(obj);
      obj.bar = 2;
      return obj;
    }
    "#
    );
}

#[test]
fn a_reference_in_the_declarations_own_statement_list_is_inlined() {
    assert_transform!(
        "const obj = {};\nobj.foo = 1;\nlog(obj);",
        [untwist_deobfuscate::merge_object_assignments::boxed],
        @"log({ foo: 1 });"
    );
}

#[test]
fn conservative_merges_any_computed_key_and_names_the_hazard_it_adopts() {
    for (source, merged) in [
        (
            "const k = \"a\";\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "[k]: 1",
        ),
        ("const obj = {};\nobj[fn()] = 1;\nlog(obj);", "[fn()]: 1"),
        ("const obj = {};\nobj[a.b] = 1;\nlog(obj);", "[a.b]: 1"),
        ("const obj = {};\nobj[-1] = 1;\nlog(obj);", "[-1]: 1"),
        ("const obj = {};\nobj[p] = 1;\nlog(obj);", "[p]: 1"),
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_with_notes(&alloc, source);
        assert!(out.contains(merged), "{source} -> {out}");
        assert!(
            notes
                .iter()
                .any(|note| note == untwist_deobfuscate::notes::MERGE_ADOPTED_DYNAMIC_KEY),
            "{source} -> {notes:?}"
        );
    }
}

#[test]
fn the_default_merges_only_a_computed_key_that_resolves_to_a_safe_literal() {
    for (source, merged) in [
        (
            "const k = \"a b\";\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "[k]: 1",
        ),
        (
            "const k = 7;\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "[k]: 1",
        ),
        (
            "const k = `a`;\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "[k]: 1",
        ),
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert!(out.contains(merged), "{source} -> {out}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

fn run_unsafe_lane<'a>(alloc: &'a Allocator, source: &'a str) -> String {
    let (code, report) = untwist_testing::transform_report(
        alloc,
        source,
        &[untwist_deobfuscate::merge_object_assignments::boxed],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    assert!(report.converged(), "{:?}", report.stop);
    code
}

const A_SYMBOL_KEYED_RUN: &str = "const tag = Symbol(\"tag\");\nconst other = Symbol.for(f());\nconst obj = {};\nobj[tag] = 1;\nobj[other] = 2;\nobj.b = 3;\nlog(obj);";

#[test]
fn the_unsafe_lane_merges_a_computed_key_bound_to_a_fresh_symbol() {
    let alloc = Allocator::default();
    let out = run_unsafe_lane(&alloc, A_SYMBOL_KEYED_RUN);
    assert!(out.contains("[tag]: 1"), "{out}");
    assert!(out.contains("[other]: 2"), "{out}");
    assert!(out.contains("b: 3"), "{out}");
    let (safe, _) = run_superset(&alloc, A_SYMBOL_KEYED_RUN);
    assert!(safe.contains("obj[tag] = 1"), "{safe}");
}

#[test]
fn the_unsafe_lane_refuses_a_symbol_key_it_cannot_prove_fresh() {
    for source in [
        "function Symbol(x) {\n  return x;\n}\nconst tag = Symbol(\"__proto__\");\nconst obj = {};\nobj[tag] = 1;\nlog(obj);",
        "let tag = Symbol(\"t\");\ntag = g;\nconst obj = {};\nobj[tag] = 1;\nlog(obj);",
        "const tag = Symbol.keyFor(s);\nconst obj = {};\nobj[tag] = 1;\nlog(obj);",
        "const tag = String(\"__proto__\");\nconst obj = {};\nobj[tag] = 1;\nlog(obj);",
    ] {
        let alloc = Allocator::default();
        let out = run_unsafe_lane(&alloc, source);
        assert!(out.contains("obj[tag] = 1"), "{source} -> {out}");
    }
}

#[test]
fn the_default_refuses_a_computed_key_it_cannot_resolve_to_a_safe_literal() {
    for (source, kept) in [
        (
            "const obj = {};\nobj[fn()] = 1;\nlog(obj);",
            "obj[fn()] = 1",
        ),
        (
            "const k = fn();\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "obj[k] = 1",
        ),
        (
            "const k = a.b;\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "obj[k] = 1",
        ),
        (
            "function f(k) {\n  const obj = {};\n  obj[k] = 1;\n  log(obj);\n}",
            "obj[k] = 1",
        ),
        (
            "let k = \"a\";\nk = \"b\";\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "obj[k] = 1",
        ),
        (
            "const k = \"__proto__\";\nconst obj = {};\nobj[k] = 1;\nlog(obj);",
            "obj[k] = 1",
        ),
        ("const obj = {};\nobj[p] = 1;\nlog(obj);", "obj[p] = 1"),
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert!(out.contains(kept), "{source} -> {out}");
        assert!(notes.is_empty(), "{source} -> {notes:?}");
    }
}

#[test]
fn a_rebound_object_merges_when_every_write_follows_an_inert_run() {
    for source in [
        "let obj = {};\nobj.a = 1;\nobj.b = v;\nobj = other;\nlog(obj);",
        "let obj = {};\nobj.a = null;\nlog(obj);\nobj = other;",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_with_notes(&alloc, source);
        assert!(out.contains("obj = {"), "conservative: {source} -> {out}");
        assert!(!out.contains("obj.a ="), "conservative: {source} -> {out}");
        assert!(notes.is_empty(), "conservative: {source} -> {notes:?}");
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert!(out.contains("obj = {"), "superset: {source} -> {out}");
        assert!(!out.contains("obj.a ="), "superset: {source} -> {out}");
        assert!(notes.is_empty(), "superset: {source} -> {notes:?}");
    }
}

#[test]
fn a_rebound_object_is_refused_when_the_write_is_anywhere_else() {
    for source in [
        "let obj = {};\nobj = other;\nobj.a = 1;\nlog(obj);",
        "obj = other;\nvar obj = {};\nobj.a = 1;\nlog(obj);",
        "let obj = {};\nobj.a = 1;\nfunction later() { obj = other; }\nlog(obj);",
        "let obj = {};\nobj.a = other.b;\nobj = other;\nlog(obj);",
        "let obj = {};\nobj.a = fn();\nobj = other;\nlog(obj);",
    ] {
        let alloc = Allocator::default();
        let (out, notes) = run_with_notes(&alloc, source);
        assert_eq!(out, printed(source), "conservative: {source}");
        assert!(notes.is_empty(), "conservative: {source} -> {notes:?}");
        let alloc = Allocator::default();
        let (out, notes) = run_superset(&alloc, source);
        assert_eq!(out, printed(source), "superset: {source}");
        assert!(notes.is_empty(), "superset: {source} -> {notes:?}");
    }
}

const C_DECLARED_LAST: &str = "var a = {};\na.k1 = 1;\nvar b = {};\nb.k2 = 2;\nvar c = { k3: 3 };\nconsole.log(a.k1, b.k2, c.k3);";
const C_DECLARED_FIRST: &str = "var c = { k3: 3 };\nvar a = {};\na.k1 = 1;\nvar b = {};\nb.k2 = 2;\nconsole.log(a.k1, b.k2, c.k3);";
const ONE_FOLD_BEFORE_C: &str =
    "var a = {};\na.k1 = 1;\nvar c = { k3: 3 };\nconsole.log(a.k1, c.k3);";
const C_READ_TWICE: &str =
    "var a = {};\na.k1 = 1;\nvar c = { k3: 3 };\nconsole.log(a.k1, c.k3, c.k3);";

#[test]
fn conservative_keeps_a_literal_the_fold_never_merged() {
    for (source, expected) in [
        (
            ONE_FOLD_BEFORE_C,
            "var c = { k3: 3 };\nconsole.log({ k1: 1 }.k1, c.k3);",
        ),
        (
            C_DECLARED_LAST,
            "var c = { k3: 3 };\nconsole.log({ k1: 1 }.k1, { k2: 2 }.k2, c.k3);",
        ),
        (
            C_DECLARED_FIRST,
            "var c = { k3: 3 };\nconsole.log({ k1: 1 }.k1, { k2: 2 }.k2, c.k3);",
        ),
        (
            C_READ_TWICE,
            "var c = { k3: 3 };\nconsole.log({ k1: 1 }.k1, c.k3, c.k3);",
        ),
    ] {
        let alloc = Allocator::default();
        assert_eq!(
            run_with_notes(&alloc, source).0.trim(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn the_default_candidate_set_is_unchanged_by_the_span_key() {
    for (source, expected) in [
        (
            ONE_FOLD_BEFORE_C,
            "console.log({ k1: 1 }.k1, { k3: 3 }.k3);",
        ),
        (
            C_DECLARED_LAST,
            "console.log({ k1: 1 }.k1, { k2: 2 }.k2, { k3: 3 }.k3);",
        ),
        (
            C_DECLARED_FIRST,
            "console.log({ k1: 1 }.k1, { k2: 2 }.k2, { k3: 3 }.k3);",
        ),
        (
            C_READ_TWICE,
            "var c = { k3: 3 };\nconsole.log({ k1: 1 }.k1, c.k3, c.k3);",
        ),
    ] {
        let alloc = Allocator::default();
        assert_eq!(run_superset(&alloc, source).0.trim(), expected, "{source}");
    }
}

#[test]
fn a_merged_property_carries_a_real_synthetic_span() {
    let alloc = Allocator::default();
    let source = "function hi() {\n  var o = {};\n  o.a = 1;\n  o.b = 2;\n  use(o);\n}";
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let ctors: Vec<PassCtor<'_>> = vec![untwist_deobfuscate::merge_object_assignments::boxed];
    let mut group = PassGroup::from_ctors(&ctors, PassOptions::default());
    let (_sem, _report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    let mut scan = ZeroSpans::default();
    scan.visit_program(&parsed.program);
    assert!(
        scan.0.is_empty(),
        "nodes the fold synthesised still carry Span::new(0, 0): {:?}",
        scan.0
    );
}

#[test]
fn a_fold_freezes_one_declaration_not_one_assignment() {
    let (code, _notes) = common::run_all_stages(
        "function hi() {\n  var o = {};\n  o.a = 1;\n  o.b = 2;\n  o.c = 3;\n  use(o);\n}",
    );
    assert_eq!(
        code.trim(),
        "function hi() {\n  use({\n    a: 1,\n    b: 2,\n    c: 3\n  });\n}"
    );
    let (again, _notes) = common::run_all_stages(&code);
    assert_eq!(again.trim(), code.trim(), "the fold is not idempotent");
}

const PROTO_SHORTHAND: &str = "function g() {\n  var __proto__ = { y: 1 };\n  var o = { __proto__ };\n  return Object.keys(o).join() + o.y;\n}\nconsole.log(g());\n";

#[test]
fn a_proto_shorthand_is_not_inlined_in_either_mode() {
    for extended in [true, false] {
        let alloc = Allocator::default();
        let (code, _) = untwist_testing::transform_report(
            &alloc,
            PROTO_SHORTHAND,
            &[untwist_deobfuscate::merge_object_assignments::boxed],
            PassOptions {
                extended,
                ..PassOptions::default()
            },
        );
        assert!(code.contains("__proto__ = { y: 1 }"), "{code}");
        untwist_testing::node::assert_node_agrees(PROTO_SHORTHAND, &code);
    }
}
