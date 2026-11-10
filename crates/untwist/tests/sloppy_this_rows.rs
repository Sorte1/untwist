use untwist::{Options, Pipeline};
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/sloppy_this_rows.txt");

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
    assert!(all.len() >= 16, "found {} rows", all.len());
    for (label, _) in all {
        assert!(
            ["dec-", "ref-", "pt-"]
                .iter()
                .any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn the_bits_proof_takes_a_sloppy_this_for_the_global_object() {
    for (label, source) in rows() {
        if label.starts_with("pt-") {
            continue;
        }
        let code = transformed(false, &source);
        assert_eq!(
            decoded(&code),
            label.starts_with("dec-"),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn the_points_to_proof_takes_a_sloppy_this_for_the_global_object() {
    for (label, source) in rows() {
        if !label.starts_with("dec-") {
            let code = transformed(true, &source);
            assert!(!decoded(&code), "{label}:\n{code}");
        }
    }
}

#[test]
fn both_proofs_run_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let for_bits: Vec<(String, String)> = programs
        .iter()
        .filter(|(label, _)| !label.starts_with("pt-"))
        .cloned()
        .collect();
    let bits = |source: &str| transformed(false, source);
    let points_to = |source: &str| transformed(true, source);
    let bits_mode: [Mode<'_>; 1] = [("bits", &bits)];
    let points_to_mode: [Mode<'_>; 1] = [("points-to", &points_to)];
    let wrong = [
        node_rows::mismatches(&node, &for_bits, &bits_mode),
        node_rows::mismatches(&node, &programs, &points_to_mode),
    ]
    .concat();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
