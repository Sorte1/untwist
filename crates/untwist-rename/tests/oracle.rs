mod common;

use common::{assert_scope_safe, rename_compact};
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_rename::collect::collect;
use untwist_rename::fingerprint::alpha_fingerprint;
use untwist_rename::oracle::{NameOracle, OuterRule};
use untwist_rename::reserved::Excluded;
use untwist_rename::{RenameConfig, RenameMode};

fn probe(source: &str, rule: OuterRule, binding: &str, candidate: &str) -> bool {
    probe_deferring(source, rule, binding, candidate, &[])
}

fn probe_deferring(
    source: &str,
    rule: OuterRule,
    binding: &str,
    candidate: &str,
    deferred: &[&str],
) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, rule);
    let named = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|info| info.name == name)
            .map(|info| info.symbol)
            .expect("binding")
    };
    let pending = deferred.iter().map(|name| named(name)).collect::<Vec<_>>();
    let mut oracle = NameOracle::new(&sem.scoping, &collected);
    oracle.defer(pending);
    oracle.is_available(candidate, named(binding))
}

#[test]
fn reserved_words_and_program_globals_are_never_available() {
    let source = "function f(p) { return console; }";
    assert!(!probe(source, OuterRule::UsedEnclosing, "p", "class"));
    assert!(!probe(source, OuterRule::UsedEnclosing, "p", "undefined"));
    assert!(!probe(source, OuterRule::UsedEnclosing, "p", "console"));
    assert!(probe(source, OuterRule::UsedEnclosing, "p", "a"));
}

#[test]
fn a_sibling_binding_blocks_the_name() {
    assert!(!probe(
        "function f(p, q) {}",
        OuterRule::UsedEnclosing,
        "p",
        "q"
    ));
}

#[test]
fn a_descendant_binding_blocks_a_name_only_when_the_symbol_is_free_inside_it() {
    let shadowed_param = "var a = 1; var b = 2; function c(b) { return a + b; }";
    assert!(probe(shadowed_param, OuterRule::UsedEnclosing, "b", "b"));

    let shadowed_block = "var b = 2; function c() { var x = b; { let b = 1; b; } }";
    assert!(probe(shadowed_block, OuterRule::UsedEnclosing, "b", "b"));

    let captured = "var outer = 1; function f() { var blocked = 1; return outer + blocked; }";
    assert!(!probe(
        captured,
        OuterRule::UsedEnclosing,
        "outer",
        "blocked"
    ));
    assert!(!probe(captured, OuterRule::ProgramWide, "outer", "blocked"));
}

#[test]
fn an_enclosing_binding_blocks_only_when_the_rule_or_a_reference_says_so() {
    let used = "var taken = 1; function f(p) { return taken; }";
    assert!(!probe(used, OuterRule::UsedEnclosing, "p", "taken"));
    assert!(!probe(used, OuterRule::ProgramWide, "p", "taken"));

    let unused = "var taken = 1; function f(p) { return p; } taken;";
    assert!(probe(unused, OuterRule::UsedEnclosing, "p", "taken"));
    assert!(!probe(unused, OuterRule::ProgramWide, "p", "taken"));
}

#[test]
fn the_program_wide_rule_rejects_a_name_bound_in_an_unrelated_scope() {
    let source = "function f(p) {} function g() { var elsewhere = 1; return elsewhere; }";
    assert!(!probe(source, OuterRule::ProgramWide, "p", "elsewhere"));
    assert!(probe(source, OuterRule::UsedEnclosing, "p", "elsewhere"));
}

#[test]
fn a_symbol_may_keep_the_name_it_already_holds() {
    assert!(probe("function f(p) {}", OuterRule::ProgramWide, "p", "p"));
}

#[test]
fn taking_a_second_name_releases_the_first_but_keeps_it_handed_out() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "function f(p) { return p; } function g(q) {}");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let named = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|info| info.name == name)
            .map(|info| info.symbol)
            .expect("binding")
    };
    let (p, q) = (named("p"), named("q"));
    let mut oracle = NameOracle::new(&sem.scoping, &collected);

    assert_eq!(
        oracle.take(p, ["a".to_string()].into_iter()).as_deref(),
        Some("a")
    );
    assert_eq!(oracle.current_name(p), "a");
    assert_eq!(
        oracle.take(p, ["b".to_string()].into_iter()).as_deref(),
        Some("b")
    );
    assert_eq!(oracle.current_name(p), "b");
    assert!(
        !oracle.is_available("a", q),
        "a handed-out name is never reused"
    );
    assert!(
        oracle.is_available("p", p),
        "the entry name stays the symbol's own"
    );
    assert!(!oracle.is_available("p", q));

    let plan = oracle.into_plan();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan.apply(&mut parsed.program, &sem.scoping, &&alloc), 1);
    assert_eq!(
        print(&parsed.program, PrintStyle::Compact),
        "function f(b){return b}function g(q){}"
    );
}

