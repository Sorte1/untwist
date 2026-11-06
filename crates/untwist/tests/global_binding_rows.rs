use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/global_binding_rows.txt");
const PREFIXES: [&str; 4] = ["dec-", "ref-", "guard-", "kept-"];

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn decoded(code: &str) -> bool {
    !code.contains("b(k)")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            PREFIXES.iter().any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label() {
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        let code = transformed(&options(false), &source);
        assert_eq!(
            decoded(&code),
            should_decode,
            "{label}: expected decode={should_decode}:\n{code}"
        );
    }
}

#[test]
fn the_default_lane_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 30, "found {} rows", programs.len());
    let safe = options(false);
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_unsafe_lane_runs_like_node_where_no_code_comes_from_text() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| !label.starts_with("ref-text-"))
        .collect();
    let aggressive = options(true);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_module_decodes_a_top_level_variable_the_global_object_writes() {
    let (_, prelude) = rows()
        .into_iter()
        .find(|(label, _)| label == "dec-a-top-level-variable-nothing-reaches")
        .expect("the plain top-level row");
    let source = prelude.replace(
        "var k; k = 1; console.log(b(k));",
        "export {}; var k; k = 1; globalThis.k = 2; console.log(b(k));",
    );
    assert!(source.contains("export {}"), "the prelude changed shape");
    for lane in [false, true] {
        let code = transformed(&options(lane), &source);
        assert!(decoded(&code), "unsafe={lane}:\n{code}");
    }
}
