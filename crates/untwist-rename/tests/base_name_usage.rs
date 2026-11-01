mod common;

use common::unique_config;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::base_name::{base_name, Context};
use untwist_rename::collect::collect;
use untwist_rename::oracle::OuterRule;
use untwist_rename::usage::collect_naming_usage;

fn base_of(source: &str, name: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let usage = collect_naming_usage(&parsed.program, &sem.scoping);
    let cx = Context {
        collected: &collected,
        usage: &usage,
        namer: None,
        config: unique_config(),
    };
    collected
        .bindings
        .iter()
        .find(|info| info.name == name)
        .map(|info| base_name(info, &cx))
        .expect("binding")
}

#[test]
fn a_member_initialiser_takes_the_property_name() {
    assert_eq!(base_of("var a = obj.prop;", "a"), "prop");
    assert_eq!(base_of(r#"var a = obj["other"];"#, "a"), "other");
}

#[test]
fn call_initialisers_lose_their_accessor_prefix() {
    assert_eq!(base_of("var a = getUser();", "a"), "user");
    assert_eq!(base_of("var a = store.fetchOrders();", "a"), "orders");
    assert_eq!(base_of("var a = compute();", "a"), "compute");
}

#[test]
fn a_call_of_a_binding_this_run_renames_takes_its_human_name_or_else_its_base() {
    assert_eq!(
        base_of("var a = getUser(); function getUser() {}", "a"),
        "user"
    );
    assert_eq!(base_of("var a = new Thing(); class Thing {}", "a"), "thing");
    assert_eq!(
        base_of("var user1 = fn1(); function fn1() {}", "user1"),
        "user"
    );
    assert_eq!(base_of("var fn3 = fn1(); function fn1() {}", "fn3"), "fn");
    assert_eq!(
        base_of("var a = b(); var b = c(); function c() {}", "a"),
        "fn"
    );
    assert_eq!(base_of("var a = b(); var b = a();", "a"), "fn");
}

#[test]
fn dom_lookups_and_error_constructions_get_their_own_word() {
    assert_eq!(base_of("var a = document.getElementById('x');", "a"), "el");
    assert_eq!(base_of("var a = root.querySelector('.x');", "a"), "el");
    assert_eq!(base_of("var a = new TypeError('x');", "a"), "err");
    assert_eq!(base_of("var a = new Thing();", "a"), "thing");
}

#[test]
fn an_object_literal_value_takes_its_key() {
    let source = "var t = 1; var n = 2; send({ videoRef: t, cameraRef: n });";
    assert_eq!(base_of(source, "t"), "videoRef");
    assert_eq!(base_of(source, "n"), "cameraRef");
}

#[test]
fn a_destructured_property_takes_its_key() {
    assert_eq!(base_of("const { cos: v } = Math; v(1);", "v"), "cos");
}

#[test]
fn property_reads_name_http_pairs_elements_and_errors() {
    let source = "function handle(q, s) { q.body; s.json({}); }";
    assert_eq!(base_of(source, "q"), "req");
    assert_eq!(base_of(source, "s"), "res");
    assert_eq!(
        base_of("function f(node) { node.classList; }", "node"),
        "el"
    );
    assert_eq!(base_of("function f(x) { x.message; x.stack; }", "x"), "err");
}

#[test]
fn a_called_parameter_is_a_callback_and_a_called_local_is_a_function() {
    assert_eq!(base_of("function f(done) { done(); }", "done"), "cb");
    assert_eq!(base_of("var run = something; run();", "run"), "fn");
}

#[test]
fn a_catch_parameter_keeps_e_even_when_its_message_is_read() {
    assert_eq!(
        base_of("try {} catch (caught) { caught.message; }", "caught"),
        "e"
    );
}

#[test]
fn import_and_require_bindings_take_their_module_path() {
    assert_eq!(
        base_of(r#"import a from "/lib/some-lib-a/index.js";"#, "a"),
        "lib_someLibA"
    );
    assert_eq!(base_of(r#"import * as b from "./utils.js";"#, "b"), "utils");
    assert_eq!(base_of(r#"import { sin as c } from "m";"#, "c"), "sin");
    assert_eq!(base_of(r#"var d = require("node:fs");"#, "d"), "nodeFs");
}

#[test]
fn a_for_loop_counter_keeps_its_letter_while_its_neighbour_keeps_its_evidence() {
    let source = "for (var i = 0, len = a.length; i < len; i++) {}";
    assert_eq!(base_of(source, "i"), "i");
    assert_eq!(base_of(source, "len"), "length");
}

#[test]
fn a_counter_read_into_an_object_literal_is_still_a_counter() {
    let source = "for (var i = 0; i < n; i++) { out.push({ index: i }) }";
    assert_eq!(base_of(source, "i"), "i");
}
