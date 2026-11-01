use oxc_allocator::Allocator;
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::proof::{names_a_token, redeclared, spelled_at, written_outside};

#[test]
fn a_name_is_a_token_only_between_non_identifier_characters() {
    assert!(names_a_token("a().reverse()", &["a"]));
    assert!(names_a_token("window.a", &["a"]));
    assert!(!names_a_token("abc", &["a"]));
    assert!(!names_a_token("_a", &["a"]));
    assert!(!names_a_token("a$", &["a"]));
    assert!(names_a_token("x = _0x4c3e;", &["_0x4c3e"]));
    assert!(!names_a_token("Hello World!", &["a", "b"]));
    assert!(names_a_token("\u{e4} a", &["a"]));
    assert!(names_a_token("\u{e4}a", &["a"]));
    assert!(names_a_token("a\u{200c}", &["a"]));
    assert!(names_a_token("a\u{bc}", &["a"]));
    assert!(names_a_token("\u{2167}a", &["a"]));
    assert!(names_a_token("aa a", &["a"]));
    assert!(!names_a_token("", &["a"]));
    assert!(!names_a_token("a", &[""]));
}

#[test]
fn every_string_and_static_key_is_scanned_once() {
    let alloc = Allocator::default();
    let source = "var s = 'x b y'; o.c; ({ d: x } = o); `e`; var t = 'f';";
    let parsed = parse_program(&alloc, source);
    let found = spelled_at(&parsed.program, &["b", "c", "d", "e", "g"]);
    let text = |i: usize| found[i].map(|span| &source[span.start as usize..span.end as usize]);
    assert_eq!(text(0), Some("'x b y'"));
    assert_eq!(text(1), Some("c"));
    assert_eq!(text(2), Some("d"));
    assert_eq!(text(3), Some("e"));
    assert_eq!(found[4], None);
}

#[test]
fn a_regular_expression_source_spells_a_name() {
    let alloc = Allocator::default();
    let source = "eval(/a().reverse()/.source);";
    let parsed = parse_program(&alloc, source);
    let found = spelled_at(&parsed.program, &["a", "reverse", "b"]);
    assert!(found[0].is_some());
    assert!(found[1].is_some());
    assert_eq!(found[2], None);
}

#[test]
fn a_write_outside_the_decoder_statement_is_found() {
    let alloc = Allocator::default();
    let source = "function b() { b = function () {}; }\nb = 1;";
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let symbol = semantics.scoping.symbol_ids().next().unwrap();
    let statement = parsed.program.body[0].span();
    assert!(written_outside(
        &parsed.program,
        &semantics.scoping,
        symbol,
        statement
    ));
    let inside_only = parse_program(&alloc, "function b() { b = function () {}; }");
    let semantics = build_scoping(&inside_only.program);
    let symbol = semantics.scoping.symbol_ids().next().unwrap();
    let statement = inside_only.program.body[0].span();
    assert!(!written_outside(
        &inside_only.program,
        &semantics.scoping,
        symbol,
        statement
    ));
}

#[test]
fn a_write_inside_a_nested_decoder_statement_is_inside() {
    let alloc = Allocator::default();
    let source = "(function () {\n  function b() { b = function () {}; }\n  b();\n})();";
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let symbol = semantics
        .scoping
        .symbol_ids()
        .find(|symbol| semantics.scoping.symbol_name(*symbol) == "b")
        .unwrap();
    let start = source.find("function b").unwrap() as u32;
    let end = source.find("}; }").unwrap() as u32 + 4;
    let statement = oxc_span::Span::new(start, end);
    assert!(!written_outside(
        &parsed.program,
        &semantics.scoping,
        symbol,
        statement
    ));
}

fn redeclaration_of(source: &str, name: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let scoping = &semantics.scoping;
    let symbol = scoping
        .symbol_ids()
        .find(|symbol| {
            scoping.symbol_name(*symbol) == name
                && scoping.symbol_scope_id(*symbol) == scoping.root_scope_id()
        })
        .unwrap();
    redeclared(scoping, symbol)
        .map(|span| source[span.start as usize..span.end as usize].to_string())
}

#[test]
fn a_second_declaration_of_the_binding_is_found() {
    let twice = "function a() { return 1; }\nfunction a() { return 2; }";
    assert_eq!(redeclaration_of(twice, "a").as_deref(), Some("a"));
    let by_var = "function a() { return 1; }\nvar a = function () { return 2; };";
    assert_eq!(redeclaration_of(by_var, "a").as_deref(), Some("a"));
    let in_a_loop = "function a() { return 1; }\nfor (var a in {}) {}";
    assert_eq!(redeclaration_of(in_a_loop, "a").as_deref(), Some("a"));
    assert_eq!(redeclaration_of("function a() { return 1; }", "a"), None);
}

#[test]
fn a_function_declared_in_a_block_under_the_binding_is_found() {
    let block = "function a() { return 1; }\n{ function a() { return 2; } }";
    assert_eq!(redeclaration_of(block, "a").as_deref(), Some("a"));
    let branch = "function a() { return 1; }\nif (x) { function a() { return 2; } }";
    assert_eq!(redeclaration_of(branch, "a").as_deref(), Some("a"));
    let nested = "function a() { return 1; }\nfunction f() { { function a() { return 2; } } }";
    assert_eq!(redeclaration_of(nested, "a").as_deref(), Some("a"));
}

#[test]
fn a_shadowing_function_scope_declaration_is_not_a_redeclaration() {
    let shadow = "function a() { return 1; }\nfunction f() { function a() { return 2; } }";
    assert_eq!(redeclaration_of(shadow, "a"), None);
    let other = "function a() { return 1; }\n{ function c() { return 2; } }";
    assert_eq!(redeclaration_of(other, "a"), None);
}

#[test]
fn an_object_literal_key_does_not_spell_a_name() {
    let alloc = Allocator::default();
    let source = "var t = { d: 1, 'c': 2, get g() { return 3; }, [h]: 4 };";
    let parsed = parse_program(&alloc, source);
    assert_eq!(
        spelled_at(&parsed.program, &["d", "c", "g"]),
        vec![None, None, None]
    );
    let patterns = "var { d: x } = o; ({ c: y } = o); class K { g() {} }";
    let parsed = parse_program(&alloc, patterns);
    let found = spelled_at(&parsed.program, &["d", "c", "g"]);
    assert!(found.iter().all(Option::is_some), "{found:?}");
}
