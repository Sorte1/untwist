use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{pretty_codegen, print, PrintStyle};
use untwist_core::reflow::{
    misplace_an_edit_for_tests, print_placing, reflow, reflow_recorded, reflow_tallied,
    GuardRecord, GuardTally,
};

fn placed(source: &str) -> (bool, String, GuardRecord) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let printed = print_placing(&alloc, &mut parsed.program, None);
    let placed = printed.is_placed();
    let (code, record, _) = printed.finish(None);
    (placed, code, record)
}

fn placed_as_reparsed(source: &str) -> (String, GuardRecord) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let reparsed = reflow_recorded(pretty_codegen(&parsed.program));
    let (_, code, record) = placed(source);
    assert_eq!(
        (code, record),
        reparsed,
        "the placed reflow differs: {source}"
    );
    reparsed
}

fn pretty(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let printed = print(&parsed.program, PrintStyle::Pretty);
    assert_eq!(placed_as_reparsed(source).0, printed);
    printed
}

fn guard_tally(compacted: u32, kept: u32) -> GuardTally {
    GuardTally { compacted, kept }
}

fn tallied(source: &str) -> (String, GuardTally) {
    let (code, record) = placed_as_reparsed(source);
    (code, record.tally())
}

fn assert_compacted(source: &str, target: &str, compact: &str) {
    assert_eq!(source.matches(target).count(), 1, "{source}");
    let expected = source.replace(target, compact);
    let out = pretty(source);
    assert_eq!(out, expected);
    assert_eq!(reflow(out.clone()), out);
    assert_eq!(pretty(&out), out);
    assert_eq!(tallied(source).1, guard_tally(1, 0));
}

fn assert_unchanged(source: &str) {
    assert_eq!(pretty(source), source);
    assert_eq!(tallied(source).1, GuardTally::default());
}

const LITERAL_APPLY_GUARD: &str = "var ctrl = function () {
  var f = !![];
  return function (c, fn) {
    var r = f ? function () {
      if (fn) {
        var x = fn.apply(c, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = ![];
    return r;
  };
}();";

const LITERAL_APPLY_GUARD_TARGET: &str = "function () {
      if (fn) {
        var x = fn.apply(c, arguments);
        fn = null;
        return x;
      }
    }";

#[test]
fn a_the_reference_template_prints_its_read_target_on_one_line() {
    assert_compacted(
        LITERAL_APPLY_GUARD,
        LITERAL_APPLY_GUARD_TARGET,
        "function () {if (fn) {var x = fn.apply(c, arguments);fn = null;return x;}}",
    );
}

#[test]
fn an_encoded_key_behind_a_leading_alias_is_matched() {
    let source = "var ctrl = function () {
  var f = true;
  return function (c, fn) {
    var r = f ? function () {
      var w = d;
      if (fn) {
        var x = fn[w(430)](c, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = false;
    return r;
  };
}();";
    let target = "function () {
      var w = d;
      if (fn) {
        var x = fn[w(430)](c, arguments);
        fn = null;
        return x;
      }
    }";
    assert_compacted(
        source,
        target,
        "function () {var w = d;if (fn) {var x = fn[w(430)](c, arguments);fn = null;return x;}}",
    );
}

#[test]
fn a_leading_wrapper_function_is_matched() {
    let source = "function greet() {
  var ctrl = function () {
    var f = !![];
    return function (c, fn) {
      var r = f ? function () {
        function w(a, b) {
          return d(a - -717, b);
        }
        if (fn) {
          var x = fn[w(-299, \"Ke8F\")](c, arguments);
          fn = null;
          return x;
        }
      } : function () {};
      f = ![];
      return r;
    };
  }();
  return ctrl;
}";
    let target = "function () {
        function w(a, b) {
          return d(a - -717, b);
        }
        if (fn) {
          var x = fn[w(-299, \"Ke8F\")](c, arguments);
          fn = null;
          return x;
        }
      }";
    assert_compacted(
        source,
        target,
        "function () {function w(a, b) {return d(a - -717, b);}if (fn) {var x = fn[w(-299, \"Ke8F\")](c, arguments);fn = null;return x;}}",
    );
}

#[test]
fn the_sequence_forms_are_matched() {
    let source = "var ctrl = function () {
  var f = !![];
  return function (c, fn) {
    var r = f ? function () {
      var w = d;
      if (fn) {
        var x = fn[w(430)](c, arguments);
        return fn = null, x;
      }
    } : function () {};
    return f = ![], r;
  };
}();";
    let target = "function () {
      var w = d;
      if (fn) {
        var x = fn[w(430)](c, arguments);
        return fn = null, x;
      }
    }";
    assert_compacted(
        source,
        target,
        "function () {var w = d;if (fn) {var x = fn[w(430)](c, arguments);return fn = null, x;}}",
    );
}

const GUARD_STATEMENTS: &str = "var State = function (p) {
  this.a = p;
  this.b = [1, 0, 0];
  this.c = function () {
    return \"newState\";
  };
  this.d = \"\\\\w+ *\\\\(\\\\) *{\\\\w+ *\";
  this.e = \"['|\\\"].+['|\\\"];? *}\";
};";

const NEW_STATE_TARGET: &str = "function () {
    return \"newState\";
  }";