#[test]
fn taking_a_name_records_it_and_the_plan_rewrites_every_reference() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "function f(p) { return p + 1; } f(2);");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let mut oracle = NameOracle::new(&sem.scoping, &collected);
    let param = collected
        .bindings
        .iter()
        .find(|b| b.name == "p")
        .map(|b| b.symbol)
        .expect("p");
    let function = collected
        .bindings
        .iter()
        .find(|b| b.name == "f")
        .map(|b| b.symbol)
        .expect("f");

    assert_eq!(
        oracle.take(param, ["a".to_string()].into_iter()).as_deref(),
        Some("a")
    );
    assert_eq!(
        oracle
            .take(function, ["a".to_string(), "b".to_string()].into_iter())
            .as_deref(),
        Some("b")
    );
    let plan = oracle.into_plan();
    assert_eq!(plan.len(), 2);

    let renamed = plan.apply(&mut parsed.program, &sem.scoping, &&alloc);
    assert_eq!(renamed, 2);
    assert_eq!(
        print(&parsed.program, PrintStyle::Compact),
        "function b(a){return a+1}b(2);"
    );
}

#[test]
fn the_alpha_fingerprint_sees_capture_and_ignores_pure_renaming() {
    let before = "var outer = 1; function f() { var inner = 2; inner; return outer; }";
    let safe = "var $a = 1; function $b() { var $c = 2; $c; return $a; }";
    let captured = "var outer = 1; function f() { var outer = 2; outer; return outer; }";

    assert_eq!(alpha_fingerprint(before), alpha_fingerprint(safe));
    assert_ne!(
        alpha_fingerprint(before),
        alpha_fingerprint(captured),
        "the capture changes resolution, not the number of bindings"
    );
    assert!(
        alpha_fingerprint("var = ;").is_none(),
        "a source that does not parse has no fingerprint"
    );
    assert_scope_safe(before, safe);
}

#[test]
fn the_fingerprint_expands_object_literal_shorthand_so_a_renamed_key_shows() {
    assert_ne!(
        alpha_fingerprint("let a = 1; f({ a });"),
        alpha_fingerprint("let b = 1; f({ b });"),
        "renaming the binding renames the own property, which the fingerprint must see"
    );
    assert_eq!(
        alpha_fingerprint("let a = 1; f({ a: a });"),
        alpha_fingerprint("let b = 1; f({ a: b });"),
        "a colon-form property keeps its key, so renaming the value changes nothing"
    );
}

#[test]
fn the_fingerprint_expands_every_shorthand_form() {
    assert_ne!(
        alpha_fingerprint("const { a } = o; a;"),
        alpha_fingerprint("const { b } = o; b;"),
        "a binding-pattern shorthand reads a different property once renamed"
    );
    assert_eq!(
        alpha_fingerprint("const { a: a } = o; a;"),
        alpha_fingerprint("const { a: b } = o; b;"),
        "a colon-form binding property keeps its key"
    );
    assert_ne!(
        alpha_fingerprint("let a; ({ a } = o); a;"),
        alpha_fingerprint("let b; ({ b } = o); b;"),
        "a shorthand assignment target is its own key"
    );
    assert_eq!(
        alpha_fingerprint("let a; ({ a: a } = o); a;"),
        alpha_fingerprint("let b; ({ a: b } = o); b;"),
        "a colon-form assignment target keeps its key"
    );
    assert_ne!(
        alpha_fingerprint("let __proto__ = 1; f({ __proto__ });"),
        alpha_fingerprint("let v = 1; f({ v });"),
        "the C1 class: oxc prints `__proto__` shorthand verbatim, so expansion must be forced"
    );
}

