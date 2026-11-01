use untwist_core::pass::PassOptions;
use untwist_testing::{assert_transform, transform_report, Allocator, PassCtorAlias};

#[test]
fn issue_230_shape_1_a_plain_array_alias_is_resolved() {
    assert_transform!(
        "var arr = ['log', 'Hello, World!'];\nvar arr2 = arr;\nconsole[arr2[0]](arr2[1]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"console["log"]("Hello, World!");"#
    );
}

#[test]
fn issue_230_shape_2_an_alias_in_a_second_declaration_is_resolved() {
    assert_transform!(
        "var string_array = ['log', 'Hello World'];\nconst str = string_array;\nconsole[str[0]](str[1]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"console["log"]("Hello World");"#
    );
}

#[test]
fn a_chain_of_aliases_is_resolved() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nvar c = b;\nconsole[c[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"console["log"]();"#
    );
}

#[test]
fn an_alias_that_escapes_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nfn(b);\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var b = a;
    fn(b);
    console[b[0]]();
    "#
    );
}

#[test]
fn a_mutated_alias_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nlet b = a;\nb = other;\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    let b = a;
    b = other;
    console[b[0]]();
    "#
    );
}

#[test]
fn a_self_alias_does_not_loop() {
    assert_transform!(
        "var a = a;\nconsole.log(a);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = a;
    console.log(a);
    "#
    );
}

#[test]
fn an_alias_written_through_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nb[0] = 'warn';\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var b = a;
    b[0] = "warn";
    console[b[0]]();
    "#
    );
}

#[test]
fn a_redeclared_alias_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nvar b = other;\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var b = a;
    var b = other;
    console[b[0]]();
    "#
    );
}

#[test]
fn an_alias_of_a_shadowed_array_name_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nconsole[b[0]]();\nfunction f() {\n  var a = 1;\n  return a;\n}",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var b = a;
    console[b[0]]();
    function f() {
      var a = 1;
      return a;
    }
    "#
    );
}

const REVERSE_CHAIN_LINKS: usize = 17;

fn a_reverse_chain_longer_than_the_cap() -> String {
    let last = REVERSE_CHAIN_LINKS;
    let mut source = String::new();
    for index in (1..=last).rev() {
        let previous = index - 1;
        source.push_str(&format!("var a{index} = a{previous};\n"));
    }
    source.push_str("var a0 = ['log'];\n");
    source.push_str(&format!("console[a{last}[0]]();\n"));
    source
}

#[test]
fn a_reverse_declared_chain_is_refused_without_churning_the_worklist() {
    let source = a_reverse_chain_longer_than_the_cap();
    let alloc = Allocator::default();
    let ctors: &[PassCtorAlias<'_>] = &[untwist_deobfuscate::string_array::boxed];
    let (code, report) = transform_report(
        &alloc,
        &source,
        ctors,
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    assert!(report.converged(), "{:?}", report.stop);
    assert!(code.contains("var a0 = [\"log\"];"), "{code}");
    assert!(code.contains("var a1 = a0;"), "{code}");
    assert!(code.contains("var a17 = a16;"), "{code}");
    assert!(code.contains("console[a17[0]]();"), "{code}");
}

#[test]
fn an_alias_declared_outside_a_statement_list_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nif (x) var b = a;\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    if (x) var b = a;
    console[b[0]]();
    "#
    );
}

#[test]
fn an_alias_of_a_reassigned_array_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\na = other;\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var b = a;
    a = other;
    console[b[0]]();
    "#
    );
}

#[test]
fn an_alias_of_a_redeclared_array_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nvar a = other;\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var b = a;
    var a = other;
    console[b[0]]();
    "#
    );
}

#[test]
fn an_alias_of_an_array_rebound_inside_a_function_is_left_alone() {
    assert_transform!(
        "var a = ['log', 'x'];\nvar b = a;\nfunction f() {\n  a = other;\n}\nf();\nconsole[b[0]](b[1]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log", "x"];
    var b = a;
    function f() {
      a = other;
    }
    f();
    console[b[0]](b[1]);
    "#
    );
}

#[test]
fn an_alias_declared_in_a_nested_block_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nif (x) {\n  var b = a;\n}\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    if (x) {
      var b = a;
    }
    console[b[0]]();
    "#
    );
}

#[test]
fn an_alias_that_precedes_a_let_array_is_left_alone() {
    assert_transform!(
        "var b = a;\nlet a = ['log'];\nconsole.log(1);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var b = a;
    let a = ["log"];
    console.log(1);
    "#
    );
}

#[test]
fn an_alias_that_precedes_a_var_array_is_left_alone() {
    assert_transform!(
        "var b = a;\nvar a = ['log'];\nconsole[b[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var b = a;
    var a = ["log"];
    console[b[0]]();
    "#
    );
}

#[test]
fn an_alias_of_a_forward_declared_alias_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar c = b;\nvar b = a;\nconsole[c[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var c = b;
    var b = a;
    console[c[0]]();
    "#
    );
}

#[test]
fn an_alias_of_a_forward_declared_let_alias_is_left_alone() {
    assert_transform!(
        "var a = ['log'];\nvar c = b;\nlet b = a;\nconsole[c[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var a = ["log"];
    var c = b;
    let b = a;
    console[c[0]]();
    "#
    );
}

#[test]
fn a_forward_chain_of_three_aliases_is_resolved() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nvar c = b;\nvar d = c;\nconsole[d[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"console["log"]();"#
    );
}

#[test]
fn an_alias_read_inside_a_hoisted_function_that_can_run_first_is_left_alone() {
    assert_transform!(
        "function g() { return b[0]; }\nconsole.log(g());\nvar a = ['log'];\nvar b = a;",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function g() {
      return b[0];
    }
    console.log(g());
    var a = ["log"];
    var b = a;
    "#
    );
}

#[test]
fn an_alias_read_inside_a_function_declared_after_an_inert_prefix_is_resolved() {
    assert_transform!(
        "var a = ['log'];\nvar b = a;\nfunction g() { return b[0]; }\nconsole.log(g());",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function g() {
      return "log";
    }
    console.log(g());
    "#
    );
}
