use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const KEPT_CONDITIONALS: &[&str] = &["dropped-function.js"];

fn default_lane(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = js_files_in("fixtures/dead-code");
    assert_eq!(fixtures.len(), 9);
    fixtures
}

fn has_opaque_test(code: &str) -> bool {
    [
        "\"x\" === \"x\"",
        "\"x\" !== \"x\"",
        "\"a\" === \"a\"",
        "\"b\" === \"c\"",
    ]
    .iter()
    .any(|test| code.contains(test))
}

#[test]
fn every_dead_code_fixture_prints_what_node_prints_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let fixtures = fixtures();
    let runs: [Mode<'_>; 1] = [("default", &default_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures, &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_default_lane_folds_every_dead_code_fixture_but_the_refused_one() {
    for (label, source) in fixtures() {
        let code = default_lane(&source);
        let kept = KEPT_CONDITIONALS.iter().any(|name| label.ends_with(name));
        assert_eq!(has_opaque_test(&code), kept, "{label}:\n{code}");
    }
}

const A_DEAD_BRANCH_READ_THROUGH_A_CONTROL_FLOW_OBJECT: &str = "function f(g) {\n  var o = { 'abc': function (a, b) { return a(b); }, 'def': function (a, b) { return a !== b; } };\n  if (o['def'](\"a\", \"a\")) {\n    var x = g;\n    return o['abc'](x, 1);\n  } else {\n    return o['abc'](g, 2);\n  }\n}\nconsole.log(f(String));";

fn lanes(source: &str) -> Vec<String> {
    [false, true]
        .into_iter()
        .map(|unsafe_transforms| {
            Pipeline::new(Options {
                unsafe_transforms,
                ..Options::default()
            })
            .run(source)
            .code()
            .to_string()
        })
        .collect()
}

#[test]
fn a_dead_branch_var_read_only_inside_the_branch_leaves_no_declaration_behind() {
    for code in lanes(A_DEAD_BRANCH_READ_THROUGH_A_CONTROL_FLOW_OBJECT) {
        assert_eq!(
            code.trim(),
            "function f(g) {\n  return g(2);\n}\nconsole.log(f(String));"
        );
    }
}

#[test]
fn a_dead_branch_var_read_after_the_branch_keeps_its_declaration() {
    let source = A_DEAD_BRANCH_READ_THROUGH_A_CONTROL_FLOW_OBJECT.replace(
        "  } else {\n    return o['abc'](g, 2);\n  }\n}",
        "  }\n  return x;\n}",
    );
    for code in lanes(&source) {
        assert!(code.contains("let x;") || code.contains("var x;"), "{code}");
    }
}
