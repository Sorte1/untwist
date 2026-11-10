use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_str::Ident;
use rustc_hash::FxHashSet;
use untwist_core::parse::parse_program;
use untwist_core::semantic::{
    bindings_possibly_used_before, build_scoping, effectively_constant_bindings, has_module_syntax,
    is_binding_possibly_used_before, is_constant_binding, parameter_symbols, read_reference_count,
    rebuild_scoping, single_write_span, write_reference_count,
};

#[test]
fn counts_reads_and_writes_per_symbol() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "var a = 1; a; a; a = 2; var c = 3; c;");
    let sem = build_scoping(&p.program);
    let root = sem.scoping.root_scope_id();
    let a = sem.scoping.get_binding(root, Ident::from("a")).unwrap();
    let c = sem.scoping.get_binding(root, Ident::from("c")).unwrap();
    assert_eq!(read_reference_count(&sem.scoping, a), 2);
    assert_eq!(write_reference_count(&sem.scoping, a), 1);
    assert!(!is_constant_binding(&sem.scoping, a));
    assert_eq!(read_reference_count(&sem.scoping, c), 1);
    assert!(is_constant_binding(&sem.scoping, c));
}

#[test]
fn rebuild_yields_same_symbol_count() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "function f(x) { let y = x; return y; }");
    let first = build_scoping(&p.program);
    let again = rebuild_scoping(&p.program, first.stats);
    assert_eq!(first.scoping.symbols_len(), again.scoping.symbols_len());
}

fn find_symbol(
    sem: &untwist_core::semantic::Semantics,
    name: &str,
) -> oxc_syntax::symbol::SymbolId {
    sem.scoping
        .symbol_ids()
        .find(|symbol| sem.scoping.symbol_name(*symbol) == name)
        .expect("bound")
}

fn constant_binding(source: &str, name: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let symbol = find_symbol(&sem, name);
    is_constant_binding(&sem.scoping, symbol)
}

fn possibly_used_before(source: &str, name: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let symbol = find_symbol(&sem, name);
    let boundary = single_write_span(&parsed.program, &sem.scoping, symbol).expect("single write");
    is_binding_possibly_used_before(&parsed.program, &sem.scoping, symbol, boundary)
}

#[test]
fn parameter_symbols_and_single_write_span_agree() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f(c) { c = 2; }");
    let sem = build_scoping(&parsed.program);
    let params = parameter_symbols(&parsed.program);
    assert_eq!(params.len(), 1);
    let symbol = *params.iter().next().expect("one parameter");
    assert!(single_write_span(&parsed.program, &sem.scoping, symbol).is_some());
}

#[test]
fn finding_1_a_nested_write_is_possibly_used_before_through_the_public_predicate() {
    assert!(possibly_used_before(
        "function f(c) { var g = function () { c = 2; }; return c; }",
        "c"
    ));
}

#[test]
fn finding_3_single_write_span_refuses_a_second_write() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f(c) { c = 1; c = 2; }");
    let sem = build_scoping(&parsed.program);
    let symbol = find_symbol(&sem, "c");
    assert!(single_write_span(&parsed.program, &sem.scoping, symbol).is_none());
}

#[test]
fn finding_4_ownership_is_live_for_a_root_var_and_a_block_scoped_let() {
    assert!(possibly_used_before(
        "var c; c = 2; function g() { return c; }",
        "c"
    ));
    assert!(possibly_used_before(
        "{ let c; c = 2; var g = function () { return c; }; }",
        "c"
    ));
}

#[test]
fn finding_4_class_field_initialisers_are_deferred_contexts() {
    assert!(possibly_used_before(
        "function f(c) { c = 2; class K { p = c; } }",
        "c"
    ));
}

#[test]
fn finding_4_class_static_blocks_are_deferred_contexts() {
    assert!(possibly_used_before(
        "function f(c) { class K { static { c = 2; } } return c; }",
        "c"
    ));
}

#[test]
fn finding_4_accessor_property_values_are_deferred_contexts() {
    assert!(possibly_used_before(
        "function f(c) { c = 2; class K { accessor p = c; } }",
        "c"
    ));
}

