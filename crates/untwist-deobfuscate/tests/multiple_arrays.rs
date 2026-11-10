mod common;

use untwist_core::pass::{Note, NoteKind};

const TWO_UNITS: &str = "\
function a1() { var h = ['log', 'first']; a1 = function () { return h; }; return a1(); }
function b1(c, d) { var e = a1(); b1 = function (f, g) { f = f - 0x0; return e[f]; }; return b1(c, d); }
function a2() { var h = ['warn', 'second']; a2 = function () { return h; }; return a2(); }
function b2(c, d) { var e = a2(); b2 = function (f, g) { f = f - 0x0; return e[f]; }; return b2(c, d); }
console[b1(0x0)](b1(0x1));
console[b2(0x0)](b2(0x1));";

#[test]
fn two_string_arrays_in_one_file_both_decode_in_one_round() {
    let (code, report) = common::run_report(TWO_UNITS);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("console[\"warn\"](\"second\")"), "{code}");
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__STRING_ARRAY_1__"), "{code}");
    assert_eq!(report.iterations, 2, "{:?}", report.per_pass);
    assert_eq!(refusals(&report.notes), Vec::<String>::new());
}

#[test]
fn a_failing_unit_keeps_only_its_own_machinery() {
    let source = TWO_UNITS.replace(
        "console[b2(0x0)](b2(0x1));",
        "console[b2(0x0)](b2(unknownGlobal));",
    );
    let (code, report) = common::run_converged(&source);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("function a2("), "{code}");
    assert!(!code.contains("__STRING_ARRAY_1__"), "{code}");
    assert!(!code.contains("function __STRING_ARRAY__("), "{code}");
    assert_eq!(
        refusals(&report.notes),
        vec![
            "decoder machinery kept: 1 reference(s) to the string array or its decoders did not \
             decode",
            untwist_deobfuscate::notes::MACHINERY_NAME_OBSERVED,
        ]
    );
}

const A_SECOND_UNIT_WITH_A_ROTATOR: &str = r#"function a1() { var h = ['log', 'first']; a1 = function () { return h; }; return a1(); }
function b1(c, d) { var e = a1(); b1 = function (f, g) { f = f - 0x0; return e[f]; }; return b1(c, d); }
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b2(0x10, 0x0)) / 0x1 + parseInt(b2(0x11, 0x0)) / 0x2;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a2, 0x16));
console[b1(0x0)](b1(0x1));
function a2() {
  var l = ['warn', 'second', '11alpha', '22beta'];
  a2 = function () { return l; };
  return a2();
}
function b2(c, d) {
  var e = a2();
  return b2 = function (f, g) { f = f - 0x10; var h = e[f]; return h; }, b2(c, d);
}
function hi(n) {
  console[b2(0x12, 0x0)](b2(n, 0x0));
}
hi(0x13);"#;

const THE_KEPT_SECOND_ROTATOR: &str = r#"(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b2(16, 0)) / 1 + parseInt(b2(17, 0)) / 2;
      if (f === d) {
        break;
      } else {
        e["push"](e["shift"]());
      }
    } catch (g) {
      e["push"](e["shift"]());
    }
  }
})(a2, 22);
"#;

#[test]
fn a_second_units_rotator_is_machinery_and_keeps_its_own_body() {
    let (code, _) = common::run_passes(A_SECOND_UNIT_WITH_A_ROTATOR);
    let (before, rest) = code
        .split_once("function a2(")
        .unwrap_or_else(|| panic!("the second unit's machinery was removed: {code}"));
    assert_eq!(
        before,
        format!("{THE_KEPT_SECOND_ROTATOR}console[\"log\"](\"first\");\n")
    );
    assert!(rest.contains("console[\"warn\"](b2(n, 0))"), "{rest}");
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
}

fn with_a_print_before_the_second_rotator(print: &str) -> String {
    A_SECOND_UNIT_WITH_A_ROTATOR
        .replace("console[b1(0x0)](b1(0x1));\n", "")
        .replace(
            "(function (c, d) {\n  var e = c();",
            &format!("{print}\n(function (c, d) {{\n  var e = c();"),
        )
}

