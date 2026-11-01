mod common;

use std::sync::mpsc;
use std::time::Duration;

use common::assert_scope_safe;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, Pass, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_rename::base_name::BaseNamer;
use untwist_rename::collect::BindingInfo;
use untwist_rename::unique::{
    release_turn, reserve, shared_state, FileSlot, SharedUniqueState, TurnGuard, Unique,
};
use untwist_rename::usage::Usage;
use untwist_rename::{rename_passes_with_state, RenameConfig, RenameMode};

fn config(rename_all: bool) -> RenameConfig {
    RenameConfig {
        mode: RenameMode::Unique,
        rename_all,
        extended: true,
    }
}

fn unique_with_slot(
    source: &str,
    state: &SharedUniqueState,
    slot: FileSlot,
    rename_all: bool,
) -> String {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly"
    );
    let sem = build_scoping(&parsed.program);
    let passes = rename_passes_with_state(config(rename_all), state, slot);
    let mut group = PassGroup::new(passes, PassOptions::default());
    let (_sem, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    assert!(
        report.converged(),
        "rename did not converge: stop={:?}",
        report.stop
    );
    let output = print(&parsed.program, PrintStyle::Compact);
    assert_scope_safe(source, &output);
    output
}

fn unique_with(source: &str, state: &SharedUniqueState, index: usize, rename_all: bool) -> String {
    let slot = reserve(state, index).expect("test reserves slots in ascending order");
    unique_with_slot(source, state, slot, rename_all)
}

fn unique(source: &str) -> String {
    unique_with(source, &shared_state(), 0, false)
}

fn unique_with_namer(source: &str, state: SharedUniqueState) -> String {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly"
    );
    let sem = build_scoping(&parsed.program);
    let slot = reserve(&state, 0).expect("a fresh state grants slot 0");
    let pass: Box<dyn Pass<'_> + '_> =
        Box::new(Unique::new(config(false), state, slot).with_namer(Box::new(FixedNamer)));
    let mut group = PassGroup::new(vec![pass], PassOptions::default());
    let (_sem, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    assert!(
        report.converged(),
        "rename did not converge: stop={:?}",
        report.stop
    );
    print(&parsed.program, PrintStyle::Compact)
}

#[test]
fn every_binding_gets_a_readable_base_and_a_number() {
    assert_eq!(
        unique("function _0xa() {} function _0xb() {}"),
        "function fn1(){}function fn2(){}"
    );
}