const NEW_STATE_COMPACT: &str = "function () {return \"newState\";}";

#[test]
fn a_the_new_state_constructor_prints_its_read_target_on_one_line() {
    assert_compacted(GUARD_STATEMENTS, NEW_STATE_TARGET, NEW_STATE_COMPACT);
}

#[test]
fn the_constructor_as_one_sequence_is_matched() {
    let source = "var State = function (p) {
  this[\"a\"] = p, this[\"b\"] = [1, 0, 0], this[\"c\"] = function () {
    return \"newState\";
  }, this[\"d\"] = \"\\\\w+ *\\\\(\\\\) *{\\\\w+ *\", this[\"e\"] = \"['|\\\"].+['|\\\"];? *}\";
};";
    assert_compacted(source, NEW_STATE_TARGET, NEW_STATE_COMPACT);
}

const ONE_DECLARATION_GUARD: &str = "var F = function (s) {
  var A = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\";
  var a = \"\", b = \"\", c = a + F;
  for (var q = 0, t, n, i = 0; n = s.charAt(i++); ~n && (t = q % 4 ? t * 64 + n : n, q++ % 4) ? a += c.charCodeAt(i + 10) - 10 !== 0 ? String.fromCharCode(255 & t >> (-2 * q & 6)) : q : 0) {
    n = A.indexOf(n);
  }
  return a;
};";

const ONE_DECLARATION_GUARD_COMPACT: &str = "function (s) {var A = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\";var a = \"\", b = \"\", c = a + F;for (var q = 0, t, n, i = 0; n = s.charAt(i++); ~n && (t = q % 4 ? t * 64 + n : n, q++ % 4) ? a += c.charCodeAt(i + 10) - 10 !== 0 ? String.fromCharCode(255 & t >> (-2 * q & 6)) : q : 0) {n = A.indexOf(n);}return a;}";

fn guard_target(source: &str) -> &str {
    let start = source.find("function (s)").unwrap_or_default();
    &source[start..source.len() - 1]
}

#[test]
fn a_the_self_reading_base64_helper_prints_on_one_line() {
    assert_compacted(
        ONE_DECLARATION_GUARD,
        guard_target(ONE_DECLARATION_GUARD),
        ONE_DECLARATION_GUARD_COMPACT,
    );
}

