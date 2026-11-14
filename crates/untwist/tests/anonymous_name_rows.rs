use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, listed_unsafe_witnesses, node_or_skip, Mode};

const ROWS: &str = include_str!("data/anonymous_name_rows.txt");

const UNSAFE_WITNESSES: [&str; 16] = [
    "var-function",
    "var-class",
    "let-arrow",
    "const-async-arrow",
    "assign-function",
    "for-init",
    "generator",
    "destructured-name",
    "name-read-in-eval-text",
    "name-read-in-function-text",
    "paren-assign-function",
    "paren-assign-class",
    "paren-assign-arrow",
    "paren-assign-comment",
    "paren-nested",
    "paren-in-value",
];

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

#[test]
fn an_anonymous_function_behind_a_comma_keeps_its_name_where_node_reads_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 12, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (unexplained, witnesses) = listed_unsafe_witnesses(wrong, &UNSAFE_WITNESSES);
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert_eq!(witnesses, UNSAFE_WITNESSES);
}

#[test]
fn a_program_that_reads_no_name_still_has_its_sequence_split() {
    let safe = Options::default();
    for (label, source) in rows()
        .iter()
        .filter(|(label, _)| label.starts_with("no-name-read"))
    {
        let code = transformed(&safe, source);
        assert!(!code.contains("(console.log"), "{label}: {code}");
    }
}
