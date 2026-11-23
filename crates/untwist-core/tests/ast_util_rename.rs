use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_str::Ident;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::rename::{
    is_name_taken, unique_name, unique_name_avoiding, Renames, UNIQUE_NAME_SUFFIX_CAP,
};
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;

#[test]
fn renaming_a_binding_rewrites_the_declaration_and_every_reference() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "function a() { return 1; }\na();\nvar b = a;\n");
    let sem = build_scoping(&parsed.program);
    let Statement::FunctionDeclaration(function) = &parsed.program.body[0] else {
        panic!("function declaration");
    };
    let symbol = function
        .id
        .as_ref()
        .and_then(|id| id.symbol_id.get())
        .expect("bound");

    let mut renames = Renames::new();
    renames.insert(symbol, Ident::from("__STRING_ARRAY__"));
    let count = renames.apply(&mut parsed.program, &sem.scoping, &&alloc);

    assert_eq!(count, 3);
    assert_eq!(
        print(&parsed.program, PrintStyle::Compact),
        "function __STRING_ARRAY__(){return 1}__STRING_ARRAY__();var b=__STRING_ARRAY__;"
    );
}

#[test]
fn renaming_only_the_binding_leaves_every_reference_reading_the_name() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "function a() { return 1; }\na();\nvar b = a;\n");
    let _sem = build_scoping(&parsed.program);
    let Statement::FunctionDeclaration(function) = &parsed.program.body[0] else {
        panic!("function declaration");
    };
    let symbol = function
        .id
        .as_ref()
        .and_then(|id| id.symbol_id.get())
        .expect("bound");

    let mut renames = Renames::new();
    renames.insert(symbol, Ident::from("__STRING_ARRAY__"));
    let count = renames.apply_bindings(&mut parsed.program);

    assert_eq!(count, 1);
    assert_eq!(
        print(&parsed.program, PrintStyle::Compact),
        "function __STRING_ARRAY__(){return 1}a();var b=a;"
    );
}

#[test]
fn an_unresolved_reference_takes_the_name_too() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "c2 = 1;\nfunction f(c) {\n  return c;\n}\n");
    let sem = build_scoping(&parsed.program);
    assert!(is_name_taken(&sem.scoping, "c2"));
    let name = unique_name("c2", &sem.scoping, &&alloc).expect("a name");
    assert_eq!(name.as_str(), "c2_2__");
}

#[test]
fn a_taken_name_gets_a_numeric_suffix() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var __STRING_ARRAY__ = 1;");
    let sem = build_scoping(&parsed.program);
    assert!(is_name_taken(&sem.scoping, "__STRING_ARRAY__"));
    assert!(!is_name_taken(&sem.scoping, "__DECODE_0__"));
    let name = unique_name("__STRING_ARRAY__", &sem.scoping, &&alloc).expect("a name");
    assert_eq!(name.as_str(), "__STRING_ARRAY_2__");
}

#[test]
fn a_reserved_name_is_avoided_even_when_the_scope_is_free() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var q = 1;");
    let sem = build_scoping(&parsed.program);
    let mut reserved = FxHashSet::default();
    reserved.insert("Component".to_string());
    let name = unique_name_avoiding("Component", &sem.scoping, &reserved, &&alloc).expect("a name");
    assert_eq!(name.as_str(), "Component_2__");
}

fn scoped(source: &str) -> (Allocator, untwist_core::semantic::Semantics) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    (alloc, sem)
}

#[test]
fn a_free_base_is_returned_as_is() {
    let (alloc, sem) = scoped("var other = 1;");
    assert_eq!(
        unique_name("foo", &sem.scoping, &&alloc).map(|name| name.to_string()),
        Some("foo".to_string())
    );
}

#[test]
fn a_reserved_base_past_the_budget_returns_none() {
    let (alloc, sem) = scoped("var other = 1;");
    let mut reserved = FxHashSet::default();
    reserved.insert("foo".to_string());
    for suffix in 2..=UNIQUE_NAME_SUFFIX_CAP {
        reserved.insert(format!("foo_{suffix}__"));
    }
    assert_eq!(
        unique_name_avoiding("foo", &sem.scoping, &reserved, &&alloc),
        None
    );
}

#[test]
fn an_exhausted_suffix_budget_returns_none_rather_than_a_taken_name() {
    let mut source = String::from("var foo = 1;\n");
    for suffix in 2..=UNIQUE_NAME_SUFFIX_CAP {
        source.push_str(&format!("var foo_{suffix}__ = 1;\n"));
    }
    let (alloc, sem) = scoped(&source);
    assert_eq!(unique_name("foo", &sem.scoping, &&alloc), None);
}