#[test]
fn the_helper_with_split_declarations_is_matched() {
    let source = "var F = function (s) {
  var A = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\";
  var a = \"\";
  var b = \"\";
  var c = a + F;
  for (var q = 0, t, n, i = 0; n = s[\"charAt\"](i++); ~n && (t = q % 4 ? t * 64 + n : n, q++ % 4) ? a += c[\"charCodeAt\"](i + 10) - 10 !== 0 ? String[\"fromCharCode\"](255 & t >> (-2 * q & 6)) : q : 0) {
    n = A[\"indexOf\"](n);
  }
  return a;
};";
    let compact = "function (s) {var A = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\";var a = \"\";var b = \"\";var c = a + F;for (var q = 0, t, n, i = 0; n = s[\"charAt\"](i++); ~n && (t = q % 4 ? t * 64 + n : n, q++ % 4) ? a += c[\"charCodeAt\"](i + 10) - 10 !== 0 ? String[\"fromCharCode\"](255 & t >> (-2 * q & 6)) : q : 0) {n = A[\"indexOf\"](n);}return a;}";
    assert_compacted(source, guard_target(source), compact);
}

#[test]
fn leading_constant_tables_are_matched() {
    let source = "var ctrl = function () {
  var k = {
    a: \"0x377\",
    b: 3
  };
  var f = true;
  return function (c, fn) {
    var r = f ? function () {
      var j = { a: \"0x37a\" };
      function w(p, q) {
        return d(q - -j.a, p);
      }
      if (fn) {
        var x = fn[w(-k.a, -k.b)](c, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = false;
    return r;
  };
}();";
    let target = "function () {
      var j = { a: \"0x37a\" };
      function w(p, q) {
        return d(q - -j.a, p);
      }
      if (fn) {
        var x = fn[w(-k.a, -k.b)](c, arguments);
        fn = null;
        return x;
      }
    }";
    assert_compacted(
        source,
        target,
        "function () {var j = { a: \"0x37a\" };function w(p, q) {return d(q - -j.a, p);}if (fn) {var x = fn[w(-k.a, -k.b)](c, arguments);fn = null;return x;}}",
    );
}

#[test]
fn counters_written_as_arithmetic_are_matched() {
    let source = GUARD_STATEMENTS.replace(
        "[1, 0, 0]",
        "[-1385 * -1 + -7397 + 6013, -3482 + 6593 + -51 * 61, 1 * -9127 + 2712 + 6415]",
    );
    assert_compacted(&source, NEW_STATE_TARGET, NEW_STATE_COMPACT);
}

#[test]
fn an_offset_written_as_arithmetic_is_matched() {
    let source = ONE_DECLARATION_GUARD.replace(
        "c.charCodeAt(i + 10) - 10 !== 0",
        "c.charCodeAt(i + (-8737 + -7673 + 16420)) - (-38 * 232 + 35 * -58 + 10856) !== -3613 + 3613",
    );
    let compact = ONE_DECLARATION_GUARD_COMPACT.replace(
        "c.charCodeAt(i + 10) - 10 !== 0",
        "c.charCodeAt(i + (-8737 + -7673 + 16420)) - (-38 * 232 + 35 * -58 + 10856) !== -3613 + 3613",
    );
    assert_compacted(&source, guard_target(&source), &compact);
}

#[test]
fn a_leading_table_that_runs_code_is_left_as_printed() {
    assert_unchanged(&LITERAL_APPLY_GUARD.replace(
        "  var f = !![];\n",
        "  var k = { a: g() };\n  var f = !![];\n",
    ));
}

#[test]
fn counters_that_fold_to_other_numbers_are_left_as_printed() {
    assert_unchanged(&GUARD_STATEMENTS.replace("[1, 0, 0]", "[1 + 1, 0, 0]"));
}

#[test]
fn an_offset_that_folds_to_eleven_is_left_as_printed() {
    assert_unchanged(&ONE_DECLARATION_GUARD.replace(
        "c.charCodeAt(i + 10) - 10",
        "c.charCodeAt(i + (5 + 6)) - 10",
    ));
}

#[test]
fn split_declarations_with_a_non_empty_start_are_left_as_printed() {
    assert_unchanged(&ONE_DECLARATION_GUARD.replace(
        "  var a = \"\", b = \"\", c = a + F;\n",
        "  var a = \"\";\n  var b = \"x\";\n  var c = a + F;\n",
    ));
}

#[test]
fn a_leading_table_holding_a_name_is_left_as_printed() {
    assert_unchanged(&LITERAL_APPLY_GUARD.replace(
        "  var f = !![];\n",
        "  var k = { a: b };\n  var f = !![];\n",
    ));
}

#[test]
fn a_controller_whose_flag_starts_false_is_left_as_printed() {
    assert_unchanged(&LITERAL_APPLY_GUARD.replace("var f = !![];", "var f = false;"));
}

#[test]
fn a_guarded_function_with_three_parameters_is_left_as_printed() {
    assert_unchanged(&LITERAL_APPLY_GUARD.replace("function (c, fn)", "function (c, fn, z)"));
}

#[test]
fn a_statement_after_the_guarded_call_is_left_as_printed() {
    assert_unchanged(&LITERAL_APPLY_GUARD.replace(
        "        return x;\n      }\n",
        "        return x;\n      }\n      g();\n",
    ));
}

#[test]
fn a_guarded_function_reset_to_undefined_is_left_as_printed() {
    assert_unchanged(&LITERAL_APPLY_GUARD.replace("fn = null;", "fn = undefined;"));
}

#[test]
fn an_alternate_that_is_not_empty_is_left_as_printed() {
    assert_unchanged(
        &LITERAL_APPLY_GUARD.replace(": function () {};", ": function () {\n      g();\n    };"),
    );
}

#[test]
fn a_state_function_returning_another_word_is_left_as_printed() {
    assert_unchanged(&GUARD_STATEMENTS.replace("newState", "oldState"));
}

#[test]
fn a_regex_piece_one_character_off_is_left_as_printed() {
    assert_unchanged(&GUARD_STATEMENTS.replace(";? *}", ";? *]"));
}

#[test]
fn a_constructor_with_four_assignments_is_left_as_printed() {
    assert_unchanged(&GUARD_STATEMENTS.replace("  this.b = [1, 0, 0];\n", ""));
}

#[test]
fn a_helper_with_another_alphabet_is_left_as_printed() {
    assert_unchanged(&ONE_DECLARATION_GUARD.replace("+/=", "-_="));
}

#[test]
fn a_helper_without_the_self_reading_test_is_left_as_printed() {
    assert_unchanged(&ONE_DECLARATION_GUARD.replace("c.charCodeAt(i + 10) - 10 !== 0", "c"));
}

#[test]
fn a_target_holding_a_raw_line_break_stays_across_lines_and_is_tallied() {
    let source =
        LITERAL_APPLY_GUARD.replace("fn.apply(c, arguments)", "fn[`ap\nply`](c, arguments)");
    let (out, tally) = tallied(&source);
    assert_eq!(out, source);
    assert_eq!(tally, guard_tally(0, 1));
}

#[test]
fn a_guard_beside_ordinary_edits_is_tallied_once() {
    let source = format!("var a = [1, 2, 3];\nvar g = function () {{}};\n{LITERAL_APPLY_GUARD}");
    let (out, tally) = tallied(&source);
    assert_eq!(tally, guard_tally(1, 0));
    assert_eq!(
        out,
        source.replace(
            LITERAL_APPLY_GUARD_TARGET,
            "function () {if (fn) {var x = fn.apply(c, arguments);fn = null;return x;}}"
        )
    );
    assert_eq!(reflow_tallied(out.clone()), (out, GuardTally::default()));
}

fn nested_guards(depth: usize, apply: &str) -> String {
    (0..depth).fold("console.log(1);".to_string(), |inner, level| {
        format!(
            "var c{level} = function () {{ var f = true; return function (a, b) {{ var r = f ? function () {{ function h() {{ return function () {{ {inner} }}; }} if (b) {{ var x = {apply}(a, arguments); b = null; return x; }} }} : function () {{}}; f = false; return r; }}; }}();"
        )
    })
}

const NESTED_COMPACT: &str = "var c1 = function () {
  var f = true;
  return function (a, b) {
    var r = f ? function () {function h() {return function () {var c0 = function () {var f = true;return function (a, b) {var r = f ? function () {function h() {return function () {console.log(1);};}if (b) {var x = b.apply(a, arguments);b = null;return x;}} : function () {};f = false;return r;};}();};}if (b) {var x = b.apply(a, arguments);b = null;return x;}} : function () {};
    f = false;
    return r;
  };
}();";

#[test]
fn a_guard_inside_a_compacted_target_is_not_searched_again() {
    let (out, tally) = tallied(&nested_guards(2, "b.apply"));
    assert_eq!(out, NESTED_COMPACT);
    assert_eq!(tally, guard_tally(1, 0));
    assert_eq!(tallied(&nested_guards(3, "b.apply")).1, guard_tally(1, 0));
}

#[test]
fn a_guard_inside_a_kept_target_is_not_searched_again() {
    let alloc = Allocator::default();
    let printed = pretty_codegen(&parse_program(&alloc, &nested_guards(2, "b.apply")).program);
    let outer = printed.rfind("b.apply").unwrap_or_default();
    let source = format!("{}b[`ap\nply`]{}", &printed[..outer], &printed[outer + 7..]);
    let (out, tally) = reflow_tallied(source.clone());
    assert_eq!(out, source.replace("function(", "function ("));
    assert_eq!(tally, guard_tally(0, 1));
}

const LITERAL_APPLY_GUARD_COMPACT: &str =
    "function () {if (fn) {var x = fn.apply(c, arguments);fn = null;return x;}}";

fn assigned(declared: &str, name: &str, operator: &str) -> String {
    let from = format!("var {name} = ");
    let to = format!("var {name};\n{name} {operator} ");
    declared.replacen(&from, &to, 1)
}

#[test]
fn a_let_controller_prints_its_read_target_on_one_line() {
    let source = LITERAL_APPLY_GUARD.replacen("var ctrl", "let ctrl", 1);
    assert_compacted(
        &source,
        LITERAL_APPLY_GUARD_TARGET,
        LITERAL_APPLY_GUARD_COMPACT,
    );
}

#[test]
fn a_const_state_constructor_prints_its_read_target_on_one_line() {
    let source = GUARD_STATEMENTS.replacen("var State", "const State", 1);
    assert_compacted(&source, NEW_STATE_TARGET, NEW_STATE_COMPACT);
}

#[test]
fn a_controller_assigned_with_a_plain_equals_prints_its_read_target_on_one_line() {
    let source = assigned(LITERAL_APPLY_GUARD, "ctrl", "=");
    assert_compacted(
        &source,
        LITERAL_APPLY_GUARD_TARGET,
        LITERAL_APPLY_GUARD_COMPACT,
    );
}

#[test]
fn a_helper_assigned_with_a_plain_equals_prints_on_one_line() {
    let source = assigned(ONE_DECLARATION_GUARD, "F", "=");
    assert_compacted(
        &source,
        guard_target(&source),
        ONE_DECLARATION_GUARD_COMPACT,
    );
}

#[test]
fn a_compound_assignment_is_left_as_printed() {
    assert_unchanged(&assigned(LITERAL_APPLY_GUARD, "ctrl", "+="));
}

#[test]
fn a_destructuring_target_is_left_as_printed() {
    let source = LITERAL_APPLY_GUARD.replacen("var ctrl = ", "var ctrl;\n[ctrl] = ", 1);
    assert_unchanged(&source);
}

#[test]
fn a_member_target_is_left_as_printed() {
    let source = LITERAL_APPLY_GUARD.replacen("var ctrl = ", "var o = {};\no.ctrl = ", 1);
    assert_unchanged(&source);
}

#[test]
fn an_assignment_inside_a_sequence_is_left_as_printed() {
    let source = LITERAL_APPLY_GUARD.replacen("var ctrl = ", "var ctrl;\n0, ctrl = ", 1);
    assert_unchanged(&source);
}

fn reprinted(code: &str) -> String {
    let alloc = Allocator::default();
    pretty_codegen(&parse_program(&alloc, code).program)
}

fn recorded(code: &str) -> (String, GuardRecord) {
    let reparsed = reflow_recorded(reprinted(code));
    assert_eq!(placed_as_reparsed(code), reparsed);
    reparsed
}

#[test]
fn a_reprint_of_a_guard_already_on_one_line_is_rewritten_but_not_tallied_again() {
    let (out, earlier) = recorded(LITERAL_APPLY_GUARD);
    assert_eq!(earlier.tally(), guard_tally(1, 0));
    assert!(!reprinted(&out).contains(LITERAL_APPLY_GUARD_COMPACT));
    let (again, record) = recorded(&out);
    assert_eq!(again, out);
    assert_eq!(record.tally(), guard_tally(1, 0));
    assert_eq!(record.tally_beyond(&earlier), GuardTally::default());
    let (_, other) = recorded(GUARD_STATEMENTS);
    assert_eq!(other.tally_beyond(&earlier), guard_tally(1, 0));
}

#[test]
fn a_reprint_of_a_kept_guard_is_not_tallied_again() {
    let source =
        LITERAL_APPLY_GUARD.replace("fn.apply(c, arguments)", "fn[`ap\nply`](c, arguments)");
    let (out, earlier) = recorded(&source);
    assert_eq!(earlier.tally(), guard_tally(0, 1));
    let (_, record) = recorded(&out);
    assert_eq!(record.tally(), guard_tally(0, 1));
    assert_eq!(record.tally_beyond(&earlier), GuardTally::default());
}

#[test]
fn two_equal_guards_are_each_tallied_and_absorbed() {
    let source = format!(
        "{LITERAL_APPLY_GUARD}\n{}",
        LITERAL_APPLY_GUARD.replace("var ctrl", "var other")
    );
    let (_, record) = recorded(&source);
    assert_eq!(record.tally(), guard_tally(2, 0));
    let mut all = GuardRecord::default();
    all.absorb(record.clone());
    all.absorb(recorded(GUARD_STATEMENTS).1);
    assert_eq!(all.tally(), guard_tally(3, 0));
    assert_eq!(record.tally_beyond(&all), GuardTally::default());
}

#[test]
fn a_guard_shaped_call_outside_any_slot_is_left_as_printed() {
    let source = LITERAL_APPLY_GUARD
        .replacen("var ctrl = ", "console.log(", 1)
        .replacen("}();", "}());", 1);
    assert_unchanged(&source);
}

#[test]
fn a_guard_seen_more_often_than_earlier_counts_the_excess() {
    let (_, earlier) = recorded(LITERAL_APPLY_GUARD);
    let twice = format!(
        "{LITERAL_APPLY_GUARD}\n{}",
        LITERAL_APPLY_GUARD.replace("var ctrl", "var other")
    );
    let (_, record) = recorded(&twice);
    assert_eq!(record.tally_beyond(&earlier), guard_tally(1, 0));
}

fn beyond(earlier: &str, later: &str) -> GuardTally {
    let (_, seen) = recorded(earlier);
    let (_, record) = recorded(later);
    assert_eq!(record.tally(), guard_tally(1, 0), "not a guard: {later}");
    record.tally_beyond(&seen)
}

#[test]
fn a_guard_whose_free_and_bound_names_are_renamed_is_not_tallied_again() {
    let helper = ONE_DECLARATION_GUARD
        .replace("c = a + F", "c = a + module")
        .replace("(s)", "(input)")
        .replace("s.charAt", "input.charAt")
        .replace("var A", "var table")
        .replace("A.indexOf", "table.indexOf");
    assert_eq!(
        beyond(ONE_DECLARATION_GUARD, &helper),
        GuardTally::default()
    );
    let controller = LITERAL_APPLY_GUARD
        .replace("(c, fn)", "(context, callback)")
        .replace("fn.apply(c,", "callback.apply(context,")
        .replace("if (fn)", "if (callback)")
        .replace("fn = null", "callback = null")
        .replace("var x", "var result")
        .replace("return x", "return result");
    assert_eq!(
        beyond(LITERAL_APPLY_GUARD, &controller),
        GuardTally::default()
    );
}

#[test]
fn a_guard_that_differs_beyond_its_names_is_tallied_again() {
    let other = ONE_DECLARATION_GUARD.replace("s.charAt", "s.charCodeAt");
    assert_eq!(beyond(ONE_DECLARATION_GUARD, &other), guard_tally(1, 0));
}

#[test]
fn a_property_spelled_like_a_renamed_name_is_not_renamed() {
    let earlier = LITERAL_APPLY_GUARD.replace("fn.apply(c, arguments)", "fn.c(c, arguments)");
    let other = LITERAL_APPLY_GUARD.replace("fn.apply(c, arguments)", "fn.x(c, arguments)");
    assert_eq!(beyond(&earlier, &other), guard_tally(1, 0), "seen: {other}");
}

fn assert_left_as_reprinted(source: &str) {
    let (out, tally) = reflow_tallied(reprinted(source));
    assert!(out.contains("fn = null;\n"), "{out}");
    assert!(!out.contains(LITERAL_APPLY_GUARD_COMPACT), "{out}");
    assert_eq!(tally, GuardTally::default());
}

#[test]
fn logical_assignments_are_left_as_printed() {
    for operator in ["??=", "||=", "&&="] {
        assert_left_as_reprinted(&assigned(LITERAL_APPLY_GUARD, "ctrl", operator));
    }
}

#[test]
fn a_chained_assignment_is_left_as_printed() {
    let source =
        LITERAL_APPLY_GUARD.replacen("var ctrl = ", "var ctrl, other;\nother = ctrl = ", 1);
    assert_left_as_reprinted(&source);
}

#[test]
fn a_class_field_is_left_as_printed() {
    let source = format!(
        "class K {{\n  ctrl = {}\n}}",
        &LITERAL_APPLY_GUARD["var ctrl = ".len()..]
    );
    assert_left_as_reprinted(&source);
}

#[test]
fn edits_between_and_after_guards_keep_their_places() {
    let pieces = [
        "var a = function() { return [1, 2, 3]; };".to_string(),
        LITERAL_APPLY_GUARD.to_string(),
        "var b = function() { return [4,\n5, 6]; };".to_string(),
        LITERAL_APPLY_GUARD.replace("var ctrl", "var other"),
        "var c = function() { return [7, 8, 9]; };".to_string(),
    ];
    let printed: Vec<String> = pieces.iter().map(|piece| reprinted(piece)).collect();
    let (out, tally) = reflow_tallied(printed.concat());
    let expected: String = printed.into_iter().map(reflow).collect();
    assert_eq!(out, expected);
    assert_eq!(tally, guard_tally(2, 0));
}

fn with_key(key: &str) -> String {
    LITERAL_APPLY_GUARD.replace(
        "fn.apply(c, arguments)",
        &format!("fn[{key}](c, arguments)"),
    )
}

#[test]
fn a_guard_that_differs_in_a_regex_a_number_an_object_key_or_a_template_is_tallied_again() {
    for (earlier, later) in [
        ("/apply/.source", "/call/.source"),
        ("/\\d/.source", "/\\w/.source"),
        ("/a/g.flags", "/a/y.flags"),
        ("1e21", "1e22"),
        ("0xabcdef12345", "0xfedcba54321"),
        ("{ foo: 'apply' }.foo", "{ bar: 'apply' }.foo"),
        (
            "{ foo() { return 'apply'; } }.foo()",
            "{ bar() { return 'apply'; } }.foo()",
        ),
        ("`${`apply`}`", "`${`call`}`"),
    ] {
        let (earlier, later) = (with_key(earlier), with_key(later));
        assert_eq!(beyond(&earlier, &later), guard_tally(1, 0), "{later}");
    }
}

#[test]
fn a_guard_whose_keys_differ_only_in_names_is_not_tallied_again() {
    for key in [
        "'apply'",
        "/apply/.source",
        "1e21",
        "{ foo: c }.foo",
        "`${c}`",
    ] {
        let earlier = with_key(key);
        let later = earlier
            .replace("(c, fn)", "(context, fn)")
            .replace("](c, arguments)", "](context, arguments)")
            .replace("foo: c }", "foo: context }")
            .replace("${c}", "${context}");
        assert_eq!(beyond(&earlier, &later), GuardTally::default(), "{later}");
    }
}

#[test]
fn a_controller_declared_with_let_and_const_is_matched() {
    let source = "let ctrl = function () {
  let f = true;
  return function (c, fn) {
    const r = f ? function () {
      const w = d;
      if (fn) {
        const x = fn[w(430)](c, arguments);
        return fn = null, x;
      }
    } : function () {};
    return f = false, r;
  };
}();";
    let target = "function () {
      const w = d;
      if (fn) {
        const x = fn[w(430)](c, arguments);
        return fn = null, x;
      }
    }";
    assert_compacted(
        source,
        target,
        "function () {const w = d;if (fn) {const x = fn[w(430)](c, arguments);return fn = null, x;}}",
    );
}

#[test]
fn a_controller_with_dead_code_and_proxy_tables_is_matched() {
    let source = "let ctrl = function () {
  const p = {
    eq: function (a, b) {
      return a === b;
    },
    k: t[d(1)]
  };
  let f = true;
  return function (c, fn) {
    const q = {
      ne: function (a, b) {
        return p[d(2)](a, b);
      },
      z: p.k
    };
    const r = f ? function () {
      if (q[d(3)](q.z, d(4))) {
        return u[v];
      } else if (fn) {
        const x = fn[d(5)](c, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = false;
    return r;
  };
}();";
    let target = "function () {
      if (q[d(3)](q.z, d(4))) {
        return u[v];
      } else if (fn) {
        const x = fn[d(5)](c, arguments);
        fn = null;
        return x;
      }
    }";
    assert_compacted(
        source,
        target,
        "function () {if (q[d(3)](q.z, d(4))) {return u[v];} else if (fn) {const x = fn[d(5)](c, arguments);fn = null;return x;}}",
    );
}

#[test]
fn guards_are_placed_from_the_printed_program_without_a_reparse() {
    for source in [
        LITERAL_APPLY_GUARD,
        GUARD_STATEMENTS,
        ONE_DECLARATION_GUARD,
        &nested_guards(2, "b.apply"),
        &format!("var a = [1, 2, 3];\nvar g = function () {{}};\n{LITERAL_APPLY_GUARD}"),
    ] {
        let (placed, code, record) = placed(source);
        assert!(placed, "{source}");
        assert_eq!((code, record), recorded(source), "{source}");
    }
}

#[test]
fn a_guard_placed_a_byte_off_differs_from_the_reparse() {
    let source = format!("var a = [1, 2, 3];\n{LITERAL_APPLY_GUARD}");
    misplace_an_edit_for_tests(true);
    let (placed, code, _) = placed(&source);
    misplace_an_edit_for_tests(false);
    assert!(placed);
    assert_ne!(code, recorded(&source).0);
}

#[test]
fn a_guard_beside_text_spelled_like_its_marker_is_reflowed_through_the_reparse() {
    let source = format!("{LITERAL_APPLY_GUARD}\nvar z = \"\u{e000}\";");
    let (placed, code, record) = placed(&source);
    assert!(!placed);
    assert_eq!((code, record), recorded(&source));
}
