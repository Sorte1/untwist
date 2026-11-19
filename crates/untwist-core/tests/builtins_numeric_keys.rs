use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::builtins::{scan, Builtins, Reliance};
use untwist_core::semantic::{build_scoping, has_module_syntax};

const STRING: Reliance = Reliance {
    roots: &["String"],
    members: &[],
};

fn verdict(source: &str) -> Builtins {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let module = has_module_syntax(&parsed.program);
    scan(&parsed.program, &sem.scoping, module, false, &STRING).verdict
}

#[test]
fn a_key_proven_numeric_reaches_no_prototype() {
    for source in [
        "function f(a) { for (let i = 0; i < a.length; i++) g(a[i]); }",
        "function f(a, n) { let i = n - 1; i *= 2; g(a[i]); }",
        "function f(a, n) { let i = 0; i += 1; g(a[i + 1]); }",
        "function f(a, n) { g(a[n | 0], a[-n], a[~n], a[!n], a[n++]); }",
        "function f(p, q) { p = p - 277; var c = d(); return c[p]; }",
        "function f(a, c) { g(a[c -= 0], a[c = +c]); }",
        "var a = [1]; for (var i = 0; i < 1; i++) g(a[i]);",
        "function f(p, q) { p = p - 1; var a = d(); var k = p + a[0]; return c[k]; }",
        "function f(a, n, s) { g(a[(n | 0) + s + 'x']); }",
        "function f(a, k) { let i = 0; i += k; g(a[i]); }",
    ] {
        assert_eq!(verdict(source), Builtins::Intact, "{source}");
    }
}

#[test]
fn a_key_that_may_be_a_string_still_counts() {
    for source in [
        "function f(a, k) { g(a[k]); }",
        "function f(a, k) { let i = 0; i = k; g(a[i]); }",
        "function f(a, o) { for (let k in o) g(a[k]); }",
        "function f(a, o) { let i = 0; [i] = o; g(a[i]); }",
        "function f(a, n) { let i = 0; h = () => { i = n; }; g(a[i]); }",
        "function f(p) { var c = d(); g(c[p]); p = p - 1; }",
        "function f(p) { p = p - 1; arguments[0] = k; return d()[p]; }",
        "function f(p) { p = p - 1; p = k; return d()[p]; }",
        "function f(p, q = p) { p = p - 1; return d()[p]; }",
        "function f(p) { p = p - 1; var a = d(); var k = a[0] + p; return c[k]; }",
        "function f(a, s, n) { g(a[s + (n | 0)]); }",
        "var a = [1]; var i = 0; window.i = k; g(a[i]);",
        "var a = [1]; var i = 0; this[k] = 1; g(a[i]);",
    ] {
        assert_ne!(verdict(source), Builtins::Intact, "{source}");
    }
}
