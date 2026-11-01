use untwist_testing::assert_transform;
use untwist_unminify::computed_properties::boxed;

#[test]
fn member_expression() {
    assert_transform!(
        r#"require("foo")["default"]?.["_"];"#,
        [boxed],
        @r#"require("foo").default?._;"#
    );
}

#[test]
fn object() {
    assert_transform!(r#"const x = { ["foo"](){}, ["bar"]: 1 };"#, [boxed], @r"
    const x = {
      foo() {},
      bar: 1
    };
    ");
}

#[test]
fn class() {
    assert_transform!(r#"class Foo { ["foo"](){} ["bar"] = 1; }"#, [boxed], @r"
    class Foo {
      foo() {}
      bar = 1;
    }
    ");
}

#[test]
fn ignore_invalid_identifier() {
    assert_transform!(
        r#"console["1"]("hello");"#,
        [boxed],
        @r#"console["1"]("hello");"#
    );
}

#[test]
fn ignore_proto_property() {
    assert_transform!(
        r#"const x = { ["__proto__"]: 1 };"#,
        [boxed],
        @r#"const x = { ["__proto__"]: 1 };"#
    );
}

#[test]
fn assignment_target() {
    assert_transform!(r#"a["class"] = b;"#, [boxed], @"a.class = b;");
}

#[test]
fn ignore_constructor_method() {
    assert_transform!(r#"class Foo { ["constructor"](){} }"#, [boxed], @r#"
    class Foo {
      ["constructor"]() {}
    }
    "#);
}

#[test]
fn ignore_constructor_field() {
    assert_transform!(r#"class A { ["constructor"] = 1; }"#, [boxed], @r#"
    class A {
      ["constructor"] = 1;
    }
    "#);
}

#[test]
fn ignore_static_prototype_method() {
    assert_transform!(r#"class B { static ["prototype"]() {} }"#, [boxed], @r#"
    class B {
      static ["prototype"]() {}
    }
    "#);
}

#[test]
fn uncompute_other_class_method() {
    assert_transform!(r#"class C { ["x"]() {} }"#, [boxed], @r"
    class C {
      x() {}
    }
    ");
}

#[test]
fn uncompute_constructor_object_property() {
    assert_transform!(r#"({ ["constructor"]: 1 });"#, [boxed], @"({ constructor: 1 });");
}

#[test]
fn uncompute_non_static_prototype_method() {
    assert_transform!(r#"class A { ["prototype"]() {} }"#, [boxed], @r"
    class A {
      prototype() {}
    }
    ");
}

#[test]
fn uncompute_prototype_object_property() {
    assert_transform!(r#"({ ["prototype"]: 1 });"#, [boxed], @"({ prototype: 1 });");
}

#[test]
fn update_expression_target() {
    assert_transform!(r#"a["pos"]++;"#, [boxed], @"a.pos++;");
}

#[test]
fn prefix_update_expression_target() {
    assert_transform!(r#"--this["_nActive"];"#, [boxed], @"--this._nActive;");
}

#[test]
fn ignore_invalid_identifier_update_target() {
    assert_transform!(r#"a["1"]++;"#, [boxed], @r#"a["1"]++;"#);
}
