mod common;

use common::unique_config;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::base_name::{base_name, BaseNamer, Context};
use untwist_rename::collect::{collect, BindingInfo};
use untwist_rename::oracle::OuterRule;
use untwist_rename::usage::{collect_usage, Usage};

fn bases(source: &str) -> Vec<(String, String)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let usage = collect_usage(&parsed.program, &sem.scoping);
    let cx = Context {
        collected: &collected,
        usage: &usage,
        namer: None,
        config: unique_config(),
    };
    collected
        .bindings
        .iter()
        .map(|info| (info.name.clone(), base_name(info, &cx)))
        .collect()
}

fn base_of(source: &str, name: &str) -> String {
    bases(source)
        .into_iter()
        .find(|(binding, _)| binding == name)
        .map(|(_, base)| base)
        .expect("binding")
}

fn unique(source: &str) -> String {
    common::rename(source, unique_config())
}

#[test]
fn declarations_get_their_kind_word() {
    assert_eq!(base_of("function decl() {}", "decl"), "fn");
    assert_eq!(base_of("class Decl {}", "Decl"), "cls");
    assert_eq!(base_of("try {} catch (caught) {}", "caught"), "e");
}

#[test]
fn initialiser_kinds_get_their_type_word() {
    let source = r#"var a = function () {}; var b = () => {}; var c = class {}; var d = [];
                    var e = "s"; var f = 1; var g = true; var h = {}; var i2 = /x/; var j;"#;
    assert_eq!(base_of(source, "a"), "fn");
    assert_eq!(base_of(source, "b"), "fn");
    assert_eq!(base_of(source, "c"), "cls");
    assert_eq!(base_of(source, "d"), "arr");
    assert_eq!(base_of(source, "e"), "str");
    assert_eq!(base_of(source, "f"), "num");
    assert_eq!(base_of(source, "g"), "bool");
    assert_eq!(base_of(source, "h"), "obj");
    assert_eq!(base_of(source, "i2"), "re");
    assert_eq!(base_of(source, "j"), "v");
}

#[test]
fn loop_counters_follow_their_nesting_depth() {
    let source = "for (var p = 0; ; ) { for (var q = 0; ; ) { for (var r = 0; ; ) { for (var s = 0; ; ) {} } } }";
    assert_eq!(base_of(source, "p"), "i");
    assert_eq!(base_of(source, "q"), "j");
    assert_eq!(base_of(source, "r"), "k");
    assert_eq!(base_of(source, "s"), "i");
}

#[test]
fn a_parameter_with_no_evidence_falls_back() {
    assert_eq!(base_of("function f(unknownParam) {}", "unknownParam"), "v");
}

struct FixedNamer;

impl BaseNamer for FixedNamer {
    fn base_name(&self, binding: &BindingInfo, _usage: &Usage) -> Option<String> {
        (binding.name == "target").then(|| "fromHook".to_string())
    }
}

#[test]
fn the_base_namer_hook_wins_over_every_structural_rule() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function target() {} function other() {}");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let usage = collect_usage(&parsed.program, &sem.scoping);
    let namer = FixedNamer;
    let cx = Context {
        collected: &collected,
        usage: &usage,
        namer: Some(&namer),
        config: unique_config(),
    };

    let named = |name: &str| {
        collected
            .bindings
            .iter()
            .find(|info| info.name == name)
            .map(|info| base_name(info, &cx))
            .expect("binding")
    };
    assert_eq!(named("target"), "fromHook");
    assert_eq!(named("other"), "fn");
}

#[test]
fn a_base_that_is_not_an_identifier_falls_back() {
    struct JunkNamer;
    impl BaseNamer for JunkNamer {
        fn base_name(&self, _binding: &BindingInfo, _usage: &Usage) -> Option<String> {
            Some("---".to_string())
        }
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f() {}");
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let usage = collect_usage(&parsed.program, &sem.scoping);
    let namer = JunkNamer;
    let cx = Context {
        collected: &collected,
        usage: &usage,
        namer: Some(&namer),
        config: unique_config(),
    };
    assert_eq!(base_name(&collected.bindings[0], &cx), "v");
}

#[test]
fn an_identifier_alias_takes_its_target_base_name() {
    assert_eq!(
        unique("var handler = document.querySelector(\"a\");\nvar h = handler;\nh.click();"),
        "var el1 = document.querySelector(\"a\");\nvar el2 = el1;\nel2.click();"
    );
}

#[test]
fn an_alias_of_a_require_follows_to_the_module_name() {
    assert_eq!(
        unique("var lib = require(\"lodash\");\nvar l = lib;\nl.map();"),
        "var lodash1 = require(\"lodash\");\nvar lodash2 = lodash1;\nlodash2.map();"
    );
}

#[test]
fn a_cyclic_alias_pair_falls_back_rather_than_looping() {
    let out = unique("var a = b;\nvar b = a;\nuse(a, b);");
    assert!(out.starts_with("var v1 = v2;"), "{out}");
    assert!(out.contains("var v2 = v1;"), "{out}");
}

#[test]
fn an_alias_chain_longer_than_the_cap_falls_back() {
    let out = unique(
        "var fn0 = function () {};\nvar a1 = fn0;\nvar a2 = a1;\nvar a3 = a2;\nvar a4 = a3;\nvar a5 = a4;\nuse(a5);",
    );
    assert!(out.contains("var v1 = fn5;"), "{out}");
}

#[test]
fn an_alias_of_a_function_or_class_declaration_takes_the_declarations_kind_word() {
    assert_eq!(
        base_of("function helper() {}\nvar h = helper;\nh();", "h"),
        "fn"
    );
    assert_eq!(base_of("class Thing {}\nvar t = Thing;\nt;", "t"), "cls");
}

#[test]
fn an_alias_of_an_unresolved_global_falls_back() {
    assert_eq!(base_of("var w = window;\nw;", "w"), "v");
    assert_eq!(unique("var w = window;\nw;"), "var v1 = window;\nv1;");
}
