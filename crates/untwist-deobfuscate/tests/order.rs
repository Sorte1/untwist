mod common;

use untwist_core::pass::{PassGroup, PassOptions, RunReport};
use untwist_deobfuscate::{
    cleanup_ctors, cleanup_fixpoint_ctors, deobfuscate_ctors, protections_ctors,
    CLEANUP_FIXPOINT_ORDER, CLEANUP_ORDER, DEOBFUSCATE_MAX_ROUNDS, DEOBFUSCATE_ORDER,
    INLINE_DECODED, NO_STRING_ARRAY_NOTE, OBJECT_PROPS, PROTECTIONS_ORDER,
};

const OBFUSCATED: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));";

fn run_report(source: &str) -> (String, RunReport) {
    common::run_report(source)
}

#[test]
fn the_composite_order_is_the_one_the_spec_fixes() {
    assert_eq!(
        DEOBFUSCATE_ORDER,
        &[
            "find-string-array",
            "find-decoders",
            "inline-object-props",
            "inline-decoder-wrappers",
            "inline-decoded-strings",
            "remove-decoder-machinery",
        ]
    );
}

#[test]
fn the_constructors_match_the_declared_order() {
    let names: Vec<&str> = deobfuscate_ctors()
        .iter()
        .map(|ctor| ctor().name())
        .collect();
    assert_eq!(names, DEOBFUSCATE_ORDER);
}

#[test]
fn the_object_props_index_names_the_object_props_pass() {
    assert_eq!(DEOBFUSCATE_ORDER[OBJECT_PROPS], "inline-object-props");
    assert_eq!(OBJECT_PROPS, 2);
}

#[test]
fn the_inline_decoded_index_names_the_inline_decoded_strings_pass() {
    assert_eq!(DEOBFUSCATE_ORDER[INLINE_DECODED], "inline-decoded-strings");
    assert_eq!(INLINE_DECODED, 4);
}

#[test]
fn the_rotator_is_still_reported_exactly_once_per_run() {
    let (_code, report) = run_report(OBFUSCATED);
    let found = report
        .notes
        .iter()
        .filter(|note| {
            note.message
                .starts_with(untwist_deobfuscate::notes::ROTATOR_NONE)
        })
        .count();
    assert_eq!(found, 1, "{:?}", report.notes);
}

#[test]
fn the_cleanup_fixpoint_order_is_the_one_the_spec_fixes() {
    assert_eq!(
        CLEANUP_FIXPOINT_ORDER,
        &[
            "merge-strings",
            "dead-code",
            "control-flow-object",
            "control-flow-switch"
        ]
    );
    let names: Vec<&str> = cleanup_fixpoint_ctors()
        .iter()
        .map(|ctor| ctor().name())
        .collect();
    assert_eq!(names, CLEANUP_FIXPOINT_ORDER);
}

#[test]
fn the_protections_order_is_the_one_the_spec_fixes() {
    assert_eq!(PROTECTIONS_ORDER, &["self-defending", "debug-protection"]);
    let names: Vec<&str> = protections_ctors()
        .iter()
        .map(|ctor| ctor().name())
        .collect();
    assert_eq!(names, PROTECTIONS_ORDER);
}

#[test]
fn the_cleanup_order_is_the_one_the_spec_fixes() {
    assert_eq!(
        CLEANUP_ORDER,
        &[
            "merge-object-assignments",
            "inline-object-props",
            "evaluate-globals",
            "fold-pure-calls",
            "var-transformation",
            "dead-assignment-elimination",
            "fold-literal-chains",
        ]
    );
    let names: Vec<&str> = cleanup_ctors().iter().map(|ctor| ctor().name()).collect();
    assert_eq!(names, CLEANUP_ORDER);
}

#[test]
fn the_second_registration_of_inline_object_props_is_superset_only() {
    let conservative = PassOptions {
        extended: false,
        ..PassOptions::default()
    };
    let group = PassGroup::from_ctors(&cleanup_ctors(), conservative);
    let names: Vec<&str> = group
        .per_pass_changes()
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert!(!names.contains(&"inline-object-props"), "{names:?}");
}

