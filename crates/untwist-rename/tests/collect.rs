use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::collect::{collect, BindingRole, InitKind};
use untwist_rename::oracle::{NameOracle, OuterRule};

fn roles(source: &str) -> Vec<(String, BindingRole)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide)
        .bindings
        .into_iter()
        .map(|info| (info.name, info.role))
        .collect()
}

#[test]
fn bindings_arrive_in_source_order_with_their_syntactic_role() {
    let found = roles("function outer(p) { var v = 1; try {} catch (c) {} } class K {}");
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["outer", "p", "v", "c", "K"]);
    assert_eq!(found[0].1, BindingRole::Function);
    assert_eq!(found[1].1, BindingRole::Param);
    assert!(matches!(found[2].1, BindingRole::Variable(_)));
    assert_eq!(found[3].1, BindingRole::CatchParam);
    assert_eq!(found[4].1, BindingRole::Class);
}

#[test]
fn every_declaration_site_is_recorded_while_bindings_stay_one_per_symbol() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f(a) { var a = 1; var b = 2; }");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let bindings: Vec<&str> = collected
        .bindings
        .iter()
        .map(|info| info.name.as_str())
        .collect();
    assert_eq!(bindings, ["f", "a", "b"]);
    assert!(matches!(
        collected.bindings[1].role,
        BindingRole::Variable(_)
    ));
    let sites: Vec<&str> = collected
        .sites
        .iter()
        .map(|site| sem.scoping.symbol_name(site.symbol))
        .collect();
    assert_eq!(sites, ["f", "a", "a", "b"]);
    assert_eq!(collected.sites[1].role, BindingRole::Param);
    assert!(matches!(collected.sites[2].role, BindingRole::Variable(_)));
}

#[test]
fn initialiser_shapes_are_classified() {
    let found = roles(
        r#"var a = other; var b = () => {}; var c = class {}; var d = Array(1);
           var e = require("node:fs"); var f = obj.prop; var g = 12; var h = "hi";
           var i = {}; var j = []; var k = /x/; var l = true; var m = new Thing(); var n;"#,
    );
    let kind = |index: usize| match &found[index].1 {
        BindingRole::Variable(shape) => shape.kind,
        other => panic!("expected a variable role, found {other:?}"),
    };
    assert_eq!(kind(0), InitKind::Identifier);
    assert_eq!(kind(1), InitKind::Function);
    assert_eq!(kind(2), InitKind::Class);
    assert_eq!(kind(3), InitKind::Call);
    assert_eq!(kind(4), InitKind::Require);
    assert_eq!(kind(5), InitKind::Member);
    assert_eq!(kind(6), InitKind::Number);
    assert_eq!(kind(7), InitKind::String);
    assert_eq!(kind(8), InitKind::Object);
    assert_eq!(kind(9), InitKind::Array);
    assert_eq!(kind(10), InitKind::Regex);
    assert_eq!(kind(11), InitKind::Boolean);
    assert_eq!(kind(12), InitKind::New);
    assert_eq!(kind(13), InitKind::Absent);

    let BindingRole::Variable(require_shape) = &found[4].1 else {
        panic!("require");
    };
    assert_eq!(require_shape.name.as_deref(), Some("node:fs"));
    let BindingRole::Variable(member_shape) = &found[5].1 else {
        panic!("member");
    };
    assert_eq!(member_shape.name.as_deref(), Some("prop"));
    assert_eq!(member_shape.root.as_deref(), Some("obj"));
}

