mod common;

use common::ZeroSpans;
use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, PassCtor, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

fn run_one<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    common::run_ctors(
        alloc,
        source,
        &[untwist_deobfuscate::control_flow_object::boxed],
    )
}

#[test]
fn assigned_keys_are_folded_into_the_alias_declaration() {
    assert_transform!(
        "function hi() {\n  var obj = {};\n  obj['kHAOU'] = '5|1|2';\n  obj['xxxxx'] = function (a, b) { return a < b; };\n  var alias = obj;\n  console.log(alias.kHAOU, alias.xxxxx(1, 2));\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      console.log("5|1|2", 1 < 2);
    }
    "#
    );
}

#[test]
fn a_partially_populated_object_is_completed() {
    assert_transform!(
        "function hi() {\n  var obj = { aaaaa: '1' };\n  obj['bbbbb'] = '2';\n  var alias = obj;\n  console.log(alias.aaaaa, alias.bbbbb);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      console.log("1", "2");
    }
    "#
    );
}

#[test]
fn a_split_string_value_is_merged_before_it_is_folded() {
    assert_transform!(
        "function hi() {\n  var obj = {};\n  obj['kHAOU'] = '5|1|2' + '|4|3|' + '0|6';\n  var alias = obj;\n  console.log(alias.kHAOU);\n}",
        [
            untwist_unminify::merge_strings::boxed,
            untwist_deobfuscate::control_flow_object::boxed
        ],
        @r#"
    function hi() {
      console.log("5|1|2|4|3|0|6");
    }
    "#
    );
}

#[test]
fn a_missing_alias_declaration_leaves_the_assignments_alone() {
    assert_transform!(
        "function hi() {\n  var obj = {};\n  obj['kHAOU'] = '5|1|2';\n  console.log(obj.kHAOU);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = {};
      obj["kHAOU"] = "5|1|2";
      console.log(obj.kHAOU);
    }
    "#
    );
}

#[test]
fn an_extra_reference_blocks_the_fold() {
    assert_transform!(
        "function hi() {\n  var obj = {};\n  obj['kHAOU'] = '5|1|2';\n  fn(obj);\n  var alias = obj;\n  console.log(alias.kHAOU);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = {};
      obj["kHAOU"] = "5|1|2";
      fn(obj);
      var alias = obj;
      console.log(alias.kHAOU);
    }
    "#
    );
}

#[test]
fn a_statement_list_the_scan_abandons_is_left_byte_identical() {
    assert_transform!(
        "function hi() {\n  before();\n  var obj = {};\n  obj['kHAOU'] = '5|1|2';\n  after();\n  console.log(obj.kHAOU);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      before();
      var obj = {};
      obj["kHAOU"] = "5|1|2";
      after();
      console.log(obj.kHAOU);
    }
    "#
    );
}

#[test]
fn a_non_dispatch_value_stops_the_scan_without_a_note() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function hi() {\n  var obj = {};\n  obj['kHAOU'] = compute();\n  var alias = obj;\n  console.log(alias.kHAOU);\n}",
    );
    assert!(code.contains("var alias = obj"), "{code}");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn an_assigned_key_that_repeats_a_declared_key_refuses_the_fold() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function hi() {\n  var obj = { aaaaa: '1' };\n  obj['aaaaa'] = '2';\n  var alias = obj;\n  console.log(alias.aaaaa);\n}",
    );
    assert!(code.contains("var alias = obj"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("control_flow_duplicate_key: aaaaa")),
        "{notes:?}"
    );
}

#[test]
fn two_assignments_to_the_same_key_refuse_the_fold() {
    let alloc = Allocator::default();
    let (code, notes) = run_one(
        &alloc,
        "function hi() {\n  var obj = {};\n  obj['aaaaa'] = '1';\n  obj['aaaaa'] = '2';\n  var alias = obj;\n  console.log(alias.aaaaa);\n}",
    );
    assert!(code.contains("var alias = obj"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("control_flow_duplicate_key: aaaaa")),
        "{notes:?}"
    );
}