#[test]
fn every_shorthand_form_fingerprints_with_its_key_spelled_out() {
    let printed = |source: &str| alpha_fingerprint(source).expect("the source parses");
    assert!(printed("let a = 1; f({ a });").contains("{a:$s0}"));
    assert!(printed("const { a } = o; a;").contains("{a:$s0}"));
    assert!(printed("let a; ({ a } = o); a;").contains("({a:$s0}="));
    assert!(printed("let a; ({ a = 2 } = o); a;").contains("({a:$s0=2}="));
    assert!(
        printed("let __proto__ = 1; f({ __proto__ });").contains("{__proto__:$s0}"),
        "the one key whose shorthand flag oxc honours verbatim must still expand"
    );
}

#[test]
fn a_global_shaped_like_a_placeholder_cannot_mask_a_capture() {
    let before = "var outer = 1; function f() { var inner = 2; inner; return outer; }; $s0;";
    let captured = "var outer = 1; function f() { var outer = 2; outer; return outer; }; $s0;";
    assert_ne!(
        alpha_fingerprint(before),
        alpha_fingerprint(captured),
        "an unresolved `$s0` must not collide with the placeholders"
    );
    let printed = alpha_fingerprint(before).expect("the source parses");
    assert!(printed.contains("$ss0"), "{printed}");
}

#[test]
fn a_warning_only_diagnostic_still_yields_a_fingerprint() {
    assert!(
        alpha_fingerprint("var a = 1; a;").is_some(),
        "a clean parse has a fingerprint"
    );
    assert!(
        alpha_fingerprint("var = ;").is_none(),
        "an error-severity diagnostic has none"
    );
}

#[test]
fn assert_scope_safe_rejects_a_real_capture() {
    let before = "var outer = 1; function f() { var inner = 2; inner; return outer; }";
    let captured = "var outer = 1; function f() { var outer = 2; outer; return outer; }";
    let caught = std::panic::catch_unwind(|| assert_scope_safe(before, captured));
    assert!(
        caught.is_err(),
        "the guard every rename test leans on must fail on a capture"
    );
}

#[test]
fn the_fingerprint_is_name_blind_so_renaming_onto_a_global_is_asserted_directly() {
    let before = "var _0xa = 1; use(_0xa);";
    let onto_a_pseudo_global = "var undefined = 1; use(undefined);";
    assert_eq!(
        alpha_fingerprint(before),
        alpha_fingerprint(onto_a_pseudo_global),
        "the fingerprint proves binding structure, never the names chosen"
    );
    for mode in [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique] {
        let output = rename_compact(
            before,
            RenameConfig {
                mode,
                rename_all: true,
                extended: true,
            },
        );
        assert!(
            !output.contains("var undefined"),
            "{mode:?} shadowed a pseudo-global, which no fingerprint can see: {output}"
        );
    }
}

#[test]
fn a_deferred_symbol_holds_nothing_under_used_enclosing() {
    let siblings = "var b = 1; var a = 2;";
    assert!(!probe(siblings, OuterRule::UsedEnclosing, "b", "a"));
    assert!(probe_deferring(
        siblings,
        OuterRule::UsedEnclosing,
        "b",
        "a",
        &["a"]
    ));
    assert!(!probe_deferring(
        siblings,
        OuterRule::ProgramWide,
        "b",
        "a",
        &["a"]
    ));

    let read_before_declared = "function f(p) { { let q = b; q; } var b = 1; }";
    assert!(!probe(
        read_before_declared,
        OuterRule::UsedEnclosing,
        "q",
        "b"
    ));
    assert!(probe_deferring(
        read_before_declared,
        OuterRule::UsedEnclosing,
        "q",
        "b",
        &["b"]
    ));
}

fn keeping_probe(source: &str, rule: OuterRule, binding: &str, candidate: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, rule);
    let symbol = collected
        .bindings
        .iter()
        .find(|info| info.name == binding)
        .map(|info| info.symbol)
        .expect("binding");
    let oracle = NameOracle::keeping(
        &sem.scoping,
        &collected,
        Excluded::from_scoping(&sem.scoping),
    );
    oracle.is_available(candidate, symbol)
}

#[test]
fn a_keeping_oracle_answers_under_the_rule_its_collection_was_made_with() {
    let unused = "var taken = 1; function f(p) { return p; } taken;";
    assert!(!keeping_probe(unused, OuterRule::ProgramWide, "p", "taken"));
    assert!(keeping_probe(
        unused,
        OuterRule::UsedEnclosing,
        "p",
        "taken"
    ));
    assert_eq!(
        keeping_probe(unused, OuterRule::UsedEnclosing, "p", "taken"),
        probe(unused, OuterRule::UsedEnclosing, "p", "taken")
    );
}
