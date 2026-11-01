use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::reserved::{is_reserved, Excluded};

#[test]
fn reserved_words_and_pseudo_globals_are_excluded() {
    for name in [
        "class", "function", "new", "typeof", "let", "static", "await", "yield",
    ] {
        assert!(is_reserved(name), "{name} must be reserved");
    }
    for name in [
        "arguments",
        "eval",
        "undefined",
        "NaN",
        "Infinity",
        "globalThis",
    ] {
        assert!(is_reserved(name), "{name} must be reserved");
    }
    for name in ["a", "handler", "fn", "vLN1", "klass"] {
        assert!(!is_reserved(name), "{name} must not be reserved");
    }
}

#[test]
fn every_unresolved_global_in_the_program_is_excluded() {
    let alloc = Allocator::default();
    let parsed = parse_program(
        &alloc,
        "console.log(window); function f(local) { return local; }",
    );
    let sem = build_scoping(&parsed.program);
    let excluded = Excluded::from_scoping(&sem.scoping);

    assert!(excluded.contains("console"));
    assert!(excluded.contains("window"));
    assert!(excluded.contains("class"));
    assert!(excluded.contains("undefined"));
    assert!(!excluded.contains("local"));
    assert!(!excluded.contains("a"));
}

#[test]
fn an_inserted_name_is_excluded() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var a = 1;");
    let sem = build_scoping(&parsed.program);
    let mut excluded = Excluded::from_scoping(&sem.scoping);
    assert!(!excluded.contains("chosen"));
    excluded.insert("chosen");
    assert!(excluded.contains("chosen"));
}
