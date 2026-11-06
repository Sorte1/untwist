use untwist_testing::{assert_transform, transform, Allocator};

const ARRAY: &str = "function __STRING_ARRAY__() {\n  var h = ['log'];\n  __STRING_ARRAY__ = function () { return h; };\n  return __STRING_ARRAY__();\n}\n";

fn stamped(source: &str) -> String {
    let alloc = Allocator::default();
    transform(&alloc, source, &[untwist_deobfuscate::decoder::boxed])
}

#[test]
fn a_decoder_inside_a_function_expression_is_stamped() {
    let code = stamped(&format!(
        "(function () {{\n{ARRAY}function b(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nb(0);\n}})();"
    ));
    assert!(code.contains("__DECODE_0__"), "{code}");
}

#[test]
fn a_decoder_inside_an_arrow_body_is_stamped() {
    let code = stamped(&format!(
        "const z = () => {{\n{ARRAY}function b(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nreturn b(0);\n}};"
    ));
    assert!(code.contains("__DECODE_0__"), "{code}");
}

#[test]
fn a_decoder_inside_a_class_method_is_stamped() {
    let code = stamped(&format!(
        "class K {{\n  m() {{\n{ARRAY}function b(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\n    return b(0);\n  }}\n}}"
    ));
    assert!(code.contains("__DECODE_0__"), "{code}");
}

#[test]
fn an_existing_stamp_is_kept_and_the_new_decoder_takes_the_next_free_number() {
    let source = format!(
        "{ARRAY}function b(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nfunction __DECODE_0__(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nb(0);\n__DECODE_0__(1);"
    );
    let once = stamped(&source);
    assert!(once.contains("function __DECODE_1__(c, d)"), "{once}");
    assert!(once.contains("function __DECODE_0__(c, d)"), "{once}");
    assert!(!once.contains("__DECODE_0_2__"), "{once}");
    assert_eq!(stamped(&once), once, "a second run must change nothing");
}

#[test]
fn a_function_declaration_decoder_is_stamped() {
    assert_transform!(
        &format!("{ARRAY}function b(c, d) {{\n  var e = __STRING_ARRAY__();\n  b = function (f, g) {{ f = f - 0; var h = e[f]; return h; }};\n  return b(c, d);\n}}\nb(0);"),
        [untwist_deobfuscate::decoder::boxed],
        @r#"
    function __STRING_ARRAY__() {
      var h = ["log"];
      __STRING_ARRAY__ = function () {
        return h;
      };
      return __STRING_ARRAY__();
    }
    function __DECODE_0__(c, d) {
      var e = __STRING_ARRAY__();
      __DECODE_0__ = function (f, g) {
        f = f - 0;
        var h = e[f];
        return h;
      };
      return __DECODE_0__(c, d);
    }
    __DECODE_0__(0);
    "#
    );
}

#[test]
fn a_var_function_decoder_is_stamped_too() {
    assert_transform!(
        &format!("{ARRAY}var b = function (c, d) {{\n  var e = __STRING_ARRAY__();\n  return e[c];\n}};\nb(0);"),
        [untwist_deobfuscate::decoder::boxed],
        @r#"
    function __STRING_ARRAY__() {
      var h = ["log"];
      __STRING_ARRAY__ = function () {
        return h;
      };
      return __STRING_ARRAY__();
    }
    var __DECODE_0__ = function (c, d) {
      var e = __STRING_ARRAY__();
      return e[c];
    };
    __DECODE_0__(0);
    "#
    );
}

#[test]
fn three_decoders_are_numbered_in_source_order() {
    let source = format!(
        "{ARRAY}function b(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nfunction c2(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nfunction d2(c, d) {{ var e = __STRING_ARRAY__(); return e[c]; }}\nb(0); c2(0); d2(0);"
    );
    let out = stamped(&source);
    assert!(out.contains("function __DECODE_0__(c, d)"), "{out}");
    assert!(out.contains("function __DECODE_1__(c, d)"), "{out}");
    assert!(out.contains("function __DECODE_2__(c, d)"), "{out}");
}

