use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::builtins::{intact, scan, Builtins, Reliance};
use untwist_core::semantic::{build_scoping, has_module_syntax};

const MATH_ROUND: Reliance = Reliance {
    roots: &["Math"],
    members: &[("Math", false, &["round"])],
};

fn verdict(source: &str) -> Builtins {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    scan(&parsed.program, &sem.scoping, module, false, &MATH_ROUND).verdict
}

fn first_look_is_intact(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    intact(&parsed.program, &sem.scoping, module, &MATH_ROUND)
}

#[test]
fn a_program_that_only_calls_the_member_is_intact() {
    assert_eq!(
        verdict("x = Math.round(1.5) + Math.floor(2.5);"),
        Builtins::Intact
    );
    assert!(first_look_is_intact("x = Math.round(1.5);"));
}

#[test]
fn a_write_to_a_member_nobody_relies_on_is_intact() {
    assert_eq!(verdict("Math.floor = function () {};"), Builtins::Intact);
}

#[test]
fn a_write_to_a_relied_on_member_is_written() {
    for source in [
        "Math.round = function () {};",
        "Math['round'] = function () {};",
        "Math[key] = function () {};",
        "delete Math.round;",
        "Object.defineProperty(Math, 'round', {});",
        "Reflect.set(Math, 'round', f);",
        "Object.assign(Math, patch);",
        "Math = {};",
        "globalThis.Math = {};",
        "window.Math.round = f;",
        "this.Math = {};",
        "globalThis[name] = {};",
    ] {
        assert_eq!(verdict(source), Builtins::Written, "{source}");
    }
}

#[test]
fn a_root_used_as_a_value_is_aliased() {
    for source in [
        "var M = Math;",
        "f(Math);",
        "var g = globalThis;",
        "f(this);",
    ] {
        assert_eq!(verdict(source), Builtins::Aliased, "{source}");
        assert!(!first_look_is_intact(source), "{source}");
    }
}

#[test]
fn a_root_outside_the_reliance_is_not_watched() {
    assert_eq!(verdict("var O = Object; JSON.parse = f;"), Builtins::Intact);
}

#[test]
fn a_local_binding_of_the_same_name_is_not_the_built_in() {
    assert_eq!(
        verdict("function f(Math) { Math.round = 1; return Math; }"),
        Builtins::Intact
    );
}