fn crowded_lookup(count: usize) -> std::time::Duration {
    let names: Vec<String> = (2..count).map(|suffix| format!("foo_{suffix}__")).collect();
    let source = format!("var foo, {};", names.join(", "));
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    (0..3)
        .map(|_| {
            let started = std::time::Instant::now();
            for _ in 0..20 {
                let name = unique_name("foo", &sem.scoping, &&alloc).expect("a free suffix");
                assert_eq!(name.as_str(), format!("foo_{count}__"));
            }
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
fn a_crowded_base_name_scans_the_symbols_once_per_lookup() {
    let small = crowded_lookup(200).max(std::time::Duration::from_micros(200));
    let large = crowded_lookup(800);
    let ratio = large.as_secs_f64() / small.as_secs_f64();
    assert!(
        ratio < 8.0,
        "800 crowded names took {large:?}, 200 took {small:?}: x{ratio:.1}"
    );
}

fn renamed_text(source: &str, from: &str, to: &str) -> String {
    let alloc = Allocator::default();
    let source = alloc.alloc_str(source);
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let mut renames = Renames::new();
    for symbol in sem.scoping.symbol_ids() {
        if sem.scoping.symbol_name(symbol) == from {
            renames.insert(symbol, Ident::from_str_in(to, &&alloc));
        }
    }
    renames.apply(&mut parsed.program, &sem.scoping, &&alloc);
    print(&parsed.program, PrintStyle::Compact)
}

#[test]
fn a_renamed_shorthand_assignment_target_keeps_its_key() {
    assert_eq!(
        renamed_text("function f(o) { let x; ({x} = o); return x; }", "x", "y"),
        "function f(o){let y;({x:y}=o);return y}"
    );
}

#[test]
fn a_renamed_shorthand_assignment_target_keeps_its_key_and_default() {
    assert_eq!(
        renamed_text(
            "function f(o) { let x; ({x = 7} = o); return x; }",
            "x",
            "y"
        ),
        "function f(o){let y;({x:y=7}=o);return y}"
    );
}

#[test]
fn a_renamed_shorthand_target_of_a_for_of_head_keeps_its_key() {
    assert_eq!(
        renamed_text(
            "function f(a) { let x; for ({x} of a); return x; }",
            "x",
            "y"
        ),
        "function f(a){let y;for({x:y}of a);return y}"
    );
}

#[test]
fn a_renamed_shorthand_target_nested_in_a_pattern_keeps_its_key() {
    assert_eq!(
        renamed_text(
            "function f(a, o) { let x; [{x}] = a; ({b: {x}} = o); return x; }",
            "x",
            "y"
        ),
        "function f(a,o){let y;[{x:y}]=a;({b:{x:y}}=o);return y}"
    );
}

#[test]
fn a_renamed_shorthand_target_beside_a_rest_of_the_same_name_keeps_its_key() {
    assert_eq!(
        renamed_text(
            "function f(o) { let x; ({x, ...x} = o); return x; }",
            "x",
            "y"
        ),
        "function f(o){let y;({x:y,...y}=o);return y}"
    );
}

#[test]
fn a_shorthand_target_left_alone_stays_shorthand() {
    assert_eq!(
        renamed_text(
            "function f(o) { let x, z; ({x, z = 1} = o); return x + z; }",
            "x",
            "y"
        ),
        "function f(o){let y,z;({x:y,z=1}=o);return y+z}"
    );
}

#[test]
fn a_renamed_shorthand_binding_property_keeps_its_key() {
    assert_eq!(
        renamed_text(
            "function f(o) { let {x, w: {x: v} = 1} = o; let {z = 2} = o; return x + v + z; }",
            "x",
            "y"
        ),
        "function f(o){let{x:y,w:{x:v}=1}=o;let{z=2}=o;return y+v+z}"
    );
    assert_eq!(
        renamed_text("function f(o) { let {x = 2} = o; return x; }", "x", "y"),
        "function f(o){let{x:y=2}=o;return y}"
    );
}

#[test]
fn a_renamed_shorthand_object_literal_property_keeps_its_key() {
    assert_eq!(
        renamed_text("function f() { let x = 1; return {x}; }", "x", "y"),
        "function f(){let y=1;return{x:y}}"
    );
}

#[test]
fn a_renamed_shorthand_proto_property_keeps_an_own_proto_key() {
    assert_eq!(
        renamed_text(
            "function f() { let __proto__ = 1; return {__proto__}; }",
            "__proto__",
            "y"
        ),
        "function f(){let y=1;return{[\"__proto__\"]:y}}"
    );
}

#[test]
fn a_renamed_export_specifier_keeps_its_exported_name() {
    assert_eq!(
        renamed_text("let x = 1; export { x }; export { x as z };", "x", "y"),
        "let y=1;export{y as x};export{y as z};"
    );
}

#[test]
fn a_renamed_import_specifier_keeps_its_imported_name() {
    assert_eq!(
        renamed_text(
            "import { x } from \"m\"; import d from \"m\"; console.log(x, d);",
            "x",
            "y"
        ),
        "import{x as y}from\"m\";import d from\"m\";console.log(y,d);"
    );
}