#[test]
fn the_cleanup_group_keeps_the_superset_passes_by_default_and_drops_them_under_conservative() {
    let default_group = PassGroup::from_ctors(
        &untwist_deobfuscate::cleanup_ctors(),
        PassOptions::default(),
    );
    let conservative_group = PassGroup::from_ctors(
        &untwist_deobfuscate::cleanup_ctors(),
        PassOptions {
            extended: false,
            ..PassOptions::default()
        },
    );
    assert_eq!(default_group.per_pass_changes().len(), 7);
    assert_eq!(conservative_group.per_pass_changes().len(), 2);
}

#[test]
fn the_round_cap_is_the_fixed_one() {
    assert_eq!(DEOBFUSCATE_MAX_ROUNDS, 64);
}

#[test]
fn the_report_carries_one_entry_per_pass_in_stage_order() {
    let (_code, report) = run_report(OBFUSCATED);
    let names: Vec<&str> = report.per_pass.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, DEOBFUSCATE_ORDER);
}

#[test]
fn the_decoding_vector_keeps_its_six_entries_and_the_cleanup_group_reports_separately() {
    let (_code, report) = run_report(OBFUSCATED);
    let names: Vec<&str> = report.per_pass.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, DEOBFUSCATE_ORDER);
    let cleanup: Vec<&str> = report
        .fixpoint_per_pass
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(cleanup, CLEANUP_FIXPOINT_ORDER);
}