#[test]
fn a_reassigned_temporary_blocks_the_fold() {
    assert_transform!(
        "function hi() {\n  var obj = {};\n  obj['kHAOU'] = '5|1|2';\n  obj = other;\n  var alias = obj;\n  console.log(alias.kHAOU);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = {};
      obj["kHAOU"] = "5|1|2";
      obj = other;
      var alias = obj;
      console.log(alias.kHAOU);
    }
    "#
    );
}

#[test]
fn a_non_dispatch_key_stops_the_scan() {
    assert_transform!(
        "function hi() {\n  var obj = {};\n  obj['toolongkey'] = '5|1|2';\n  var alias = obj;\n  console.log(alias.toolongkey);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = {};
      obj["toolongkey"] = "5|1|2";
      var alias = obj;
      console.log(alias.toolongkey);
    }
    "#
    );
}

#[test]
fn a_declared_object_that_is_not_dispatch_shaped_is_not_folded() {
    assert_transform!(
        "function hi() {\n  var obj = { other: 1 };\n  obj['kHAOU'] = '5|1|2';\n  var alias = obj;\n  console.log(alias.kHAOU, alias.other);\n}",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    function hi() {
      var obj = { other: 1 };
      obj["kHAOU"] = "5|1|2";
      var alias = obj;
      console.log(alias.kHAOU, alias.other);
    }
    "#
    );
}

const NATURAL_NAMES: &str = "function hi() {\n  var t = {};\n  t.add = function (a, b) { return a + b; };\n  t.label = \"total\";\n  var dispatch = t;\n  console.log(dispatch.add(1, 2), dispatch.label);\n}";
const FIVE_LETTER: &str = "function hi() {\n  var t = {};\n  t.aokfw = function (a, b) { return a + b; };\n  t.rLxJs = \"total\";\n  var dispatch = t;\n  console.log(dispatch.aokfw(1, 2), dispatch.rLxJs);\n}";
const ONE_NATURAL_KEY: &str = "function hi() {\n  var t = {};\n  t.add = function (a, b) { return a + b; };\n  var dispatch = t;\n  console.log(dispatch.add(1, 2));\n}";

#[test]
fn the_default_folds_natural_key_assignments() {
    let (code, _notes) = common::run_all_stages_extended(NATURAL_NAMES);
    assert_eq!(
        code.trim(),
        "function hi() {\n  console.log(1 + 2, \"total\");\n}"
    );
}

#[test]
fn conservative_still_folds_only_five_letter_keys() {
    let (code, _notes) = common::run_all_stages(NATURAL_NAMES);
    assert!(code.contains("dispatch.add(1, 2)"), "{code}");
    let (code, _notes) = common::run_all_stages(FIVE_LETTER);
    assert_eq!(
        code.trim(),
        "function hi() {\n  console.log(1 + 2, \"total\");\n}"
    );
}

#[test]
fn a_single_natural_key_is_not_a_dispatch_object_in_either_mode() {
    let alloc = Allocator::default();
    for code in [
        common::default_run(
            &alloc,
            ONE_NATURAL_KEY,
            untwist_deobfuscate::control_flow_object::boxed,
        ),
        common::conservative_run(
            &alloc,
            ONE_NATURAL_KEY,
            untwist_deobfuscate::control_flow_object::boxed,
        ),
    ] {
        assert!(code.contains("t.add ="), "{code}");
    }
}

#[test]
fn a_folded_key_assignment_carries_real_synthetic_spans() {
    let alloc = Allocator::default();
    let source = "function hi() {\n  var t = {};\n  t.aokfw = function (a, b) { return a + b; };\n  t.rLxJs = \"total\";\n  var dispatch = t;\n  console.log(dispatch.aokfw(1, 2), dispatch.rLxJs);\n  use(dispatch);\n}";
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let ctors: Vec<PassCtor<'_>> = vec![untwist_deobfuscate::control_flow_object::boxed];
    let mut group = PassGroup::from_ctors(&ctors, PassOptions::default());
    let (_sem, _report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    let mut scan = ZeroSpans::default();
    scan.visit_program(&parsed.program);
    assert!(scan.0.is_empty(), "{:?}", scan.0);
}