#[test]
fn counters_are_kept_per_base_name() {
    assert_eq!(
        unique(r#"var _0x1 = []; var _0x2 = []; var _0x3 = "s"; _0x1; _0x2; _0x3;"#),
        "var arr1=[];var arr2=[];var str1=`s`;arr1;arr2;str1;"
    );
}

#[test]
fn adding_an_unrelated_binding_renumbers_nothing() {
    let without = unique(r#"var _0x1 = []; var _0x2 = []; _0x1; _0x2;"#);
    let with = unique(r#"var _0x0 = 1; var _0x1 = []; var _0x2 = []; _0x0; _0x1; _0x2;"#);
    assert_eq!(without, "var arr1=[];var arr2=[];arr1;arr2;");
    assert_eq!(with, "var num1=1;var arr1=[];var arr2=[];num1;arr1;arr2;");
}

#[test]
fn two_files_in_one_run_share_the_counters() {
    let state = shared_state();
    assert_eq!(
        unique_with("function _0xa() {}", &state, 0, false),
        "function fn1(){}"
    );
    assert_eq!(
        unique_with("function _0xb() {}", &state, 1, false),
        "function fn2(){}"
    );
}

#[test]
fn slots_are_handed_out_in_source_order() {
    let state = shared_state();
    assert!(reserve(&state, 0).is_some());
    assert!(reserve(&state, 1).is_some());
    assert!(reserve(&state, 2).is_some());
}

#[test]
fn a_slot_taken_out_of_order_is_refused() {
    let state = shared_state();
    assert!(reserve(&state, 0).is_some());
    assert!(
        reserve(&state, 2).is_none(),
        "file 2 reserved before file 1, so its names would depend on completion order"
    );
}

#[test]
fn a_slot_is_never_handed_out_twice() {
    let state = shared_state();
    assert!(reserve(&state, 0).is_some());
    assert!(reserve(&state, 0).is_none());
}

#[test]
fn usage_derived_bases_reach_the_output() {
    let output =
        unique(r#"import _0xa from "/lib/some-lib-a/index.js"; var _0xb = getUser(); _0xa; _0xb;"#);
    assert!(output.contains("lib_someLibA1"), "{output}");
    assert!(output.contains("var user1=getUser();"), "{output}");
    assert!(!output.contains("_0x"), "{output}");
}

struct FixedNamer;

impl BaseNamer for FixedNamer {
    fn base_name(&self, binding: &BindingInfo, _usage: &Usage) -> Option<String> {
        (binding.name == "_0xa").then(|| "hooked".to_string())
    }
}

#[test]
fn the_base_namer_hook_replaces_the_base_step() {
    let output = unique_with_namer("function _0xa() {} function _0xb() {}", shared_state());
    assert_eq!(output, "function hooked1(){}function fn1(){}");
}

#[test]
fn names_never_collide_with_a_program_global() {
    assert_eq!(
        unique("function _0xa() { return fn1; }"),
        "function fn2(){return fn1}"
    );
}

#[test]
fn running_unique_twice_with_a_fresh_state_changes_nothing() {
    for rename_all in [false, true] {
        let source = "function _0xa(_0xb) { var _0xc = []; return _0xc.concat(_0xb); }";
        let once = unique_with(source, &shared_state(), 0, rename_all);
        let twice = unique_with(&once, &shared_state(), 0, rename_all);
        assert_eq!(
            once, twice,
            "Unique must be idempotent (rename_all={rename_all})"
        );
    }
}

fn unique_all(source: &str) -> String {
    unique_with(source, &shared_state(), 0, true)
}

#[test]
fn a_call_of_a_named_function_expression_takes_its_human_callee_name() {
    let once = unique_all("var result = function inner() {\n  return 2;\n}();");
    assert_eq!(once, "var inner1=function fn1(){return 2}();");
    assert_eq!(unique_all(&once), once);
}

#[test]
fn a_call_of_a_hoisted_function_takes_its_human_callee_name() {
    let once = unique_all("var user = getUser();\nfunction getUser() {\n  return 1;\n}");
    assert_eq!(once, "var user1=fn1();function fn1(){return 1}");
    assert_eq!(unique_all(&once), once);
}

#[test]
fn a_construction_of_a_renamed_class_takes_its_human_class_name() {
    let once = unique_all("var thing = new Thing();\nclass Thing {}");
    assert_eq!(once, "var thing1=new cls1;class cls1{}");
    assert_eq!(unique_all(&once), once);
}

#[test]
fn a_callee_that_does_not_look_human_gives_the_base_it_receives() {
    let once = unique_all("var _0xb = _0xa();\nfunction _0xa() {\n  return 1;\n}");
    assert_eq!(once, "var fn1=fn2();function fn2(){return 1}");
    assert_eq!(unique_all(&once), once);
}

#[test]
fn a_callee_name_ending_in_a_digit_gives_the_base_it_receives() {
    let once = unique_all("var hash = sha256();\nfunction sha256() {\n  return 1;\n}");
    assert_eq!(once, "var fn1=fn2();function fn2(){return 1}");
    assert_eq!(unique_all(&once), once);
}

#[test]
fn a_construction_of_a_hinted_class_reads_the_hint_with_a_digit() {
    let source = "var _0xb = new _0xa();\nfunction _0xa() {\n  this.type = \"Vector3\";\n}";
    let once = unique_all(source);
    assert!(once.starts_with("var vector31=new Vector31;"), "{once}");
    assert_eq!(unique_all(&once), once);
}

#[test]
fn a_source_this_run_does_not_rename_keeps_its_original_name_in_the_derivation() {
    assert_eq!(
        unique_all("var el = document.getElementById(\"x\");"),
        "var el1=document.getElementById(`x`);"
    );
    assert_eq!(
        unique("var _0xb = getUser();\nfunction getUser() {\n  return 1;\n}"),
        "var user1=getUser();function getUser(){return 1}"
    );
    assert_eq!(
        unique("var user = getUser();\nfunction getUser() {\n  return 1;\n}"),
        "var user=getUser();function getUser(){return 1}"
    );
}

#[test]
fn a_commonjs_wrapper_keeps_require_module_and_exports_under_rename_all() {
    let source = "(function (require, module, exports) {\n  var sum = require(\"./sum\");\n  var total = sum([1, 2]);\n  module.exports = total;\n})();";
    let once = unique_all(source);
    assert_eq!(
        once,
        "(function(require,module,exports){var sum1=require(\"./sum\");var sum2=sum1([1,2]);module.exports=sum2})();"
    );
    assert_eq!(unique_all(&once), once);
}

#[test]
fn an_amd_wrapper_keeps_define_under_rename_all() {
    let source = "(function (define) {\n  define([\"dep\"], function (dep) {\n    return dep;\n  });\n})(globalDefine);";
    let once = unique_all(source);
    assert_eq!(
        once,
        "(function(define){define([`dep`],function(v1){return v1})})(globalDefine);"
    );
    assert_eq!(unique_all(&once), once);
}

const TURNSTILE_GRACE: Duration = Duration::from_millis(200);

#[test]
fn the_second_slot_waits_until_the_first_has_renamed() {
    let state = shared_state();
    let first = reserve(&state, 0).expect("slot 0");
    let second = reserve(&state, 1).expect("slot 1");
    let (done, finished) = mpsc::channel();
    let worker_state = std::sync::Arc::clone(&state);
    let worker = std::thread::spawn(move || {
        let output = unique_with_slot("function _0xb() {}", &worker_state, second, false);
        done.send(()).ok();
        output
    });
    assert!(
        finished.recv_timeout(TURNSTILE_GRACE).is_err(),
        "slot 1 renamed before slot 0 had its turn"
    );
    assert_eq!(
        unique_with_slot("function _0xa() {}", &state, first, false),
        "function fn1(){}"
    );
    assert_eq!(worker.join().expect("worker"), "function fn2(){}");
}

#[test]
fn a_released_turn_lets_the_next_slot_through_without_renaming() {
    let state = shared_state();
    let _first = reserve(&state, 0).expect("slot 0");
    let second = reserve(&state, 1).expect("slot 1");
    release_turn(&state, 0);
    assert_eq!(
        unique_with_slot("function _0xb() {}", &state, second, false),
        "function fn1(){}"
    );
}

#[test]
fn releasing_a_turn_that_already_passed_changes_nothing() {
    let state = shared_state();
    let first = reserve(&state, 0).expect("slot 0");
    assert_eq!(
        unique_with_slot("function _0xa() {}", &state, first, false),
        "function fn1(){}"
    );
    release_turn(&state, 0);
    let second = reserve(&state, 1).expect("slot 1");
    assert_eq!(
        unique_with_slot("function _0xb() {}", &state, second, false),
        "function fn2(){}"
    );
}

#[test]
fn a_turn_guard_dropped_by_a_panic_still_passes_the_turn_on() {
    let state = shared_state();
    let _first = reserve(&state, 0).expect("slot 0");
    let second = reserve(&state, 1).expect("slot 1");
    let worker_state = std::sync::Arc::clone(&state);
    let worker = std::thread::spawn(move || {
        let _turn = TurnGuard::new(&worker_state, 0);
        panic!("the worker panicked on purpose while holding turn 0");
    });
    assert!(worker.join().is_err());
    assert_eq!(
        unique_with_slot("function _0xb() {}", &state, second, false),
        "function fn1(){}"
    );
}

fn assert_idempotent_both_ways(source: &str) -> String {
    let once = unique_all(source);
    assert_eq!(unique_all(&once), once, "rename_all is not idempotent");
    let kept = unique(source);
    assert_eq!(unique(&kept), kept, "keep-human-names is not idempotent");
    once
}

#[test]
fn a_constructor_type_string_names_the_class() {
    let once = assert_idempotent_both_ways(
        "class _0xa { constructor() { this.isMesh = true; this.type = \"Mesh\"; } }\nvar _0xb = new _0xa();",
    );
    assert!(once.starts_with("class Mesh1{"), "{once}");
    assert!(once.ends_with("var mesh1=new Mesh1;"), "{once}");
    assert!(
        unique("class _0xa { constructor() { this.type = \"Mesh\"; } }\n_0xa;")
            .starts_with("class Mesh1{")
    );
}

#[test]
fn a_type_string_outranks_an_is_flag() {
    let once = assert_idempotent_both_ways(
        "function _0xa() { this.isFoo = true; this.type = \"Bar\"; }\n_0xa;",
    );
    assert!(once.starts_with("function Bar1(){"), "{once}");
}

#[test]
fn an_is_flag_names_a_constructor_function() {
    let once =
        assert_idempotent_both_ways("function _0xa() { this.isWebGLRenderer = true; }\n_0xa;");
    assert!(once.starts_with("function WebGLRenderer1(){"), "{once}");
    let once =
        assert_idempotent_both_ways("var _0xa = function () {};\n_0xa.prototype.isColor = !0;");
    assert!(once.starts_with("var Color1=function"), "{once}");
    let once = assert_idempotent_both_ways("function _0xa() { this[\"isLight\"] = !0x0; }\n_0xa;");
    assert!(once.starts_with("function Light1(){"), "{once}");
}

#[test]
fn a_name_like_string_field_names_the_class() {
    let once =
        assert_idempotent_both_ways("var _0xa = class { static displayName = \"Panel\"; };\n_0xa;");
    assert!(once.starts_with("var Panel1=class"), "{once}");
    let once = assert_idempotent_both_ways("function _0xa() {}\n_0xa.prototype.name = \"Thing\";");
    assert!(once.starts_with("function Thing1(){"), "{once}");
}

#[test]
fn an_arrow_in_the_constructor_still_reads_the_instance() {
    let once = assert_idempotent_both_ways(
        "class _0xa { constructor() { (() => { this.type = \"Scene\"; })(); } }\n_0xa;",
    );
    assert!(once.starts_with("class Scene1{"), "{once}");
}

#[test]
fn strings_that_are_not_class_hints_change_nothing() {
    for body in [
        "class _0xa { m() { this.type = \"Mesh\"; } }",
        "class _0xa { constructor() { function f() { this.type = \"Mesh\"; } f(); } }",
        "class _0xa { constructor() { this.type = \"mesh\"; } }",
        "class _0xa { constructor() { this.type = \"Mesh Basic\"; } }",
        "class _0xa { constructor() { this.type = \"POINTS\"; } }",
        "class _0xa { constructor() { this.type = \"Iil\"; } }",
        "class _0xa { constructor() { this.isMesh = 1; } }",
        "class _0xa { constructor() { other.type = \"Mesh\"; } }",
    ] {
        let once = assert_idempotent_both_ways(&format!("{body}\n_0xa;"));
        assert!(once.starts_with("class cls1{"), "{body}: {once}");
    }
}

#[test]
fn a_human_class_name_ignores_a_different_hint() {
    let source = "class Button { constructor() { this.type = \"Primary\"; } }\nButton;";
    assert!(assert_idempotent_both_ways(source).starts_with("class cls1{"));
    assert!(unique(source).starts_with("class Button{"));
}

#[test]
fn a_transpiled_constructor_names_its_class_through_the_this_alias() {
    let once = assert_idempotent_both_ways(
        "function _0xa() { let _0xt; (_0xt = _0xs(this, _0xa)).isMesh = true; _0xt.type = \"Mesh\"; return _0xt; }\nfunction _0xs(a, b) { return a; }\n_0xa;",
    );
    assert!(once.starts_with("function Mesh1(){"), "{once}");
    let once = assert_idempotent_both_ways(
        "function _0xa() { var _0xt = _0xb.call(this) || this; _0xt.isGroup = true; return _0xt; }\nvar _0xb = function () {};\n_0xa;",
    );
    assert!(once.starts_with("function Group1(){"), "{once}");
}
