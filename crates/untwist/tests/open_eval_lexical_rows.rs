use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const OPEN: [(&str, &str); 7] = [
    (
        "eval-writes-a-var",
        r#"function f() { var a = 1; eval("a = a + 1"); return a; } console.log(f());"#,
    ),
    (
        "eval-writes-a-trailing-var",
        r#"function f() { a = 1; eval("a = a + 1"); return a; var a; } console.log(f());"#,
    ),
    (
        "eval-redeclares-a-var",
        r#"function f() { var a = 1; eval("var a = 2"); return a; } console.log(f());"#,
    ),
    (
        "eval-text-from-a-variable",
        r#"var code = "n = n * 3"; function f() { var n = 2; eval(code); return n; } console.log(f());"#,
    ),
    (
        "eval-reads-a-var-before-its-declaration",
        r#"function f() { var r = eval("a"); var a = 1; return [r, a].join(); } console.log(f());"#,
    ),
    (
        "eval-reads-a-var-after-its-declaration",
        r#"function f() { var a = 1; return eval("a + 1") + a; } console.log(f());"#,
    ),
    (
        "eval-writes-a-block-var",
        r#"function f(k) { if (k) { var a = 1; eval("a = 5"); return a; } return 0; } console.log(f(1));"#,
    ),
];

const NAMES_NOTHING: &str =
    r#"function f() { var a = 1; return eval("1 + 2") + a; } console.log(f());"#;

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

#[test]
fn a_direct_eval_that_writes_or_declares_keeps_its_vars_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = OPEN
        .iter()
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    let safe = options(false);
    let unsafe_transforms = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    for (_, program) in OPEN {
        for lane in [&safe, &unsafe_transforms] {
            let code = transformed(lane, program);
            assert!(!code.contains("const "), "{code}");
            assert!(!code.contains("let "), "{code}");
        }
    }
}

#[test]
fn a_direct_eval_that_names_no_binding_still_lets_its_var_become_const_under_unsafe() {
    let code = transformed(&options(true), NAMES_NOTHING);
    assert!(code.contains("const a = 1;"), "{code}");
    untwist_testing::node::assert_node_agrees(NAMES_NOTHING, &code);
    let safe = transformed(&options(false), NAMES_NOTHING);
    untwist_testing::node::assert_node_agrees(NAMES_NOTHING, &safe);
}
