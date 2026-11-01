use untwist_testing::assert_transform;

#[test]
fn the_three_statement_form_is_stamped() {
    assert_transform!(
        "function _0x3bd1() {\n  var _0xa4ed2e = ['log', void 0];\n  _0x3bd1 = function () { return _0xa4ed2e; };\n  return _0x3bd1();\n}\n_0x3bd1();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function __STRING_ARRAY__() {
      var _0xa4ed2e = ["log", void 0];
      __STRING_ARRAY__ = function () {
        return _0xa4ed2e;
      };
      return __STRING_ARRAY__();
    }
    __STRING_ARRAY__();
    "#
    );
}

#[test]
fn the_two_statement_form_is_stamped() {
    assert_transform!(
        "var a = function () {\n  var h = ['log'];\n  return (a = function () { return h; })();\n};\na();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var __STRING_ARRAY__ = function () {
      var h = ["log"];
      return (__STRING_ARRAY__ = function () {
        return h;
      })();
    };
    __STRING_ARRAY__();
    "#
    );
}

#[test]
fn a_non_string_element_is_not_a_string_array() {
    assert_transform!(
        "function a() {\n  var h = ['log', 1];\n  a = function () { return h; };\n  return a();\n}\na();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function a() {
      var h = ["log", 1];
      a = function () {
        return h;
      };
      return a();
    }
    a();
    "#
    );
}

#[test]
fn stamping_is_idempotent() {
    assert_transform!(
        "function __STRING_ARRAY__() {\n  var h = ['log'];\n  __STRING_ARRAY__ = function () { return h; };\n  return __STRING_ARRAY__();\n}\n__STRING_ARRAY__();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function __STRING_ARRAY__() {
      var h = ["log"];
      __STRING_ARRAY__ = function () {
        return h;
      };
      return __STRING_ARRAY__();
    }
    __STRING_ARRAY__();
    "#
    );
}

const WRAPPED: &str =
    "function a() { var h = ['log']; a = function () { return h; }; return a(); }\n";

fn stamped(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::string_array::boxed])
}

fn filler(count: usize) -> String {
    (0..count).map(|index| format!("o.k{index};\n")).collect()
}

#[test]
fn an_unspelled_array_is_stamped_whatever_precedes_it() {
    for count in [0, 2000] {
        let code = stamped(&format!("{}{WRAPPED}a();", filler(count)));
        assert!(code.contains("__STRING_ARRAY__"), "{count}");
    }
}

#[test]
fn a_spelling_before_or_after_the_array_leaves_it_unstamped_however_far_away() {
    let spelling = "eval('a().reverse()');\n";
    let shapes = [
        format!("{spelling}{WRAPPED}"),
        format!("{WRAPPED}{spelling}"),
        format!("{}{spelling}{WRAPPED}", filler(2000)),
        format!("{spelling}{}{WRAPPED}", filler(2000)),
        format!("{WRAPPED}{}{spelling}", filler(2000)),
        format!("{}{WRAPPED}{}{spelling}", filler(2000), filler(10)),
    ];
    for shape in &shapes {
        let code = stamped(shape);
        assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    }
}

#[test]
fn a_local_array_is_reachable_by_a_spelled_name_only_under_a_direct_eval() {
    let local = format!("(function () {{\n{WRAPPED}o.a;\na();\n}})();");
    assert!(stamped(&local).contains("__STRING_ARRAY__"));
    let far = format!("var s = 'a().reverse()';\n(function () {{\n{WRAPPED}a();\n}})();");
    assert!(stamped(&far).contains("__STRING_ARRAY__"));
    let evaluated =
        format!("var s = 'a().reverse()';\n(function () {{\n{WRAPPED}eval(s);\na();\n}})();");
    assert!(!stamped(&evaluated).contains("__STRING_ARRAY__"));
    let nested = format!(
        "var s = 'a().reverse()';\n(function () {{\n{WRAPPED}(function () {{ eval(s); }})();\na();\n}})();"
    );
    assert!(!stamped(&nested).contains("__STRING_ARRAY__"));
}
