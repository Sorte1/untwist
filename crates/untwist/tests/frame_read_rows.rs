use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/frame_read_rows.txt");
const KEEPS: &str = include_str!("data/frame_read_keeps_rows.txt");

fn rows() -> Vec<(String, String)> {
    parse(ROWS)
}

fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn run(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn a_caller_read_keeps_the_calling_function_a_function() {
    let source = rows()
        .into_iter()
        .find(|(label, _)| label == "ref-caller-sees-the-calling-function")
        .map(|(_, source)| source)
        .expect("row");
    let code = run(false, &source);
    assert!(!code.contains("=>"), "{code}");
    let unsafe_code = run(true, &source);
    assert!(unsafe_code.contains("=>"), "{unsafe_code}");
}

#[test]
fn function_frame_reads_run_like_node_in_the_safe_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let default = |source: &str| run(false, source);
    let modes: [Mode<'_>; 1] = [("default", &default)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_key_that_names_another_property_still_lets_a_callee_become_an_arrow() {
    for (label, source) in parse(KEEPS) {
        let code = run(false, &source);
        assert!(code.contains("=>"), "{label}: {code}");
    }
    let Some(node) = node_or_skip() else {
        return;
    };
    let default = |source: &str| run(false, source);
    let modes: [Mode<'_>; 1] = [("default", &default)];
    let wrong = node_rows::mismatches(&node, &parse(KEEPS), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
