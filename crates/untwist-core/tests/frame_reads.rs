use oxc_allocator::Allocator;
use untwist_core::ast_util::frame_reads::{reads_arguments_property, reads_function_frame};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

fn reads(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    reads_function_frame(&parsed.program, &sem.scoping)
}

fn reads_arguments(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    reads_arguments_property(&parsed.program, &sem.scoping)
}

#[test]
fn a_literal_that_inherits_from_a_function_reads_its_frame() {
    for source in [
        "function g() { return ({ __proto__: g }).caller; }",
        "function g() { return { __proto__: g, x: 1 }[\"caller\"]; }",
        "function g() { return { \"__proto__\": g }.caller; }",
        "function g(k) { return ({ __proto__: g })[k]; }",
        "function g() { return ({ __proto__: ({ __proto__: g }).caller }).arguments[1]; }",
    ] {
        assert!(reads(source), "{source}");
    }
}

#[test]
fn a_caller_read_on_any_object_reads_a_frame() {
    for source in [
        "function g() { return [].caller; }",
        "function g() { return \"\".caller; }",
        "function g() { return `x`.caller; }",
        "function g() { return arguments.caller; }",
    ] {
        assert!(reads(source), "{source}");
    }
}

#[test]
fn a_re_pointed_prototype_ends_the_literal_exemption() {
    for source in [
        "function g(k) { Object.setPrototypeOf(Array.prototype, g); return [][k]; }",
        "function g(k) { Object.setPrototypeOf(arguments, g); return arguments[k]; }",
        "function g(k) { Reflect[\"setPrototypeOf\"](String.prototype, g); return \"\"[k]; }",
        "function g(k) { const o = []; o.__proto__ = g; return arguments[k]; }",
    ] {
        assert!(reads(source), "{source}");
    }
}

#[test]
fn a_binding_named_arguments_is_not_the_arguments_object() {
    for source in [
        "function g(k) { var arguments = g; return arguments[k]; }",
        "function h(arguments, k) { return arguments[k]; } h(h, 0);",
        "function g(k) { arguments = g; return arguments[k]; }",
        "function g(k) { { let arguments = g; return arguments[k]; } }",
        "function g(k) { return ((arguments) => arguments[k])(g); }",
        "const k = 0; arguments[k];",
    ] {
        assert!(reads(source), "{source}");
    }
}

#[test]
fn plain_literals_keep_their_exemption() {
    for source in [
        "function f(i) { return [1, 2][i] + \"ab\"[i]; }",
        "function f(i) { return ({ a: 1 })[i]; }",
    ] {
        assert!(!reads(source), "{source}");
    }
    assert!(!reads_arguments("function f(i) { return [1, 2][i]; }"));
}