fn refuses_the_second_unit_before_its_rotator(source: &str) {
    let (code, notes) = common::run_passes(source);
    assert!(code.contains("console[b2(18, 0)](b2(n, 0))"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR),
        "{notes:?}"
    );
}

#[test]
fn a_proven_first_units_print_before_the_second_units_rotator_is_inert() {
    let source = with_a_print_before_the_second_rotator("console[b1(0x0)](b1(0x1));");
    let (code, notes) = common::run_passes(&source);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("console[\"warn\"](b2(n, 0))"), "{code}");
    assert!(
        !notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR),
        "{notes:?}"
    );
}

#[test]
fn a_bare_call_of_the_first_units_decoder_before_the_second_units_rotator_is_inert() {
    let source = with_a_print_before_the_second_rotator("b1(0x1);\nconsole[b1(0x0)](b1(0x1));");
    let (code, notes) = common::run_passes(&source);
    assert!(code.contains("console[\"warn\"](b2(n, 0))"), "{code}");
    assert!(
        !notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR),
        "{notes:?}"
    );
}

#[test]
fn a_first_units_call_with_a_non_literal_argument_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_print_before_the_second_rotator(
        "var m = [0x1];\nconsole[b1(0x0)](b1(m[0x0]));",
    ));
}

#[test]
fn the_second_units_own_call_in_the_print_keeps_it_refused() {
    let source = format!(
        "{}\nexport {{}};",
        with_a_print_before_the_second_rotator("console[b1(0x0)](b2(0x12, 0x0));")
    );
    let (code, notes) = common::run_passes(&source);
    assert!(code.contains("console[\"log\"](b2(18, 0))"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR),
        "{notes:?}"
    );
}

#[test]
fn a_user_function_around_the_first_units_call_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_print_before_the_second_rotator(
        "function show(s) { console.log(s); }\nshow(b1(0x1));",
    ));
}

#[test]
fn a_shadowed_console_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_print_before_the_second_rotator(
        "function console() {}\nconsole[b1(0x0)](b1(0x1));",
    ));
}

#[test]
fn an_optional_call_on_console_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_print_before_the_second_rotator(
        "console?.[b1(0x0)](b1(0x1));",
    ));
}

#[test]
fn a_first_unit_whose_decoder_reaches_the_global_object_keeps_the_second_unit_refused() {
    let source = with_a_print_before_the_second_rotator("console[b1(0x0)](b1(0x1));").replace(
        "f = f - 0x0; return e[f]; }; return b1(c, d);",
        "f = f - 0x0; this.seen = f; return e[f]; }; return b1(c, d);",
    );
    refuses_the_second_unit_before_its_rotator(&source);
}

#[test]
fn a_first_unit_whose_decoder_takes_the_global_object_from_a_called_function_keeps_the_second_unit_refused(
) {
    let source = with_a_print_before_the_second_rotator("console[b1(0x0)](b1(0x1));").replace(
        "f = f - 0x0; return e[f]; }; return b1(c, d);",
        "f = f - 0x0; (function () { return this; })().seen = f; return e[f]; }; return b1(c, d);",
    );
    refuses_the_second_unit_before_its_rotator(&source);
}

#[test]
fn a_first_unit_whose_decoder_names_function_keeps_the_second_unit_refused() {
    let source = with_a_print_before_the_second_rotator("console[b1(0x0)](b1(0x1));").replace(
        "f = f - 0x0; return e[f]; }; return b1(c, d);",
        "f = f - 0x0; Function('0'); return e[f]; }; return b1(c, d);",
    );
    refuses_the_second_unit_before_its_rotator(&source);
}

fn with_a_first_decoder(print: &str, body: &str) -> String {
    with_a_print_before_the_second_rotator(print).replace(
        "b1 = function (f, g) { f = f - 0x0; return e[f]; };",
        &format!("b1 = function (f, g) {{ {body} }};"),
    )
}

const THE_PRINT: &str = "console[b1(0x0)](b1(0x1));";

