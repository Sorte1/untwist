use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_testing::node::run;
use untwist_testing::node_rows::{node_or_skip, observed};

const ROWS: &str = include_str!("data/builtin_import_rows.txt");
const CALLEE: &str = "b(";

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

fn transformed(unsafe_transforms: bool, source: &str) -> String {
    Pipeline::new(options(unsafe_transforms))
        .run(source)
        .code()
        .to_string()
}

fn decoded(code: &str) -> bool {
    !code.contains(CALLEE)
}

fn printed_as_module(node: &Path, dir: &Path, text: &str) -> String {
    let path = dir.join("program.mjs");
    std::fs::write(&path, text).unwrap();
    let out = run(node, &path).unwrap().expect("finishes");
    observed(&out)
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            ["dec-", "kept-", "guard-"]
                .iter()
                .any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn the_safe_lane_decodes_only_the_rows_whose_imports_load_a_codeless_builtin() {
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        let code = transformed(false, &source);
        assert_eq!(
            decoded(&code),
            should_decode,
            "{label}: expected decode={should_decode}:\n{code}"
        );
    }
}

#[test]
fn every_row_prints_like_node_in_the_safe_lane_and_every_decoded_row_in_both() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for (label, source) in rows() {
        let before = printed_as_module(&node, dir.path(), &source);
        let lanes: &[bool] = if label.starts_with("dec-") {
            &[false, true]
        } else {
            &[false]
        };
        for lane in lanes {
            let after = printed_as_module(&node, dir.path(), &transformed(*lane, &source));
            if after != before {
                wrong.push(format!(
                    "{label} (unsafe={lane}): node printed {before:?}, the output printed {after:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
