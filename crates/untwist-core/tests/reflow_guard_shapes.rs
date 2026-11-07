use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{pretty_codegen, print, PrintStyle};
use untwist_core::reflow::{print_placing, reflow, reflow_recorded, GuardRecord, GuardTally};

fn placed_as_reparsed(source: &str) -> (String, GuardRecord) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let reparsed = reflow_recorded(pretty_codegen(&parsed.program));
    let printed = print_placing(&alloc, &mut parsed.program, None);
    let (code, record, _) = printed.finish(None);
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

fn tally(source: &str) -> GuardTally {
    placed_as_reparsed(source).1.tally()
}

fn assert_compacted(source: &str, target: &str, compact: &str) {
    assert_eq!(source.matches(target).count(), 1, "{source}");
    let expected = source.replace(target, compact);
    let out = pretty(source);
    assert_eq!(out, expected);
    assert_eq!(reflow(out.clone()), out);
    assert_eq!(pretty(&out), out);
    assert_eq!(
        tally(source),
        GuardTally {
            compacted: 1,
            kept: 0
        }
    );
}

fn assert_unchanged(source: &str) {
    assert_eq!(pretty(source), source);
    assert_eq!(tally(source), GuardTally::default());
}

const COOKIE: &str = "(function (J) {
  var L = function (M) {
    while (--M) {
      J.push(J.shift());
    }
  };
  (function () {
    var M = {
      data: {
        key: \"cookie\",
        value: \"timeout\"
      },
      removeCookie: function () {
        return \"dev\";
      },
      getCookie: function () {
        L(3);
      }
    };
    M.updateCookie = function () {
      return new RegExp(\"\\\\w+ *\\\\(\\\\) *{\\\\w+ *['|\\\"].+['|\\\"];? *}\").test(M.removeCookie.toString());
    };
    var N = M.updateCookie();
    if (N) {
      M.getCookie(null, \"counter\");
    }
  })();
})(arr);";

const COOKIE_TARGET: &str = "function () {
        return \"dev\";
      }";

const COOKIE_COMPACT: &str = "function () {return \"dev\";}";

#[test]
fn the_cookie_rotator_prints_its_remove_cookie_function_on_one_line() {
    assert_compacted(COOKIE, COOKIE_TARGET, COOKIE_COMPACT);
}

#[test]
fn a_cookie_check_written_with_computed_keys_is_matched() {
    let source = COOKIE
        .replace("M.updateCookie = ", "M[\"updateCookie\"] = ")
        .replace(
            "M.removeCookie.toString()",
            "M[\"removeCookie\"][\"toString\"]()",
        );
    assert_compacted(&source, COOKIE_TARGET, COOKIE_COMPACT);
}

#[test]
fn a_cookie_check_with_another_regex_is_left_as_printed() {
    assert_unchanged(&COOKIE.replace(";? *}", ";? *]"));
}

#[test]
fn a_cookie_check_reading_another_function_is_left_as_printed() {
    assert_unchanged(&COOKIE.replace("M.removeCookie.toString()", "M.getCookie.toString()"));
}

#[test]
fn a_remove_cookie_returning_a_name_is_left_as_printed() {
    assert_unchanged(&COOKIE.replace("return \"dev\";", "return dev;"));
}

#[test]
fn a_cookie_holder_with_two_remove_cookie_keys_is_left_as_printed() {
    assert_unchanged(&COOKIE.replace(
        "      getCookie: function () {",
        "      removeCookie: function () {\n        return \"x\";\n      },\n      getCookie: function () {",
    ));
}

const INLINE_CONTROLLER: &str = "var A = function () {
  var J = true;
  return function (K, L) {
    var M = J ? function () {
      var N = function (P, Q) {
        return k(P - 719, Q);
      };
      if (L) {
        var O = L[N(\"0xac1\", \"pqW&\")](K, arguments);
        L = null;
        return O;
      }
    } : function () {};
    J = false;
    return M;
  };
}()(this, function () {
  return A.toString();
});";

const INLINE_TARGET: &str = "function () {
      var N = function (P, Q) {
        return k(P - 719, Q);
      };
      if (L) {
        var O = L[N(\"0xac1\", \"pqW&\")](K, arguments);
        L = null;
        return O;
      }
    }";

const INLINE_COMPACT: &str = "function () {var N = function (P, Q) {return k(P - 719, Q);};if (L) {var O = L[N(\"0xac1\", \"pqW&\")](K, arguments);L = null;return O;}}";

#[test]
fn a_controller_called_where_it_is_built_with_a_wrapper_variable_is_matched() {
    assert_compacted(INLINE_CONTROLLER, INLINE_TARGET, INLINE_COMPACT);
}

#[test]
fn a_wrapper_arrow_in_the_guarded_function_is_matched() {
    let arrow = |text: &str| {
        text.replace(
            "var N = function (P, Q) {\n        return k(P - 719, Q);\n      };",
            "const N = (P, Q) => {\n        return k(P - 719, Q);\n      };",
        )
        .replace(
            "var N = function (P, Q) {return k(P - 719, Q);};",
            "const N = (P, Q) => {return k(P - 719, Q);};",
        )
    };
    assert_compacted(
        &arrow(INLINE_CONTROLLER),
        &arrow(INLINE_TARGET),
        &arrow(INLINE_COMPACT),
    );
}

#[test]
fn a_wrapper_variable_that_runs_code_is_left_as_printed() {
    assert_unchanged(&INLINE_CONTROLLER.replace(
        "      var N = function (P, Q) {\n        return k(P - 719, Q);\n      };",
        "      var N = g(P);",
    ));
}

const SEQUENCE_CONTROLLER: &str = "var j;
var A = (j = true, function (K, L) {
  var M = j ? function () {
    if (L) {
      var O = L.apply(K, arguments);
      L = null;
      return O;
    }
  } : function () {};
  j = false;
  return M;
})(this, function () {
  return A.toString();
});";

const SEQUENCE_TARGET: &str = "function () {
    if (L) {
      var O = L.apply(K, arguments);
      L = null;
      return O;
    }
  }";

const SEQUENCE_COMPACT: &str =
    "function () {if (L) {var O = L.apply(K, arguments);L = null;return O;}}";

#[test]
fn a_controller_built_in_a_sequence_is_matched() {
    assert_compacted(SEQUENCE_CONTROLLER, SEQUENCE_TARGET, SEQUENCE_COMPACT);
}

#[test]
fn a_sequence_controller_with_a_working_alternate_is_left_as_printed() {
    assert_unchanged(
        &SEQUENCE_CONTROLLER.replace(": function () {};", ": function () {\n    g();\n  };"),
    );
}

#[test]
fn a_sequence_controller_choosing_on_its_own_parameter_is_left_as_printed() {
    assert_unchanged(&SEQUENCE_CONTROLLER.replace("var M = j ?", "var M = K ?"));
}

#[test]
fn a_flag_reset_to_not_one_is_matched() {
    let source = SEQUENCE_CONTROLLER.replace("j = false;", "j = !1;");
    assert_compacted(&source, SEQUENCE_TARGET, SEQUENCE_COMPACT);
}

#[test]
fn a_flag_reset_to_not_zero_is_left_as_printed() {
    assert_unchanged(&SEQUENCE_CONTROLLER.replace("j = false;", "j = !0;"));
}