#[test]
fn finding_6_the_batched_form_matches_the_single_form_for_three_shapes() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "function f(a, b, c) { console.log(b); a = 1; var g = function () { return a; }; b = 2; c = 3; return c; }",
    );
    let sem = build_scoping(&parsed.program);
    let candidates: Vec<_> = ["a", "b", "c"]
        .into_iter()
        .map(|name| {
            let symbol = find_symbol(&sem, name);
            let boundary =
                single_write_span(&parsed.program, &sem.scoping, symbol).expect("single write");
            (symbol, boundary)
        })
        .collect();
    let batched = bindings_possibly_used_before(&parsed.program, &sem.scoping, &candidates);
    for &(symbol, boundary) in &candidates {
        let single =
            is_binding_possibly_used_before(&parsed.program, &sem.scoping, symbol, boundary);
        assert_eq!(batched.contains(&symbol), single);
    }
    assert!(batched.contains(&candidates[0].0));
    assert!(batched.contains(&candidates[1].0));
    assert!(!batched.contains(&candidates[2].0));
}

#[test]
fn finding_8_parameter_symbols_excludes_rest_and_destructured_parameters() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f(a, b = 1, {c}, ...r) {}");
    build_scoping(&parsed.program);
    let params = parameter_symbols(&parsed.program);
    assert_eq!(params.len(), 2);
}

#[test]
fn a_redeclared_binding_is_not_constant() {
    assert!(constant_binding("var o = 1;", "o"));
    assert!(!constant_binding("var o = 1;\nvar o = 2;", "o"));
}

#[test]
fn effectively_constant_bindings_keeps_the_constant_candidates() {
    const SOURCE: &str = "\
let never = 1;
let written = 2;
written = 3;
function f(p, q, r) {
  p = 1;
  q;
  q = 2;
  return [p, q, r, never, written];
}
";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, SOURCE);
    let sem = build_scoping(&parsed.program);
    let all: Vec<_> = sem.scoping.symbol_ids().collect();
    let batched = effectively_constant_bindings(&sem.scoping, &all);
    for symbol in all {
        assert_eq!(
            batched.contains(&symbol),
            is_constant_binding(&sem.scoping, symbol),
            "{} disagrees",
            sem.scoping.symbol_name(symbol)
        );
    }
    let name_of = |symbol: &oxc_syntax::symbol::SymbolId| sem.scoping.symbol_name(*symbol);
    let constant: FxHashSet<&str> = batched.iter().map(name_of).collect();
    assert!(constant.contains("never"));
    assert!(constant.contains("r"));
    assert!(!constant.contains("p"));
    assert!(!constant.contains("written"));
    assert!(!constant.contains("q"));
}

#[test]
fn an_empty_candidate_list_walks_nothing_and_returns_nothing() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "let a = 1;");
    let sem = build_scoping(&parsed.program);
    assert!(effectively_constant_bindings(&sem.scoping, &[]).is_empty());
}

#[test]
fn module_syntax_is_an_import_or_an_export_and_nothing_else() {
    let is_module = |source: &str| {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        has_module_syntax(&parsed.program)
    };
    assert!(is_module("export {};"));
    assert!(is_module("import a from \"a\";"));
    assert!(is_module("let x = 1;\nexport default x;"));
    assert!(!is_module("let x = 1;\n(0, eval)(\"x = 2\");"));
    assert!(!is_module("\"use strict\";\nlet x = 1;"));
    assert!(!is_module("var m = import(\"m\");"));
}

fn possibly_used_before_once_the_first_two_statements_swap(source: &str, name: &str) -> bool {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let Some(Statement::FunctionDeclaration(function)) = parsed.program.body.first_mut() else {
        panic!("the source must start with a function declaration");
    };
    let body = function.body.as_mut().expect("a function body");
    body.statements.swap(0, 1);
    let sem = build_scoping(&parsed.program);
    let symbol = find_symbol(&sem, name);
    let boundary = single_write_span(&parsed.program, &sem.scoping, symbol).expect("single write");
    is_binding_possibly_used_before(&parsed.program, &sem.scoping, symbol, boundary)
}

#[test]
fn a_read_moved_ahead_of_the_write_counts_as_before_it_whatever_its_span() {
    assert!(possibly_used_before_once_the_first_two_statements_swap(
        "function f(g, a) { a = 5; g(a); return a; }",
        "a"
    ));
}

#[test]
fn a_read_moved_behind_the_write_does_not_count_as_before_it() {
    assert!(!possibly_used_before_once_the_first_two_statements_swap(
        "function f(g, a) { g(a); a = 5; return a; }",
        "a"
    ));
}

#[test]
fn a_read_in_the_right_hand_side_still_counts_once_statements_have_moved() {
    assert!(possibly_used_before_once_the_first_two_statements_swap(
        "function f(g, a) { g(); a = a + 1; return a; }",
        "a"
    ));
}
