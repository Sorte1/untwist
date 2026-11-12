use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/shared_global_rows.txt");
const INPUT_BREAK: &str = " @@ ";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn joined(program: &str) -> String {
    program.replace(INPUT_BREAK, "\n")
}

fn run(options: Options, program: &str) -> String {
    let inputs: Vec<&str> = program.split(INPUT_BREAK).collect();
    Pipeline::new(options).run_many(&inputs).codes.join("\n")
}

fn lane(unsafe_transforms: bool, rename: RenameMode) -> Options {
    Options {
        unsafe_transforms,
        rename,
        unpack: false,
        ..Options::default()
    }
}

#[test]
fn every_row_holds_two_inputs() {
    for (label, program) in rows() {
        assert_eq!(program.split(INPUT_BREAK).count(), 2, "{label}");
    }
}

#[test]
fn several_classic_scripts_share_their_top_level_bindings() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .map(|(label, program)| (label, joined(&program)))
        .collect();
    let originals = rows();
    let split = |joined_program: &str| {
        originals
            .iter()
            .find(|(_, program)| joined(program) == joined_program)
            .map(|(_, program)| program.clone())
            .expect("a known row")
    };
    let default = |source: &str| run(lane(false, RenameMode::None), &split(source));
    let unsafe_lane = |source: &str| run(lane(true, RenameMode::None), &split(source));
    let simple = |source: &str| run(lane(false, RenameMode::Simple), &split(source));
    let unique = |source: &str| run(lane(false, RenameMode::Unique), &split(source));
    let mangle = |source: &str| run(lane(false, RenameMode::Mangle), &split(source));
    let modes: [Mode<'_>; 5] = [
        ("default", &default),
        ("unsafe", &unsafe_lane),
        ("rename simple", &simple),
        ("rename unique", &unique),
        ("rename mangle", &mangle),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn inputs_that_share_no_names_come_out_as_they_do_alone() {
    let options = lane(false, RenameMode::None);
    for (label, program) in rows() {
        if !label.starts_with("apart-") {
            continue;
        }
        let inputs: Vec<&str> = program.split(INPUT_BREAK).collect();
        let together = Pipeline::new(options.clone()).run_many(&inputs).codes;
        let apart: Vec<String> = inputs
            .iter()
            .flat_map(|input| Pipeline::new(options.clone()).run(input).codes)
            .collect();
        assert_eq!(together, apart, "{label}");
    }
}
