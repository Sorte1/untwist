use untwist_testing::assert_transform;

#[test]
fn inline_property() {
    assert_transform!(
        "const a = { x: 1 };\nconsole.log(a.x);",
        [untwist_deobfuscate::object_props::boxed],
        @"console.log(1);"
    );
}

#[test]
fn inline_property_of_object_literal() {
    assert_transform!(
        "console.log(({ x: 1 }).x);",
        [untwist_deobfuscate::object_props::boxed],
        @"console.log(1);"
    );
}

#[test]
fn ignore_non_existent_properties() {
    assert_transform!(
        "const a = { x: 1 };\nconsole.log(a.__defineGetter__);",
        [untwist_deobfuscate::object_props::boxed],
        @"
    const a = { x: 1 };
    console.log(a.__defineGetter__);
    "
    );
}

#[test]
fn ignore_shared_variable_references() {
    assert_transform!(
        "const a = { x: 1 };\nfn(a);\nconsole.log(a.x);",
        [untwist_deobfuscate::object_props::boxed],
        @"
    const a = { x: 1 };
    fn(a);
    console.log(a.x);
    "
    );
}

#[test]
fn ignore_variable_assignment() {
    assert_transform!(
        "let a = { x: 1 };\na = { x: 2 };\nconsole.log(a.x);",
        [untwist_deobfuscate::object_props::boxed],
        @"
    let a = { x: 1 };
    a = { x: 2 };
    console.log(a.x);
    "
    );
}

#[test]
fn ignore_property_assignment_delete_and_update() {
    for source in [
        "const a = { x: 1 };\na.x = 2;\nconsole.log(a.x);",
        "const a = { x: 1 };\ndelete a.x;\nconsole.log(a.x);",
        "const a = { x: 1 };\na.x++;\nconsole.log(a.x);",
    ] {
        let alloc = untwist_testing::Allocator::default();
        let out =
            untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::object_props::boxed]);
        assert!(out.contains("a.x"), "{source} -> {out}");
    }
}

#[test]
fn ignore_destructuring_targets() {
    for source in [
        "let a = { x: 1 };\n[a.x] = [2];\nconsole.log(a.x);",
        "let a = { x: 1 };\n({ x: a.x } = { x: 2 });\nconsole.log(a.x);",
    ] {
        let alloc = untwist_testing::Allocator::default();
        let out =
            untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::object_props::boxed]);
        assert!(out.contains("a.x"), "{source} -> {out}");
    }
}

#[test]
fn the_calls_transform_argument_object_is_inlined() {
    assert_transform!(
        "function foo() {\n  var k = { c: 0x2f2, d: '0x396' };\n  return b(k.d, k.c);\n}",
        [untwist_deobfuscate::object_props::boxed],
        @r#"
    function foo() {
      return b("0x396", 754);
    }
    "#
    );
}

#[test]
fn a_duplicate_key_in_a_declared_object_is_refused() {
    assert_transform!(
        "const o = { k: 1, k: 2 };\nconsole.log(o.k);",
        [untwist_deobfuscate::object_props::boxed],
        @"
    const o = {
      k: 1,
      k: 2
    };
    console.log(o.k);
    "
    );
}

#[test]
fn a_duplicate_key_spelled_two_ways_is_refused() {
    assert_transform!(
        "const o = { 1: 'a', '1': 'b' };\nconsole.log(o[1]);",
        [untwist_deobfuscate::object_props::boxed],
        @r#"
    const o = {
      1: "a",
      "1": "b"
    };
    console.log(o[1]);
    "#
    );
}

#[test]
fn a_duplicate_key_in_an_object_literal_read_is_refused() {
    assert_transform!(
        "console.log(({ k: 1, k: 2 }).k);",
        [untwist_deobfuscate::object_props::boxed],
        @"
    console.log({
      k: 1,
      k: 2
    }.k);
    "
    );
}

#[test]
fn a_proto_key_is_refused() {
    assert_transform!(
        "const o = { __proto__: 'x' };\nconsole.log(o.__proto__);",
        [untwist_deobfuscate::object_props::boxed],
        @r#"
    const o = { __proto__: "x" };
    console.log(o.__proto__);
    "#
    );
}

#[test]
fn a_var_object_read_above_its_own_declaration_is_refused() {
    let alloc = untwist_testing::Allocator::default();
    let out = untwist_testing::transform(
        &alloc,
        "console.log(m.x);\nvar m = { x: 1 };",
        &[untwist_deobfuscate::object_props::boxed],
    );
    assert!(out.contains("m.x"), "{out}");
    assert!(out.contains("var m"), "{out}");
}

#[test]
fn a_const_object_read_in_its_temporal_dead_zone_is_refused() {
    let alloc = untwist_testing::Allocator::default();
    let out = untwist_testing::transform(
        &alloc,
        "console.log(m.x);\nconst m = { x: 1 };",
        &[untwist_deobfuscate::object_props::boxed],
    );
    assert!(out.contains("m.x"), "{out}");
    assert!(out.contains("const m"), "{out}");
}

#[test]
fn a_read_inside_a_hoisted_function_above_the_declaration_is_still_inlined() {
    assert_transform!(
        "function f() { return m.x; }\nvar m = { x: 1 };\nconsole.log(f());",
        [untwist_deobfuscate::object_props::boxed],
        @"
    function f() {
      return 1;
    }
    console.log(f());
    "
    );
}

#[test]
fn an_object_read_after_its_declaration_is_still_inlined() {
    assert_transform!(
        "var m = { x: 1 };\nconsole.log(m.x);",
        [untwist_deobfuscate::object_props::boxed],
        @"console.log(1);"
    );
}
