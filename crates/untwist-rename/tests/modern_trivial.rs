use oxc_allocator::Allocator;
use oxc_ast::ast::{Function, Statement};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::modern::trivial::{wrapper, Value};

fn verdict(source: &str) -> Option<Value> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    build_scoping(&parsed.program);
    let function: &Function<'_> = match parsed.program.body.first() {
        Some(Statement::FunctionDeclaration(function)) => function,
        _ => panic!("no function in {source}"),
    };
    wrapper(function)
}

#[test]
fn the_rollup_form_is_a_trivial_wrapper() {
    let source = "function r() { if (h) return e; h = 1; e = {}; return e; }";
    assert_eq!(verdict(source), Some(Value::Other));
}

#[test]
fn the_terser_conditional_form_is_a_trivial_wrapper() {
    let unminified = "function r() { if (h) { return e; } else { h = 1; return e = {}; } }";
    assert_eq!(verdict(unminified), Some(Value::Other));
    let assigned_first = "function r() { if (h) { return e; } else { h = 1; e = !0; return e; } }";
    assert_eq!(verdict(assigned_first), Some(Value::True));
}

#[test]
fn the_negated_guard_form_is_a_trivial_wrapper() {
    let source = "function r() { if (!h) { h = 1; e = true; } return e; }";
    assert_eq!(verdict(source), Some(Value::True));
}

#[test]
fn a_wrapper_that_computes_its_value_is_not_trivial() {
    for source in [
        "function r() { if (h) { return e; } else { h = 1; return e = f(); } }",
        "function r() { if (!h) { h = 1; e = Object.is || g; } return e; }",
        "function r() { if (h) { return e; } else { h = 1; g(); return e = {}; } }",
        "function r() { if (h) { return e; } else { h = 1; return x = {}; } }",
        "function r() { if (h) { return e; } else { h = 2; return e = {}; } }",
        "function r() { if (!h) { h = 1; e = {}; } return x; }",
        "function r() { if (!h) { h = 1; } return e; }",
        "function r() { if (h) { return e; } else { h = 1; return e; } }",
        "function r() { if (h) { return e; } else { h = 1; return e += {}; } }",
        "function r(a) { if (h) { return e; } else { h = 1; return e = {}; } }",
    ] {
        assert_eq!(verdict(source), None, "{source}");
    }
}