#[test]
fn the_rounds_repeat_until_the_program_settles() {
    let (code, first) = run_report(OBFUSCATED);
    assert_eq!(code.trim(), r#"console["log"]("Hello World!");"#);
    assert!(first.total_changes > 0);
    assert!(first.converged());

    let (again, second) = run_report(&code);
    assert_eq!((second.iterations, second.total_changes), (1, 0));
    assert_eq!(again, code);
}

#[test]
fn a_program_without_a_string_array_is_untouched_and_noted() {
    let source = "const obj = { version: '2.0.0' };\nconsole.log(obj.version);";
    let (code, notes) = common::run_passes(source);
    assert!(code.contains("version: \"2.0.0\""), "{code}");
    assert!(code.contains("obj.version"), "{code}");
    assert!(
        notes.iter().any(|note| note == NO_STRING_ARRAY_NOTE),
        "{notes:?}"
    );
}

#[test]
fn the_string_array_stage_iterates_until_the_aliases_settle() {
    let (code, _notes) =
        common::run_passes("var a = ['log', 'x'];\nvar b = a;\nconsole[b[0]](b[1]);");
    assert_eq!(code.trim(), r#"console["log"]("x");"#);
}

const ONE_WRAPPER_MANY_SITES: &str = "\
function a() { var h = ['alpha', 'beta']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
function i(c, d) { return b(c, d); }
console.log(i(0x0, 0x1), i(0x1, 0x1));";

#[test]
fn each_site_of_one_inlined_wrapper_decodes_to_its_own_string() {
    let (code, report) = run_report(ONE_WRAPPER_MANY_SITES);
    assert_eq!(code.trim(), r#"console.log("alpha", "beta");"#);
    assert!(report.converged(), "{:?}", report.stop);
}

const ROTATED_THROUGH_SWAPPED_WRAPPERS: &str = r#"(function (c, d) {
  var e = c();
  function i(c, d) { return b(c - 0x5, d); }
  function h(c, d) { return b(d - 0x5, c); }
  while (!![]) {
    try {
      var f = parseInt(h(0x0, 0x15)) / 0x1 + parseInt(i(0x16, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a, 0x16));
function a() {
  var l = ['log', 'Hello\x20World!', '11alpha', '22beta'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b(c, d);
}
function hi() {
  console[j(0x14, 0x0)](j(0x15, 0x0));
  function j(c, d) { return b(c - 0x2, d); }
}
hi();"#;

#[test]
fn a_rotation_whose_checksum_runs_through_swapped_wrappers_resolves() {
    let (code, report) = run_report(ROTATED_THROUGH_SWAPPED_WRAPPERS);
    assert_eq!(
        code.trim(),
        "function hi() {\n  console[\"log\"](\"Hello World!\");\n}\nhi();"
    );
    assert!(report.converged(), "{:?}", report.stop);
}

const ROTATOR_WITH_A_NON_LITERAL_CHECKSUM_ARGUMENT: &str = r#"(function (c, d) {
  var e = c();
  function i(c, d) { return b(c - 0x5, d); }
  while (!![]) {
    try {
      var f = parseInt(i(0x15, d)) / 0x1 + parseInt(i(0x16, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a, 0x16));
function a() {
  var l = ['log', 'Hello\x20World!', '11alpha', '22beta'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b(c, d);
}
function hi() {
  console[b(0x12, 0x0)](b(0x13, 0x0));
}
hi();"#;

#[test]
fn a_machinery_internal_call_site_neither_decodes_nor_keeps_the_machinery() {
    let (code, report) = run_report(ROTATOR_WITH_A_NON_LITERAL_CHECKSUM_ARGUMENT);
    assert_eq!(
        code.trim(),
        "function hi() {\n  console[\"log\"](\"Hello World!\");\n}\nhi();"
    );
    assert!(!code.contains("while"), "{code}");
    assert!(!code.contains("__DECODE_"), "{code}");
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(report.converged(), "{:?}", report.stop);
}

const AN_OUTSIDE_SITE_THAT_CANNOT_DECODE: &str = r#"(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(0x10, 0x0)) / 0x1 + parseInt(b(0x11, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a, 0x16));
function a() {
  var l = ['log', 'Hello\x20World!', '11alpha', '22beta'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b(c, d);
}
function hi(n) {
  console[b(0x12, 0x0)](b(n, 0x0));
}
hi(0x13);"#;

const THE_KEPT_ROTATOR: &str = r#"(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(16, 0)) / 1 + parseInt(b(17, 0)) / 2;
      if (f === d) {
        break;
      } else {
        e["push"](e["shift"]());
      }
    } catch (g) {
      e["push"](e["shift"]());
    }
  }
})(a, 22);
"#;

const THE_KEPT_DECODER: &str = r#"function b(c, d) {
  var e = a();
  b = function (f, g) {
    f = f - 16;
    var h = e[f];
    return h;
  };
  return b(c, d);
}
"#;

#[test]
fn a_kept_machinery_carries_no_decoded_literal_in_its_own_body() {
    let (code, report) = run_report(AN_OUTSIDE_SITE_THAT_CANNOT_DECODE);
    let (rotator, rest) = code
        .split_once("function a(")
        .unwrap_or_else(|| panic!("the machinery was removed: {code}"));
    assert_eq!(rotator, THE_KEPT_ROTATOR);
    assert!(rest.contains(THE_KEPT_DECODER), "{rest}");
    assert!(rest.contains("console[\"log\"](b(n, 0))"), "{rest}");
    assert!(
        report.notes.iter().any(|note| note.message
            == "decoder machinery kept: 1 reference(s) to the string array or its decoders did \
                not decode"),
        "{:?}",
        report.notes
    );
    assert!(report.converged(), "{:?}", report.stop);
}

const A_WRAPPED_REFERENCE_WITH_A_LATER_ASSIGNMENT: &str = r#"function a() {
  var l = ['log', 'ZZZ'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x0; var h = e[f]; return h; }, b(c, d);
}
function w(c, d) { return b(c, d); }
var x;
x = 0x0;
console[w(x, 0x0)]('hi');
x = 0x1;"#;

#[test]
fn the_assignment_that_precedes_a_wrapped_reference_is_the_one_inlined() {
    let (code, report) = run_report(A_WRAPPED_REFERENCE_WITH_A_LATER_ASSIGNMENT);
    assert_eq!(
        code.trim(),
        "var x;\nx = 0;\nconsole[\"log\"](\"hi\");\nx = 1;"
    );
    assert!(report.converged(), "{:?}", report.stop);
}

const A_WRAPPER_CALL_INSIDE_ANOTHER_WRAPPER_ARGUMENT: &str = r#"function a() {
  var l = ['log', 'Hello\x20World!'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x0; var h = e[f]; return h; }, b(c, d);
}
function w1(c, d) { return b(c, d); }
function w2(c, d) { return b(c - 0x1, d); }
console[w1(0x0, w2(0x2, 0x0))]('hi');"#;

#[test]
fn a_wrapper_call_nested_in_another_wrappers_argument_is_fully_resolved() {
    let (code, report) = run_report(A_WRAPPER_CALL_INSIDE_ANOTHER_WRAPPER_ARGUMENT);
    assert_eq!(code.trim(), "console[\"log\"](\"hi\");");
    assert!(report.converged(), "{:?}", report.stop);
}

const A_KEPT_MACHINERY_WITH_A_WRAPPER_IN_THE_ROTATOR: &str = r#"(function (c, d) {
  var e = c();
  function i(c, d) { return b(c - 0x5, d); }
  while (!![]) {
    try {
      var f = parseInt(i(0x15, 0x0)) / 0x1 + parseInt(i(0x16, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a, 0x16));
function a() {
  var l = ['log', 'Hello\x20World!', '11alpha', '22beta'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b(c, d);
}
function hi(n) {
  console[b(0x12, 0x0)](b(n, 0x0));
}
hi(0x13);"#;

const THE_KEPT_ROTATOR_WITH_ITS_WRAPPER: &str = r#"(function (c, d) {
  var e = c();
  function i(c, d) {
    return b(c - 5, d);
  }
  while (!![]) {
    try {
      var f = parseInt(i(21, 0)) / 1 + parseInt(i(22, 0)) / 2;
      if (f === d) {
        break;
      } else {
        e["push"](e["shift"]());
      }
    } catch (g) {
      e["push"](e["shift"]());
    }
  }
})(a, 22);
"#;

#[test]
fn a_kept_machinery_keeps_the_wrapper_declared_in_its_own_rotator() {
    let (code, report) = run_report(A_KEPT_MACHINERY_WITH_A_WRAPPER_IN_THE_ROTATOR);
    let (rotator, rest) = code
        .split_once("function a(")
        .unwrap_or_else(|| panic!("the machinery was removed: {code}"));
    assert_eq!(rotator, THE_KEPT_ROTATOR_WITH_ITS_WRAPPER);
    assert!(rest.contains("console[\"log\"](b(n, 0))"), "{rest}");
    assert!(report.converged(), "{:?}", report.stop);
}

const A_KEPT_MACHINERY_WITH_AN_ALIAS_IN_THE_ROTATOR: &str = r#"(function (c, d) {
  var e = c();
  var i = b;
  while (!![]) {
    try {
      var f = parseInt(i(0x10, 0x0)) / 0x1 + parseInt(i(0x11, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a, 0x16));
function a() {
  var l = ['log', 'Hello\x20World!', '11alpha', '22beta'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b(c, d);
}
function hi(n) {
  console[b(0x12, 0x0)](b(n, 0x0));
}
hi(0x13);"#;

const THE_KEPT_ROTATOR_WITH_ITS_ALIAS: &str = r#"(function (c, d) {
  var e = c();
  var i = b;
  while (!![]) {
    try {
      var f = parseInt(i(16, 0)) / 1 + parseInt(i(17, 0)) / 2;
      if (f === d) {
        break;
      } else {
        e["push"](e["shift"]());
      }
    } catch (g) {
      e["push"](e["shift"]());
    }
  }
})(a, 22);
"#;

#[test]
fn a_kept_machinery_keeps_the_alias_declared_in_its_own_rotator() {
    let (code, report) = run_report(A_KEPT_MACHINERY_WITH_AN_ALIAS_IN_THE_ROTATOR);
    let (rotator, rest) = code
        .split_once("function a(")
        .unwrap_or_else(|| panic!("the machinery was removed: {code}"));
    assert_eq!(rotator, THE_KEPT_ROTATOR_WITH_ITS_ALIAS);
    assert!(rest.contains("console[\"log\"](b(n, 0))"), "{rest}");
    assert!(report.converged(), "{:?}", report.stop);
}

const A_MACHINERY_INTERNAL_ALIAS_WITH_NOTHING_OUTSTANDING: &str = r#"(function (c, d) {
  var e = c();
  var i = b;
  while (!![]) {
    try {
      var f = parseInt(i(0x10, 0x0)) / 0x1 + parseInt(i(0x11, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a, 0x16));
function a() {
  var l = ['log', 'Hello\x20World!', '11alpha', '22beta'];
  a = function () { return l; };
  return a();
}
function b(c, d) {
  var e = a();
  return b = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b(c, d);
}
function hi() {
  console[b(0x12, 0x0)](b(0x13, 0x0));
}
hi();"#;

#[test]
fn a_machinery_internal_alias_neither_decodes_nor_keeps_the_machinery() {
    let (code, report) = run_report(A_MACHINERY_INTERNAL_ALIAS_WITH_NOTHING_OUTSTANDING);
    assert_eq!(
        code.trim(),
        "function hi() {\n  console[\"log\"](\"Hello World!\");\n}\nhi();"
    );
    assert!(!code.contains("while"), "{code}");
    assert!(!code.contains("__DECODE_"), "{code}");
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(report.converged(), "{:?}", report.stop);
}

const A_DEEP_ALIAS_CHAIN: &str = "\
var a0 = ['log'];
var a1 = a0;
var a2 = a1;
var a3 = a2;
var a4 = a3;
var a5 = a4;
console[a5[0]]();";

#[test]
fn a_deep_alias_chain_converges_without_a_cap_note() {
    let (code, report) = run_report(A_DEEP_ALIAS_CHAIN);
    assert_eq!(code.trim(), "console[\"log\"]();", "{code}");
    assert!(report.converged(), "{:?}", report.stop);
    assert!(
        !report.notes.iter().any(|note| note.message.contains("cap")),
        "{:?}",
        report.notes
    );
}

#[test]
fn a_direct_eval_is_reported_once() {
    let (_code, report) = run_report("eval('1');\nconsole.log(1);");
    let hazards: Vec<&str> = report
        .notes
        .iter()
        .filter(|note| note.message.starts_with("direct eval"))
        .map(|note| note.message.as_str())
        .collect();
    assert_eq!(hazards.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_with_statement_is_reported_once() {
    let (_code, report) = run_report("with (o) { console.log(x); }");
    let hazards: Vec<&str> = report
        .notes
        .iter()
        .filter(|note| note.message.starts_with("with statement"))
        .map(|note| note.message.as_str())
        .collect();
    assert_eq!(hazards.len(), 1, "{:?}", report.notes);
}

#[test]
fn the_skipped_note_names_the_stage_once() {
    let (_code, report) = run_report("console.log(1);");
    let skipped: Vec<&untwist_core::pass::Note> = report
        .notes
        .iter()
        .filter(|note| note.message == NO_STRING_ARRAY_NOTE)
        .collect();
    assert_eq!(skipped.len(), 1, "{:?}", report.notes);
    assert_eq!(skipped[0].pass, "deobfuscate");
}

const AN_ARRAY_THAT_ONLY_DECODING_MAKES_LITERAL: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
var colours = ['#fff', b(0x1)];
console[b(0x0)](colours[0], colours[1]);";

#[test]
fn conservative_decides_simple_array_eligibility_before_any_decoding() {
    let (code, report) = run_report(AN_ARRAY_THAT_ONLY_DECODING_MAKES_LITERAL);
    assert_eq!(
        code.trim(),
        "var colours = [\"#fff\", \"Hello World!\"];\nconsole[\"log\"](colours[0], colours[1]);",
        "the reference deobfuscator looks for simple string arrays once, before the decoders run, so an array \
         whose elements only become literals after decoding is never inlined"
    );
    assert!(report.converged(), "{:?}", report.stop);
}

#[test]
fn extended_inlines_the_array_the_decoding_made_literal() {
    let (code, report) = common::run_report_extended(AN_ARRAY_THAT_ONLY_DECODING_MAKES_LITERAL);
    assert_eq!(code.trim(), "console[\"log\"](\"#fff\", \"Hello World!\");");
    assert!(report.converged(), "{:?}", report.stop);
}
