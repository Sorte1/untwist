use untwist_core::guard_shape::{source_readers, SourceReaders};

const CONTROLLER_ONE_LINE: &str = "var c = function () {
  var f = true;
  return function (t, fn) {
    var r = f ? function () {if (fn) {var x = fn.apply(t, arguments);fn = null;return x;}} : function () {};
    f = false;
    return r;
  };
}();
";

const CONTROLLER_ACROSS_LINES: &str = "var c = function () {
  var f = true;
  return function (t, fn) {
    var r = f ? function () {
      if (fn) {
        var x = fn.apply(t, arguments);
        fn = null;
        return x;
      }
    } : function () {};
    f = false;
    return r;
  };
}();
";

const READER: &str = "var n = c(this, function () {
  return n.toString().search(\"(((.+)+)+)+$\").toString().constructor(n).search(\"(((.+)+)+)+$\");
});
n();
";

const ENCODED_READER: &str = "var n = c(this, () => {
  if (n[k(1)]()[k(2)]()[k(3)](\"\\n\") !== -1) {
    return;
  }
  return n[k(2)]()[k(4)](D[k(5)])[k(2)]()[k(6)](n)[k(4)](D[k(5)]);
});
n();
";

#[test]
fn a_reader_whose_target_prints_on_one_line_is_guarded() {
    let code = format!("{CONTROLLER_ONE_LINE}{READER}");
    assert_eq!(source_readers(&code), SourceReaders::Guarded);
}

#[test]
fn a_reader_whose_target_prints_across_lines_is_unguarded() {
    let code = format!("{CONTROLLER_ACROSS_LINES}{READER}");
    assert_eq!(source_readers(&code), SourceReaders::Unguarded);
}

#[test]
fn an_encoded_arrow_reader_is_seen() {
    let code = format!("{CONTROLLER_ACROSS_LINES}{ENCODED_READER}");
    assert_eq!(source_readers(&code), SourceReaders::Unguarded);
    let code = format!("{CONTROLLER_ONE_LINE}{ENCODED_READER}");
    assert_eq!(source_readers(&code), SourceReaders::Guarded);
}

#[test]
fn a_reader_whose_controller_is_not_found_is_unguarded() {
    let code = format!("function c(t, fn) {{\n  return fn;\n}}\n{READER}");
    assert_eq!(source_readers(&code), SourceReaders::Unguarded);
}

#[test]
fn a_controller_called_where_it_is_built_is_resolved() {
    let built = |controller: &str| {
        let callee = controller
            .trim_end()
            .trim_start_matches("var c = ")
            .trim_end_matches(';');
        READER.replacen("c(this,", &format!("({callee})(this,"), 1)
    };
    assert_eq!(
        source_readers(&built(CONTROLLER_ONE_LINE)),
        SourceReaders::Guarded
    );
    assert_eq!(
        source_readers(&built(CONTROLLER_ACROSS_LINES)),
        SourceReaders::Unguarded
    );
}

#[test]
fn a_program_without_a_reader_has_none() {
    assert_eq!(source_readers(CONTROLLER_ACROSS_LINES), SourceReaders::None);
    let short = "var n = c(this, function () {\n  return n.toString().length;\n});\n";
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ACROSS_LINES}{short}")),
        SourceReaders::None
    );
    let other = "var n = c(this, function () {\n  return m.toString().search(r).toString().constructor(m);\n});\n";
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ACROSS_LINES}{other}")),
        SourceReaders::None
    );
}

#[test]
fn a_reader_called_through_a_proxy_is_seen() {
    let proxied = READER.replacen("c(this,", "D.p(c, this,", 1);
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ACROSS_LINES}{proxied}")),
        SourceReaders::Unguarded
    );
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ONE_LINE}{proxied}")),
        SourceReaders::Guarded
    );
}