#[test]
fn a_first_decoder_reaching_function_through_a_parameter_key_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        "console[b1(0x0)](b1(0x1, 'constructor'));",
        "f = f - 0x0; if (g) { var F = e[g][g]; F('0')(); } return e[f];",
    ));
}

#[test]
fn a_first_decoder_reaching_function_through_a_string_base_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        "console[b1(0x0)](b1(0x1, 'constructor'));",
        "f = f - 0x0; if (g) { var F = ''[g][g]; F('0')(); } return e[f];",
    ));
}

#[test]
fn a_first_decoder_reaching_function_through_a_local_key_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        THE_PRINT,
        "f = f - 0x0; if (f === 0x1) { var k = 'constr' + 'uctor'; var F = e[k][k]; F('0')(); } return e[f];",
    ));
}

#[test]
fn a_first_decoder_reaching_define_property_through_a_local_key_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        THE_PRINT,
        "f = f - 0x0; if (f === 0x1) { var k = 'constr' + 'uctor'; var dp = ({})[k]['define' + 'Property']; dp(Array.prototype, 'shift', { value: function () { return 'log'; } }); } return e[f];",
    ));
}

#[test]
fn a_first_decoder_reaching_assign_through_a_local_key_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        THE_PRINT,
        "f = f - 0x0; if (f === 0x1) { var k = 'constr' + 'uctor'; var as = ({})[k]['ass' + 'ign']; as(Array.prototype, { shift: function () { return 'log'; } }); } return e[f];",
    ));
}

#[test]
fn a_first_decoder_parameter_overwritten_with_plus_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        THE_PRINT,
        "f = f + -0x0; return e[f];",
    ));
}

#[test]
fn a_first_decoder_parameter_reassigned_to_a_string_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        "console[b1(0x0)](b1(0x1, 0x1));",
        "f = f - 0x0; if (g) { f = 'constructor'; } return e[f];",
    ));
}

#[test]
fn a_first_decoder_parameter_compounded_with_a_string_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        "console[b1(0x0)](b1(0x1, 0x1));",
        "f = f - 0x0; if (g) { f += 'x'; } return e[f];",
    ));
}

#[test]
fn a_first_decoder_reading_before_its_parameter_is_overwritten_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        THE_PRINT,
        "var h = e[f]; f = f - 0x0; return h;",
    ));
}

#[test]
fn a_first_decoder_reading_its_parameter_in_a_default_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(
        &with_a_print_before_the_second_rotator(THE_PRINT).replace(
            "b1 = function (f, g) { f = f - 0x0;",
            "b1 = function (f, g = e[f]) { f = f - 0x0;",
        ),
    );
}

#[test]
fn a_first_decoder_with_only_numeric_writes_to_its_parameter_leaves_the_second_unit_inert() {
    let source = with_a_first_decoder(
        THE_PRINT,
        "f = f * 0x1; if (g) { f = f - 0x1; } return e[f];",
    );
    let (code, notes) = common::run_passes(&source);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("console[\"warn\"](b2(n, 0))"), "{code}");
    assert!(
        !notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR),
        "{notes:?}"
    );
}

#[test]
fn a_refused_first_unit_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&format!(
        "{}\nconsole.log(typeof b1);",
        with_a_print_before_the_second_rotator(THE_PRINT)
    ));
}

#[test]
fn a_first_unit_the_evaluator_bails_on_keeps_the_second_unit_refused() {
    refuses_the_second_unit_before_its_rotator(&with_a_first_decoder(
        "console[b1(0x0)](b1(0x1, 0x1));",
        "f = f - 0x0; if (g) { Math.random(); } return e[f];",
    ));
}

const TWO_ROTATED_UNITS: &str = r#"(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b1(0x2)) / 0x1;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a1, 0x7));
function a1() { var h = ['7x', 'log', 'first']; a1 = function () { return h; }; return a1(); }
function b1(c, d) { var e = a1(); b1 = function (f, g) { f = f - 0x0; return e[f]; }; return b1(c, d); }
console[b1(0x0)](b1(0x1));
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b2(0x10)) / 0x1;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) {
      e['push'](e['shift']());
    }
  }
}(a2, 0x9));
function a2() { var l = ['warn', '9y', 'second']; a2 = function () { return l; }; return a2(); }
function b2(c, d) { var e = a2(); b2 = function (f, g) { f = f - 0x10; return e[f]; }; return b2(c, d); }
console[b2(0x12)](b2(0x11));"#;