#[test]
fn a_call_records_the_shape_of_its_callee() {
    let found = roles(r#"var a = f()(); var b = "'".concat(x).concat(y); var c = store.load();"#);
    let shape = |index: usize| match &found[index].1 {
        BindingRole::Variable(shape) => shape.clone(),
        other => panic!("expected a variable role, found {other:?}"),
    };

    let nested = shape(0);
    let inner = nested.callee.expect("f()() records its callee");
    assert_eq!(inner.kind, InitKind::Call);
    let innermost = inner.callee.expect("f() records its callee");
    assert_eq!(innermost.kind, InitKind::Identifier);
    assert_eq!(innermost.name.as_deref(), Some("f"));

    let member_callee = shape(1).callee.expect("a member callee is recorded");
    assert_eq!(member_callee.kind, InitKind::Member);

    let named = shape(2);
    assert_eq!(named.name.as_deref(), Some("load"));
    assert_eq!(named.root.as_deref(), Some("store"));
}

#[test]
fn destructuring_and_import_roles_carry_their_key() {
    let found = roles(
        r#"import def from "./m"; import * as ns from "./m"; import { sin as s } from "./m";
           const { cos: c } = Math; const [first] = list;"#,
    );
    assert_eq!(found[0].1, BindingRole::ImportDefault("./m".to_string()));
    assert_eq!(found[1].1, BindingRole::ImportNamespace("./m".to_string()));
    assert_eq!(
        found[2].1,
        BindingRole::ImportNamed {
            source: "./m".to_string(),
            imported: "sin".to_string()
        }
    );
    assert_eq!(found[3].1, BindingRole::PatternProperty("cos".to_string()));
    assert_eq!(found[4].1, BindingRole::PatternElement);
}

#[test]
fn array_pattern_elements_are_distinguished_from_nested_patterns() {
    let found = roles(
        "var [direct] = list; var [defaulted = 1] = list; var [...rest] = list; \
         var [[nested]] = list;",
    );
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["direct", "defaulted", "rest", "nested"]);
    assert_eq!(found[0].1, BindingRole::PatternElement);
    assert_eq!(found[1].1, BindingRole::Unknown);
    assert_eq!(found[2].1, BindingRole::Unknown);
    assert_eq!(found[3].1, BindingRole::PatternElement);
}

#[test]
fn free_sets_hold_outer_symbols_used_inside_a_scope() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "var outer = 1; var unused = 2; function f() { return outer; }",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);

    let symbol = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|info| info.name == name)
            .map(|info| info.symbol)
            .expect("binding")
    };

    let root = sem.scoping.root_scope_id();
    let function_scope = sem
        .scoping
        .scope_descendants_from_root()
        .find(|&scope| sem.scoping.scope_parent_id(scope) == Some(root))
        .expect("the function owns a scope below the root");
    let free = collected
        .free
        .get(&function_scope)
        .cloned()
        .unwrap_or_default();

    assert!(
        free.contains(&symbol("outer")),
        "outer is referenced inside f and declared outside"
    );
    assert!(
        !free.contains(&symbol("unused")),
        "unused is never referenced inside f"
    );
    assert!(
        !free.contains(&symbol("f")),
        "f is not referenced inside its own body"
    );
    assert!(collected.free.get(&root).is_none_or(|free| free.is_empty()));
}

#[test]
fn exported_bindings_and_loop_counters_and_hazards_are_flagged() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "export const pinned = 1; const local = 2; export { local as renamed }; \
         for (var i = 0; i < 3; i++) { for (var j = 0; j < 3; j++) {} } eval('x');",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let symbol = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.symbol)
            .expect("binding")
    };

    assert!(collected.pinned.contains(&symbol("pinned")));
    assert!(!collected.pinned.contains(&symbol("local")));
    assert_eq!(collected.loop_counters.get(&symbol("i")).copied(), Some(0));
    assert_eq!(collected.loop_counters.get(&symbol("j")).copied(), Some(1));
    assert_eq!(collected.hazard, Some("direct eval"));
}

