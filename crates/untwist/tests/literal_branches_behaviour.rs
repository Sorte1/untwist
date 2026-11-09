use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const FIXTURE_COUNT: usize = 13;

fn default_lane(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = js_files_in("fixtures/literal-branches");
    assert_eq!(fixtures.len(), FIXTURE_COUNT);
    fixtures
}

fn is_kept(label: &str) -> bool {
    label
        .rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("kept-"))
}

#[test]
fn every_literal_branch_fixture_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 1] = [("default", &default_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn dead_branches_are_gone_from_every_fixture_that_is_not_kept() {
    for (label, source) in fixtures().into_iter().filter(|(label, _)| !is_kept(label)) {
        let code = default_lane(&source);
        assert!(!code.contains("dead"), "{label}:\n{code}");
    }
}

#[test]
fn the_user_sample_loses_its_dead_block() {
    let code = default_lane("if (false) {\n  while (true) {}\n}\nconsole.log(1);\n");
    assert!(!code.contains("while"), "{code}");
    assert!(!code.contains("if"), "{code}");
}

#[test]
fn a_function_declared_in_a_dead_block_keeps_its_branch() {
    let code = default_lane("function f() {\n  if (false) {\n    function g() {}\n  }\n}\n");
    assert!(code.contains("function g"), "{code}");
}

#[test]
fn a_labelled_dead_branch_is_kept() {
    let code = default_lane("lbl: if (false) {\n  break lbl;\n}\nlbl2: while (false) {}\n");
    assert!(code.contains("lbl: if (false)"), "{code}");
    assert!(code.contains("lbl2: while (false)"), "{code}");
}

#[test]
fn a_reference_picked_by_a_constant_test_stays_wrapped() {
    let code = default_lane("(true ? o.f : g)();\n(0 || o.f)();\n");
    assert!(code.contains("true ? o.f : g"), "{code}");
    assert!(code.contains("0 || o.f"), "{code}");
}

#[test]
fn the_counter_sample_keeps_no_empty_statement() {
    let source = "let counter = 0;\nlet test = val => {\n  counter += val;\n  return [\"hello\", \"world\", \"log\"][val];\n};\nif (false) {\n  while (true) {}\n}\n;\nconsole[test(2)](test(0), test(1), counter);\n";
    assert_eq!(
        default_lane(source),
        "console.log(\"hello\", \"world\", 3);"
    );
}
