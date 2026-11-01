use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::calls::agnostic::agnostic;

fn doubling_dag(depth: usize) -> String {
    let mut source = String::from("var g = this;\nfunction f0(o, k) { return o[k]; }\n");
    for level in 1..depth {
        let below = level - 1;
        source.push_str(&format!(
            "function f{level}(o, k) {{ return f{below}(o, k) + f{below}(o, k); }}\n"
        ));
    }
    source.push_str(&format!("f{}(g, 'x');\n", depth - 1));
    source
}

fn chain(length: usize) -> String {
    let mut source = String::from("var g = this;\nfunction f0(o, k) { return o[k]; }\n");
    for level in 1..length {
        let below = level - 1;
        source.push_str(&format!(
            "function f{level}(o, k) {{ return f{below}(o, k); }}\n"
        ));
    }
    source.push_str(&format!("f{}(g, 'x');\n", length - 1));
    source
}

fn code_text_judged(source: &str) -> bool {
    let allocator = Allocator::default();
    let program = parse_program(&allocator, source).program;
    let scoping = build_scoping(&program).scoping;
    agnostic(&program, &scoping, None, &[]).code_text.is_some()
}

#[test]
fn a_doubling_call_dag_ends_the_substitution_walk_failing_closed() {
    assert!(code_text_judged(&doubling_dag(48)));
}

#[test]
fn a_call_chain_deeper_than_the_inline_limit_fails_closed() {
    assert!(code_text_judged(&chain(200)));
}

#[test]
fn a_short_call_chain_and_a_small_dag_are_still_walked_through() {
    assert!(!code_text_judged(&chain(8)));
    assert!(!code_text_judged(&doubling_dag(8)));
}