#[test]
fn only_a_numeric_or_absent_initialiser_counts_as_a_loop_counter() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "for (var i = 0; i < 3; i++) {} for (var j; j < 3; j++) {} \
         for (var k = start(); k < 3; k++) {} for (var m = limit; m < 3; m++) {}",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let counts = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|b| b.name == name)
            .map(|b| collected.loop_counters.contains_key(&b.symbol))
            .expect("binding")
    };
    assert!(counts("i"), "a numeric initialiser counts upward");
    assert!(counts("j"), "an absent initialiser counts upward");
    assert!(!counts("k"), "a call initialiser is not a counter");
    assert!(!counts("m"), "an identifier initialiser is not a counter");
}

#[test]
fn proto_shorthand_values_and_sloppy_block_functions_are_pinned() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "let __proto__ = 1; use({ __proto__ }); let q = 2; use({ q }); \
         function outer() { { function g() { return 7; } } return g(); }",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let symbol = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.symbol)
            .expect("binding")
    };

    assert!(collected.pinned.contains(&symbol("__proto__")));
    assert!(!collected.pinned.contains(&symbol("q")));
    assert!(collected.pinned.contains(&symbol("g")));
    assert!(!collected.pinned.contains(&symbol("outer")));
    assert!(collected.block_function_names.contains("g"));
}

#[test]
fn an_exported_declaration_pins_only_what_it_declares() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "export function f(p) { var v = 1; return v; } export const { a, b } = o;",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let symbol = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.symbol)
            .expect("binding")
    };

    assert!(collected.pinned.contains(&symbol("f")));
    assert!(
        !collected.pinned.contains(&symbol("p")),
        "a parameter is not part of the API"
    );
    assert!(
        !collected.pinned.contains(&symbol("v")),
        "a local is not part of the API"
    );
    assert!(collected.pinned.contains(&symbol("a")));
    assert!(collected.pinned.contains(&symbol("b")));
}

#[test]
fn labels_and_bound_names_are_collected() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "outer: for (;;) { break outer; } function f(p) { return p; }",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);

    assert!(collected.labels.contains("outer"));
    assert!(!collected.labels.contains("f"));
    assert!(collected.program_references.contains("f"));
    assert!(collected.program_references.contains("p"));
    assert!(!collected.program_references.contains("outer"));
}

#[test]
fn a_class_decorator_is_evaluated_in_the_enclosing_scope() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "function dec(x) { return x; } function f() { @dec class K {} }",
    );
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let binding = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|info| info.name == name)
            .expect("binding")
    };

    let decorator = binding("dec").symbol;
    let enclosing = binding("K").scope;
    let free = collected.free.get(&enclosing).cloned().unwrap_or_default();

    assert!(
        free.contains(&decorator),
        "a class-level decorator reference belongs to the scope that encloses the class"
    );
}

#[test]
fn by_symbol_indexes_every_binding() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var a = 1; function f(p) { let q = p; return q; }");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    assert_eq!(collected.by_symbol.len(), collected.bindings.len());
    for (index, info) in collected.bindings.iter().enumerate() {
        assert_eq!(
            collected.by_symbol.get(&info.symbol),
            Some(&index),
            "{}",
            info.name
        );
    }
}

#[test]
fn program_references_are_built_only_for_the_rule_that_reads_them() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f(p) { return p; }");
    let sem = build_scoping(&parsed.program);
    assert!(OuterRule::ProgramWide.needs_program_references());
    assert!(!OuterRule::UsedEnclosing.needs_program_references());
    assert!(
        collect(&parsed.program, &sem.scoping, OuterRule::UsedEnclosing)
            .program_references
            .is_empty()
    );
    let program_wide = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    assert!(program_wide.program_references.contains("f"));
    assert!(program_wide.program_references.contains("p"));
}

#[test]
fn a_program_wide_oracle_refuses_a_name_bound_elsewhere_in_the_program() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f(p) { return p; } function g(taken) {}");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let p = collected
        .bindings
        .iter()
        .find(|info| info.name == "p")
        .map(|info| info.symbol)
        .expect("p");
    let oracle = NameOracle::new(&sem.scoping, &collected);
    assert!(!oracle.is_available("taken", p));
    assert!(oracle.is_available("fresh", p));
}