#[test]
fn a_sealed_first_rotator_before_the_second_units_rotator_is_inert() {
    let (code, report) = common::run_converged(TWO_ROTATED_UNITS);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("console[\"warn\"](\"second\")"), "{code}");
    assert!(
        !code.contains("function a1(") && !code.contains("function a2("),
        "{code}"
    );
    assert_eq!(refusals(&report.notes), Vec::<String>::new());
}

#[test]
fn an_unsealed_first_rotator_keeps_both_units_refused() {
    let source = TWO_ROTATED_UNITS.replacen(
        "var e = c();\n  while",
        "var e = c(); globalThis.seen = e;\n  while",
        1,
    );
    let (code, notes) = common::run_passes(&source);
    assert!(code.contains("console[b1(0)](b1(1))"), "{code}");
    assert!(code.contains("console[b2(18)](b2(17))"), "{code}");
    for expected in [
        untwist_deobfuscate::notes::ROTATOR_NOT_SEALED,
        untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR,
    ] {
        assert!(notes.iter().any(|note| note == expected), "{notes:?}");
    }
}

const AN_UNATTRIBUTED_DECODER: &str = "\
function __DECODE_0__(x) { return 'ZZZ'; }
function a1() { var h = ['log', 'first']; a1 = function () { return h; }; return a1(); }
function b1(c, d) { var e = a1(); b1 = function (f, g) { f = f - 0x0; return e[f]; }; return b1(c, d); }
function a2() { var h = ['warn', 'second']; a2 = function () { return h; }; return a2(); }
function b2(c, d) { var e = a2(); b2 = function (f, g) { f = f - 0x0; return e[f]; }; return b2(c, d); }
console[b1(0x0)](b1(0x1));
console[b2(0x0)](b2(0x1));
console.log(__DECODE_0__(0x0));";

const A_CHAINED_SECOND_UNIT: &str = "\
function a1() { var h = ['0', 'log']; a1 = function () { return h; }; return a1(); }
function b1(c, d) { var e = a1(); b1 = function (f, g) { f = f - 0x0; return e[f]; }; return b1(c, d); }
function a2() { var h = ['warn', 'second']; a2 = function () { return h; }; return a2(); }
function b2(c, d) { var e = a2(); b2 = function (f, g) { f = f - 0x0; return e[f]; }; return b2(c, d); }
console[b2(b1(0x0))]('x');";

const AN_AMBIGUOUS_DECODER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'first']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __STRING_ARRAY_1__() { var l = ['warn', 'second']; __STRING_ARRAY_1__ = function () { return l; }; return __STRING_ARRAY_1__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); var k = __STRING_ARRAY_1__(); return k[c]; }
console[__DECODE_0__(0x0)](__DECODE_0__(0x1));";

fn behaviour(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/behaviour/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn refusals(notes: &[Note]) -> Vec<String> {
    notes
        .iter()
        .filter(|note| note.kind == NoteKind::Refusal)
        .map(|note| note.message.clone())
        .collect()
}

#[test]
fn an_unattributed_decoder_keeps_its_call_sites_and_is_named_once() {
    let (code, report) = common::run_converged(AN_UNATTRIBUTED_DECODER);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("console[\"warn\"](\"second\")"), "{code}");
    assert!(code.contains("console.log(__DECODE_0__(0))"), "{code}");
    assert!(code.contains("return \"ZZZ\""), "{code}");
    assert_eq!(
        refusals(&report.notes),
        vec![
            "__DECODE_0__ does not reference exactly one stamped string array, so it belongs to no \
             unit and its call sites were left alone"
        ]
    );
}

