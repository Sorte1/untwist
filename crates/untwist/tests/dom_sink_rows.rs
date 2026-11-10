use untwist::{Options, Pipeline};
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;

const ROWS: &str = include_str!("data/dom_sink_rows.txt");

fn rows() -> Vec<(&'static str, &'static str)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
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

fn wrong_rows(points_to: bool) -> Vec<String> {
    rows()
        .into_iter()
        .filter(|(label, source)| {
            decoded(&transformed(points_to, source)) != label.starts_with("dec-")
        })
        .map(|(label, _)| label.to_string())
        .collect()
}

#[test]
fn every_row_has_a_known_prefix() {
    let all = rows();
    assert!(all.len() >= 40, "found {} rows", all.len());
    for (label, _) in all {
        assert!(
            label.starts_with("dec-") || label.starts_with("ref-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn program_text_handed_to_the_dom_refuses_the_decode() {
    let wrong = wrong_rows(false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn program_text_handed_to_the_dom_refuses_the_decode_under_points_to() {
    let wrong = wrong_rows(true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
