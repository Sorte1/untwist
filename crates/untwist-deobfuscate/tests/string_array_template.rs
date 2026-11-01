use untwist_testing::assert_transform;

#[test]
fn a_template_literal_element_is_still_a_string_array() {
    assert_transform!(
        "function a() {\n  var h = [`log`, 'x'];\n  a = function () { return h; };\n  return a();\n}\na();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function __STRING_ARRAY__() {
      var h = [`log`, "x"];
      __STRING_ARRAY__ = function () {
        return h;
      };
      return __STRING_ARRAY__();
    }
    __STRING_ARRAY__();
    "#
    );
}

#[test]
fn a_template_literal_element_is_inlined_as_a_string_literal() {
    assert_transform!(
        "const arr = [`log`, `Hello, World!`];\nconsole[arr[0]](arr[1]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"console["log"]("Hello, World!");"#
    );
}

#[test]
fn an_interpolated_template_element_is_not_a_string_array() {
    assert_transform!(
        "const arr = [`a${b}c`];\nconsole.log(arr[0]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    const arr = [`a${b}c`];
    console.log(arr[0]);
    "#
    );
}

#[test]
fn a_tagged_template_element_is_not_a_string_array() {
    assert_transform!(
        "const arr = [tag`a`];\nconsole.log(arr[0]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    const arr = [tag`a`];
    console.log(arr[0]);
    "#
    );
}

#[test]
fn a_lone_surrogate_template_element_is_not_a_string_array() {
    assert_transform!(
        "const arr = [`\\ud800`];\nconsole.log(arr[0]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    const arr = [`\ud800`];
    console.log(arr[0]);
    "#
    );
}