#[test]
fn a_surviving_decoder_whose_number_has_a_gap_still_decodes_in_a_later_round() {
    let (code, report) = common::run_report(A_CHAINED_SECOND_UNIT);
    assert_eq!(code.trim(), "console[\"warn\"](\"x\");");
    assert!(!code.contains("__DECODE_"), "{code}");
    assert!(!code.contains("__STRING_ARRAY"), "{code}");
    assert_eq!(
        refusals(&report.notes),
        Vec::<String>::new(),
        "round 1 keeps the chained unit honestly; no round may report an incomplete machinery"
    );
    assert_eq!(report.iterations, 3, "{:?}", report.per_pass);
}

#[test]
fn a_decoder_that_names_two_units_is_unattributed_and_changes_nothing() {
    let (code, report) = common::run_converged(AN_AMBIGUOUS_DECODER);
    assert!(
        code.contains("console[__DECODE_0__(0)](__DECODE_0__(1))"),
        "{code}"
    );
    assert!(code.contains("function __STRING_ARRAY__("), "{code}");
    assert!(code.contains("function __STRING_ARRAY_1__("), "{code}");
    assert_eq!(
        refusals(&report.notes),
        vec![
            "__DECODE_0__ does not reference exactly one stamped string array, so it belongs to no \
             unit and its call sites were left alone"
        ]
    );
}

const A_CALL_FROM_ANOTHER_UNITS_DECODER: &str = "\
function a1() { var h = ['log', 'first']; a1 = function () { return h; }; return a1(); }
function b1(c) { var e = a1(); return e[c]; }
function a2() { var h = ['warn', 'second']; a2 = function () { return h; }; return a2(); }
function b2(c) { var e = a2(); var z = b1(c); return e[c]; }
console[b1(0x0)](b1(0x1));
console[b2(0x0)](b2(0x1));";

#[test]
fn a_call_from_another_units_decoder_counts_as_a_call_outside_the_machinery() {
    let source = format!("{A_CALL_FROM_ANOTHER_UNITS_DECODER}\nexport {{}};");
    let (code, report) = common::run_converged(&source);
    assert!(code.contains("console[\"log\"](\"first\")"), "{code}");
    assert!(code.contains("console[b2(0)](b2(1))"), "{code}");
    let notes = refusals(&report.notes);
    assert!(
        notes.iter().all(|note| !note.contains("used other than")),
        "{notes:?}"
    );
}

#[test]
fn a_memo_decoder_called_from_another_units_decoder_is_refused() {
    let source = behaviour("other-units-decoder-feeds-memo.js");
    let (code, report) = common::run_converged(&source);
    assert!(
        code.contains("b(0, 'x')") || code.contains("b(0, \"x\")"),
        "{code}"
    );
    assert!(
        refusals(&report.notes).iter().any(|note| note
            .contains("a stateful decoder is referenced outside the calls untwist proves")),
        "{:?}",
        refusals(&report.notes)
    );
}

#[test]
fn a_decoder_named_as_a_value_inside_another_units_decoder_is_refused() {
    let source = behaviour("other-units-decoder-writes-memo.js");
    let (code, report) = common::run_converged(&source);
    assert!(code.contains("console.log(b(1))"), "{code}");
    assert!(
        refusals(&report.notes)
            .iter()
            .any(|note| note.contains("is used other than as the callee of a call")),
        "{:?}",
        refusals(&report.notes)
    );
}

#[test]
fn a_string_another_unit_decodes_to_this_units_decoder_name_refuses_this_unit() {
    for fixture in [
        "decoded-string-names-other-units-decoder.js",
        "decoded-string-names-other-units-array.js",
    ] {
        let source = behaviour(fixture);
        let (code, report) = common::run_converged(&source);
        let refused = refusals(&report.notes);
        assert!(
            code.contains("console.log(q(0), q(1), b(1))"),
            "{fixture}: {code}"
        );
        for note in [
            untwist_deobfuscate::notes::DECODED_NAME,
            untwist_deobfuscate::notes::BUILTIN_PATCHED,
        ] {
            assert!(
                refused.iter().any(|seen| {
                    seen.starts_with(note)
                        || seen == untwist_deobfuscate::notes::STRING_ARRAY_GLOBAL_REACH
                }),
                "{fixture}: {refused:?}"
            );
        }
    }
}
