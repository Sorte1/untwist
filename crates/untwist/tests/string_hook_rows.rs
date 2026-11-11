use untwist::{Options, Pipeline};
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/string_hook_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(points_to: bool, source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        disabled_passes: if points_to {
            vec![BUILTIN_ALIAS_BITS]
        } else {
            Vec::new()
        },
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn decoded(code: &str) -> bool {
    !code.contains("console.log(dec0(")
}

#[test]
fn every_row_has_a_known_prefix() {
    let all = rows();
    assert!(all.len() >= 51, "found {} rows", all.len());
    for (label, _) in all {
        assert!(
            label.starts_with("dec-") || label.starts_with("ref-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn a_replaced_hook_the_decoder_consults_keeps_the_decoder() {
    for (label, source) in rows() {
        let code = transformed(false, &source);
        assert_eq!(
            decoded(&code),
            label.starts_with("dec-"),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn every_row_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let bits = |source: &str| transformed(false, source);
    let points_to = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("bits", &bits), ("points-to", &points_to)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
