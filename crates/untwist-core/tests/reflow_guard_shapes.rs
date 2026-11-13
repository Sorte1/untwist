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
fn a_cookie_check_with_another_regex_still_prints_its_bait_on_one_line() {
    let source = COOKIE.replace(";? *}", ";? *]");
    assert_compacted(&source, COOKIE_TARGET, COOKIE_COMPACT);
}

#[test]
fn a_cookie_check_reading_another_function_still_prints_its_bait_on_one_line() {
    let source = COOKIE.replace("M.removeCookie.toString()", "M.getCookie.toString()");
    assert_compacted(&source, COOKIE_TARGET, COOKIE_COMPACT);
}

#[test]
fn a_remove_cookie_returning_a_name_is_left_as_printed() {
    assert_unchanged(&COOKIE.replace("return \"dev\";", "return dev;"));
}

#[test]
fn a_cookie_holder_with_two_remove_cookie_keys_still_prints_its_bait_on_one_line() {
    let source = COOKIE.replace(
        "      getCookie: function () {",
        "      removeCookie: function () {\n        return \"x\";\n      },\n      getCookie: function () {",
    );
    assert_compacted(&source, COOKIE_TARGET, COOKIE_COMPACT);
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
fn a_wrapper_variable_that_runs_code_is_matched() {
    let swap = |text: &str| {
        text.replace(
            "      var N = function (P, Q) {\n        return k(P - 719, Q);\n      };",
            "      var N = g(P);",
        )
        .replace(
            "var N = function (P, Q) {return k(P - 719, Q);};",
            "var N = g(P);",
        )
    };
    assert_compacted(
        &swap(INLINE_CONTROLLER),
        &swap(INLINE_TARGET),
        &swap(INLINE_COMPACT),
    );
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

const DEAD_CODE_CONTROLLER: &str = "var o = function () {
  if (D.a(D.b, \"x\")) {
    var G = !![];
    return function (O, L) {
      var q = y;
      var z = {
        m: function (F, l) {
          var w = J;
          return F != l;
        },
        s: D.c
      };
      var Q = G ? function () {
        var j = J;
        var F = { u: function (A, R) {
          var v = J;
          return z.m(A, R);
        } };
        if (L) {
          if (z.s === j(434, \"0Lub\")) {
            var l = L[j(482, \"I[&I\")](O, arguments);
            L = null;
            return l;
          } else {
            return F.u(g, null);
          }
        }
      } : function () {};
      G = ![];
      return Q;
    };
  } else {
    delete g[P];
  }
}();";

const DEAD_CODE_TARGET: &str = "function () {
        var j = J;
        var F = { u: function (A, R) {
          var v = J;
          return z.m(A, R);
        } };
        if (L) {
          if (z.s === j(434, \"0Lub\")) {
            var l = L[j(482, \"I[&I\")](O, arguments);
            L = null;
            return l;
          } else {
            return F.u(g, null);
          }
        }
      }";

const DEAD_CODE_COMPACT: &str = "function () {var j = J;var F = { u: function (A, R) {var v = J;return z.m(A, R);} };if (L) {if (z.s === j(434, \"0Lub\")) {var l = L[j(482, \"I[&I\")](O, arguments);L = null;return l;} else {return F.u(g, null);}}}";

#[test]
fn a_controller_inside_a_dead_code_branch_with_proxy_tables_is_matched() {
    assert_compacted(DEAD_CODE_CONTROLLER, DEAD_CODE_TARGET, DEAD_CODE_COMPACT);
}

#[test]
fn a_dead_code_controller_with_a_working_alternate_is_left_as_printed() {
    assert_unchanged(&DEAD_CODE_CONTROLLER.replace(
        "} : function () {};",
        "} : function () {\n        g();\n      };",
    ));
}

const BRANCHED_CHOICE: &str = "var o = function () {
  var G = !![];
  return function (O, L) {
    if (k(1) === k(1)) {
      var Q = G ? function () {
        if (k(2) !== k(3)) {
          if (L) {
            var l = L.apply(O, arguments);
            L = null;
            return l;
          }
        } else {
          return n.toString();
        }
      } : function () {};
      G = ![];
      return Q;
    } else {
      return function (t) {
        return t;
      };
    }
  };
}();";

const BRANCHED_TARGET: &str = "function () {
        if (k(2) !== k(3)) {
          if (L) {
            var l = L.apply(O, arguments);
            L = null;
            return l;
          }
        } else {
          return n.toString();
        }
      }";

const BRANCHED_COMPACT: &str = "function () {if (k(2) !== k(3)) {if (L) {var l = L.apply(O, arguments);L = null;return l;}} else {return n.toString();}}";

#[test]
fn a_choice_inside_a_dead_code_branch_of_the_guarded_function_is_matched() {
    assert_compacted(BRANCHED_CHOICE, BRANCHED_TARGET, BRANCHED_COMPACT);
}

#[test]
fn a_branched_choice_whose_target_never_resets_the_callback_is_left_as_printed() {
    assert_unchanged(&BRANCHED_CHOICE.replace("            L = null;\n", ""));
}

#[test]
fn a_branched_choice_whose_target_forwards_another_receiver_is_left_as_printed() {
    assert_unchanged(&BRANCHED_CHOICE.replace("L.apply(O, arguments)", "L.apply(t, arguments)"));
}

#[test]
fn a_guarded_function_opening_with_split_object_keys_is_matched() {
    let split = |text: &str| {
        text.replacen(
            "      var q = y;\n",
            "      var q = y;\n      var h = {};\n      h[q(1)] = z;\n",
            1,
        )
    };
    assert_compacted(
        &split(DEAD_CODE_CONTROLLER),
        DEAD_CODE_TARGET,
        DEAD_CODE_COMPACT,
    );
}

const DECOY_BRANCH: &str = "var o = function () {
  var G = !![];
  return function (O, L) {
    var T = { a: \"x\" };
    if (T.a !== T.a) {
      var X = H ? function () {
        if (B) {
          var r = B.apply(C, arguments);
          E = null;
          return r;
        }
      } : function () {};
      H = ![];
      return X;
    } else {
      var Q = G ? function () {
        if (L) {
          var l = L.apply(O, arguments);
          L = null;
          return l;
        }
      } : function () {};
      G = ![];
      return Q;
    }
  };
}();";

const DECOY_TARGET: &str = "function () {
        if (L) {
          var l = L.apply(O, arguments);
          L = null;
          return l;
        }
      }";

#[test]
fn a_decoy_template_in_the_other_branch_does_not_hide_the_real_one() {
    assert_compacted(
        DECOY_BRANCH,
        DECOY_TARGET,
        "function () {if (L) {var l = L.apply(O, arguments);L = null;return l;}}",
    );
}

const MINIFIED_BRANCH: &str = "var o = function () {
  if (!W.a(W.b, W.c)) {
    let n = !0;
    return function (r, e) {
      const c = { h: W.d };
      if (!W.e(W.f, W.g)) {
        const W = n ? function () {
          if (c.h(1, 2)) {
            return new this(x);
          }
          if (e) {
            if (c.h(3, 4)) {
              const n = e.apply(r, arguments);
              e = null;
              return n;
            }
            return y;
          }
        } : function () {};
        n = !1;
        return W;
      }
      {
        const n = z[k];
        q[w] = n;
      }
    };
  }
  g = h;
}();";

const MINIFIED_TARGET: &str = "function () {
          if (c.h(1, 2)) {
            return new this(x);
          }
          if (e) {
            if (c.h(3, 4)) {
              const n = e.apply(r, arguments);
              e = null;
              return n;
            }
            return y;
          }
        }";

#[test]
fn a_minified_dead_code_branch_without_an_else_is_matched() {
    assert_compacted(
        MINIFIED_BRANCH,
        MINIFIED_TARGET,
        "function () {if (c.h(1, 2)) {return new this(x);}if (e) {if (c.h(3, 4)) {const n = e.apply(r, arguments);e = null;return n;}return y;}}",
    );
}

#[test]
fn unreachable_code_after_the_handoff_is_matched() {
    let source = SEQUENCE_CONTROLLER.replace(
        "  return M;\n})",
        "  return M;\n  if (Z) {\n    g();\n  }\n})",
    );
    assert_compacted(&source, SEQUENCE_TARGET, SEQUENCE_COMPACT);
}

#[test]
fn a_hoisted_function_between_the_reset_and_the_handoff_is_matched() {
    let source = SEQUENCE_CONTROLLER.replace(
        "  j = false;\n  return M;\n",
        "  j = false;\n  function h() {\n    return 1;\n  }\n  return M;\n",
    );
    assert_compacted(&source, SEQUENCE_TARGET, SEQUENCE_COMPACT);
}

const TERSER_HELPER: &str = "var u = function (t) {
  let n = \"\";
  let o = \"\";
  let e = n + u;
  for (let o, r, u = 0, i = 0; r = t.charAt(i++); ~r && (o = u % 4 ? 64 * o + r : r, u++ % 4) ? n += e.charCodeAt(i + 10) - 10 != 0 ? String.fromCharCode(255 & o >> (-2 * u & 6)) : u : 0) {
    r = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\".indexOf(r);
  }
  for (let t = 0, e = n.length; t < e; t++) {
    o += \"%\" + (\"00\" + n.charCodeAt(t).toString(16)).slice(-2);
  }
  return decodeURIComponent(o);
};";

#[test]
fn a_terser_helper_with_its_alphabet_inlined_and_a_loose_test_is_matched() {
    let target = &TERSER_HELPER["var u = ".len()..TERSER_HELPER.len() - 1];
    let compact: String = target
        .lines()
        .enumerate()
        .map(|(index, line)| if index == 0 { line } else { line.trim_start() })
        .collect();
    assert_compacted(TERSER_HELPER, target, &compact);
}

#[test]
fn a_bait_function_returning_dev_window_or_new_state_is_matched_anywhere() {
    for word in ["dev", "window", "newState"] {
        let source =
            format!("var k = {{\n  a: 1,\n  r: function () {{\n    return \"{word}\";\n  }}\n}};");
        let target = format!("function () {{\n    return \"{word}\";\n  }}");
        let compact = format!("function () {{return \"{word}\";}}");
        assert_compacted(&source, &target, &compact);
    }
}

#[test]
fn a_function_returning_another_word_is_left_as_printed() {
    assert_unchanged("var k = {\n  a: 1,\n  r: function () {\n    return \"devs\";\n  }\n};");
}