#[test]
fn a_function_that_never_indexes_the_array_is_not_a_decoder() {
    assert_transform!(
        &format!("{ARRAY}function b(c) {{\n  var e = __STRING_ARRAY__();\n  return e;\n}}\nb(0);"),
        [untwist_deobfuscate::decoder::boxed],
        @r#"
    function __STRING_ARRAY__() {
      var h = ["log"];
      __STRING_ARRAY__ = function () {
        return h;
      };
      return __STRING_ARRAY__();
    }
    function b(c) {
      var e = __STRING_ARRAY__();
      return e;
    }
    b(0);
    "#
    );
}

#[test]
fn a_decoder_that_indexes_the_array_call_directly_is_stamped() {
    let code = stamped(&format!(
        "{ARRAY}function b(c, d) {{\n  c -= 415;\n  return __STRING_ARRAY__()[c];\n}}\nb(415);"
    ));
    assert!(code.contains("function __DECODE_0__(c, d)"), "{code}");
}

#[test]
fn a_function_that_calls_the_array_without_indexing_it_is_not_a_decoder() {
    let code = stamped(&format!(
        "{ARRAY}function b(c) {{\n  return __STRING_ARRAY__().length + c;\n}}\nb(0);"
    ));
    assert!(!code.contains("__DECODE_"), "{code}");
}

fn filler(count: usize) -> String {
    (0..count).map(|index| format!("o.k{index};\n")).collect()
}

const DECODER: &str = "function b(c, d) { var e = __STRING_ARRAY__(); return e[c]; }\n";

#[test]
fn a_spelling_of_the_decoder_leaves_it_unstamped_however_far_away() {
    let spelling = "o.b;\n";
    let shapes = [
        format!("{ARRAY}{spelling}{DECODER}b(0);"),
        format!("{ARRAY}{DECODER}{spelling}b(0);"),
        format!("{ARRAY}{}{spelling}{DECODER}b(0);", filler(2000)),
        format!("{ARRAY}{spelling}{}{DECODER}b(0);", filler(2000)),
        format!("{ARRAY}{DECODER}{}{spelling}b(0);", filler(2000)),
    ];
    for shape in &shapes {
        let code = stamped(shape);
        assert!(!code.contains("__DECODE_0__"), "{code}");
    }
    let unspelled = stamped(&format!("{ARRAY}{}{DECODER}b(0);", filler(2000)));
    assert!(unspelled.contains("__DECODE_0__"), "{unspelled}");
}

#[test]
fn a_decoder_written_outside_its_statement_is_left_unstamped() {
    let written = stamped(&format!("{ARRAY}{DECODER}b(0);\nb = 1;"));
    assert!(!written.contains("__DECODE_0__"), "{written}");
    let redeclared = stamped(&format!("{ARRAY}{DECODER}b(0);\nvar b = 1;"));
    assert!(!redeclared.contains("__DECODE_0__"), "{redeclared}");
    let self_write = stamped(&format!(
        "{ARRAY}function b(c, d) {{ var e = __STRING_ARRAY__(); b = function (f) {{ return e[f]; }}; return b(c); }}\nb(0);"
    ));
    assert!(self_write.contains("__DECODE_0__"), "{self_write}");
}

#[test]
fn a_nested_decoder_is_checked_by_its_own_scan() {
    let nested = |body: &str| {
        stamped(&format!(
            "(function () {{\n{ARRAY}{DECODER}{body}b(0);\n}})();"
        ))
    };
    assert!(nested("").contains("__DECODE_0__"));
    assert!(nested("o.b;\n").contains("__DECODE_0__"));
    assert!(!nested("eval(s);\nvar s = 'b(1)';\n").contains("__DECODE_0__"));
    assert!(!nested("b = 1;\n").contains("__DECODE_0__"));
    assert!(!nested("{ function b() {} }\n").contains("__DECODE_0__"));
}