#[test]
fn the_older_constructor_test_reader_is_seen() {
    let older = "var n = c(this, function () {
  var Q = function () {
    return !Q.constructor(\"return /\" + this + \"/\")().constructor(\"^([^ ]+( +[^ ]+)+)+[^ ]}\").test(n);
  };
  return Q();
});
n();
";
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ACROSS_LINES}{older}")),
        SourceReaders::Unguarded
    );
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ONE_LINE}{older}")),
        SourceReaders::Guarded
    );
}

#[test]
fn a_function_that_adds_its_own_name_to_a_string_is_a_reader() {
    let across = "var F = function (s) {\n  var a = \"\";\n  var c = a + F;\n  return c.charCodeAt(s);\n};\n";
    assert_eq!(source_readers(across), SourceReaders::Unguarded);
    let with_reader = format!("{across}{CONTROLLER_ONE_LINE}{READER}");
    assert_eq!(source_readers(&with_reader), SourceReaders::Unguarded);
    let one_line = "var F = function (s) {var a = \"\";var c = a + F;return c.charCodeAt(s);};\n";
    let with_reader = format!("{one_line}{CONTROLLER_ONE_LINE}{READER}");
    assert_eq!(source_readers(&with_reader), SourceReaders::Guarded);
}

#[test]
fn a_self_read_behind_the_line_break_escape_is_not_a_reader() {
    let escaped = "var F = function (s) {\n  var a = \"\";\n  var c = a + F;\n  var z = (\"\" + function () {\n    return 0;\n  }).indexOf(\"\\n\") !== -1;\n  return z || c.charCodeAt(s);\n};\n";
    let with_reader = format!("{escaped}{CONTROLLER_ONE_LINE}{READER}");
    assert_eq!(source_readers(&with_reader), SourceReaders::Guarded);
}

#[test]
fn a_reader_that_checks_the_escaped_spelling_of_a_bait_is_unguarded() {
    let old = "var n = c(this, function () {\n  var a = function () {return \"window\";};\n  var t = function () {\n    return new RegExp(\"(\\\\\\\\[x|u](\\\\w){2,4})+\").test(a.toString());\n  };\n  t();\n});\nn();\n";
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ONE_LINE}{old}")),
        SourceReaders::Unguarded
    );
}

fn scoped(controller: &str) -> String {
    format!("function other() {{\n{controller}return c;\n}}\n")
}

#[test]
fn a_same_named_controller_in_another_scope_does_not_guard_the_reader() {
    let code = format!(
        "{}{CONTROLLER_ACROSS_LINES}{READER}",
        scoped(CONTROLLER_ONE_LINE)
    );
    assert_eq!(source_readers(&code), SourceReaders::Unguarded);
    let code = format!(
        "{}{CONTROLLER_ONE_LINE}{READER}",
        scoped(CONTROLLER_ACROSS_LINES)
    );
    assert_eq!(source_readers(&code), SourceReaders::Guarded);
}

#[test]
fn a_reassigned_controller_is_unguarded() {
    let reassigned = "c = function (t, fn) {\n  return function () {\n    return fn.apply(t, arguments);\n  };\n};\n";
    let code = format!("{CONTROLLER_ONE_LINE}{reassigned}{READER}");
    assert_eq!(source_readers(&code), SourceReaders::Unguarded);
    let updated = format!("{CONTROLLER_ONE_LINE}c += 1;\n{READER}");
    assert_eq!(source_readers(&updated), SourceReaders::Unguarded);
    let declared_twice = format!("{CONTROLLER_ONE_LINE}var c = c;\n{READER}");
    assert_eq!(source_readers(&declared_twice), SourceReaders::Unguarded);
}

#[test]
fn a_controller_assigned_once_after_a_bare_declaration_is_resolved() {
    let assigned = CONTROLLER_ONE_LINE.replacen("var c = ", "var c;\nc = ", 1);
    assert_eq!(
        source_readers(&format!("{assigned}{READER}")),
        SourceReaders::Guarded
    );
}

#[test]
fn a_controller_bound_as_a_parameter_is_unguarded() {
    let code = format!(
        "function run(c) {{\n{READER}}}\nrun({});\n",
        CONTROLLER_ONE_LINE
            .trim_start_matches("var c = ")
            .trim_end()
            .trim_end_matches(';')
    );
    assert_eq!(source_readers(&code), SourceReaders::Unguarded);
}

