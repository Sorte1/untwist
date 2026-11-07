use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/straight_line_rows.txt");
const PREFIXES: [&str; 3] = ["dec-", "ref-", "guard-"];

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

fn folded(code: &str) -> bool {
    !code.contains("test")
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
fn each_rows_fold_decision_matches_its_label_in_both_lanes() {
    for lane in [false, true] {
        for (label, source) in rows() {
            if lane && label.starts_with("ref-text-") {
                continue;
            }
            let should_fold = label.starts_with("dec-");
            let code = transformed(&options(lane), &source);
            assert_eq!(
                folded(&code),
                should_fold,
                "{label} (unsafe={lane}): expected fold={should_fold}:\n{code}"
            );
        }
    }
}

#[test]
fn the_reported_sample_prints_one_plain_call() {
    let (_, source) = rows()
        .into_iter()
        .find(|(label, _)| label == "dec-the-reported-sample")
        .expect("the reported sample row");
    let code = transformed(&options(false), &source);
    assert!(
        code.contains(r#"console.log("hello", "world", 3);"#),
        "{code}"
    );
    assert!(!code.contains("counter"), "{code}");
}

#[test]
fn a_module_folds_like_a_script() {
    let source = "export {}; let counter = 0; let test = (val) => { counter += val; return val; }; console.log(test(1), test(2), counter);";
    let code = transformed(&options(false), source);
    assert!(code.contains("console.log(1, 2, 3);"), "{code}");
}

#[test]
fn both_lanes_run_like_node_where_no_code_comes_from_text_in_the_unsafe_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 25, "found {} rows", programs.len());
    let safe = options(false);
    let aggressive = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let mut wrong = node_rows::mismatches(&node, &programs, &modes);
    let untexted: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| !label.starts_with("ref-text-"))
        .collect();
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    wrong.extend(node_rows::mismatches(&node, &untexted, &modes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
