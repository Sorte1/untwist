use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::layers::INLINE_LAYER_PASS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/prototype_reach_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn run(source: &str) -> Output {
    Pipeline::new(Options::default()).run(source)
}

#[test]
fn a_prototype_reached_through_a_pattern_or_a_spelled_key_keeps_the_eval() {
    let programs = rows();
    assert!(programs.len() >= 9, "found {} rows", programs.len());
    let peeled: Vec<&str> = programs
        .iter()
        .filter(|(label, _)| !label.starts_with("control-"))
        .filter(|(_, source)| {
            run(source)
                .stages
                .iter()
                .any(|stage| stage.stage == INLINE_LAYER_PASS)
        })
        .map(|(label, _)| label.as_str())
        .collect();
    assert!(peeled.is_empty(), "{peeled:#?}");
}

#[test]
fn a_prototype_reached_through_a_pattern_or_a_spelled_key_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run_safe = |source: &str| run(source).code().to_string();
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