#[test]
fn a_controller_with_two_targets_needs_both_on_one_line() {
    let two = |second: &str| {
        format!(
            "var c = function () {{\n  var f = true;\n  return function (t, fn) {{\n    if (typeof t === \"number\") {{\n      var r = f ? function () {{if (fn) {{var x = fn.apply(t, arguments);fn = null;return x;}}}} : function () {{}};\n      f = false;\n      return r;\n    }} else {{\n      var q = f ? {second} : function () {{}};\n      f = false;\n      return q;\n    }}\n  }};\n}}();\n{READER}"
        )
    };
    let one_line = "function () {if (fn) {var y = fn.apply(t, arguments);fn = null;return y;}}";
    let across = "function () {\n        if (fn) {\n          var y = fn.apply(t, arguments);\n          fn = null;\n          return y;\n        }\n      }";
    assert_eq!(source_readers(&two(one_line)), SourceReaders::Guarded);
    assert_eq!(source_readers(&two(across)), SourceReaders::Unguarded);
}

#[test]
fn an_expression_arrow_reader_is_seen() {
    let arrow = "var n = c(this, () => n.toString().search(\"(((.+)+)+)+$\").toString().constructor(n).search(\"(((.+)+)+)+$\"));\nn();\n";
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ACROSS_LINES}{arrow}")),
        SourceReaders::Unguarded
    );
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ONE_LINE}{arrow}")),
        SourceReaders::Guarded
    );
}

#[test]
fn a_reader_with_a_trailing_argument_is_seen() {
    let extra = READER.replacen("\n});", "\n}, 0);", 1);
    assert_eq!(
        source_readers(&format!("{CONTROLLER_ACROSS_LINES}{extra}")),
        SourceReaders::Unguarded
    );
}

fn escaped_helper(escape: &str) -> String {
    format!("var F = function (s) {{\n  var a = \"\";\n  var c = a + F;\n  var z = (\"\" + {escape}).indexOf(\"\\n\") !== -1;\n  return z || c.charCodeAt(s);\n}};\n")
}

#[test]
fn an_escape_that_prints_on_one_line_leaves_the_self_read_live() {
    for escape in [
        "function () {}",
        "(() => 0)",
        "function () {return \"dev\";}",
    ] {
        assert_eq!(
            source_readers(&escaped_helper(escape)),
            SourceReaders::Unguarded,
            "{escape}"
        );
    }
    let one_line = "var F = function (s) {var a = \"\";var c = a + F;var z = (\"\" + function () {}).indexOf(\"\\n\") !== -1;return z || c.charCodeAt(s);};\n";
    assert_eq!(source_readers(one_line), SourceReaders::Guarded);
}

#[test]
fn a_self_read_through_an_assignment_is_seen() {
    let assigned =
        "var F = function (s) {\n  var a = \"\", c;\n  c = a + F;\n  return c.charCodeAt(s);\n};\n";
    assert_eq!(source_readers(assigned), SourceReaders::Unguarded);
}

#[test]
fn a_direct_eval_that_may_name_the_controller_unguards_it() {
    let harmless = format!("var host = eval(\"require;\");\n{CONTROLLER_ONE_LINE}{READER}");
    assert_eq!(source_readers(&harmless), SourceReaders::Guarded);
    let naming = format!("{CONTROLLER_ONE_LINE}eval(\"c = 1\");\n{READER}");
    assert_eq!(source_readers(&naming), SourceReaders::Unguarded);
    let computed = format!("{CONTROLLER_ONE_LINE}eval(text);\n{READER}");
    assert_eq!(source_readers(&computed), SourceReaders::Unguarded);
    let with = format!("{CONTROLLER_ONE_LINE}with (o) {{\n  x;\n}}\n{READER}");
    assert_eq!(source_readers(&with), SourceReaders::Unguarded);
}
