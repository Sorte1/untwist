use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/built_literal_rows.txt");
const UNIT: &str = "function aL() { var h = ['apples', 'pears', 'plums', 'label', 'log', 'area', 'scale', 'width', 'height', 'slice', 'defineProperty', '__proto__']; aL = function () { return h; }; return aL(); } function bL(c, d) { var e = aL(); return (bL = function (f, g) { f = f - 0; var i = e[f]; return String.fromCharCode(i.charCodeAt(0)) + i.slice(1); }), bL(c, d); } (function (c, d) { var e = c(); var tries = 0; while (!![]) { try { if (bL(4) === 'log') break; else e['push'](e['shift']()); } catch (h) { e['push'](e['shift']()); } if (tries++ > 1000) break; } })(aL, 0); ";
const TAIL: &str = " console.log(bL(3), bL(2));";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, body)| (label.to_string(), format!("{UNIT}{body}{TAIL}")))
        .collect()
}

fn options(unsafe_transforms: bool, mutant: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: if mutant {
            vec!["built-literal"]
        } else {
            vec![]
        },
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn decoded(options: &Options, source: &str) -> bool {
    !transformed(options, source).contains("bL(")
}

fn labelled(prefix: &str) -> Vec<(String, String)> {
    rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .collect()
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            label.starts_with("dec-") || label.starts_with("guard-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label() {
    let safe = options(false, false);
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        assert_eq!(
            decoded(&safe, &source),
            should_decode,
            "{label}:\n{}",
            transformed(&safe, &source)
        );
    }
}

#[test]
fn each_guard_row_is_refused_by_the_built_literal_checks_alone() {
    let mutant = options(false, true);
    let refused: Vec<String> = labelled("guard-")
        .into_iter()
        .filter(|(_, source)| !decoded(&mutant, source))
        .map(|(label, _)| label)
        .collect();
    assert!(refused.is_empty(), "refused under the mutant: {refused:?}");
}

#[test]
fn every_row_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = options(false, false);
    let aggressive = options(true, false);
    let mutant = options(false, true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let run_mutant = |source: &str| transformed(&mutant, source);
    let modes: [Mode<'_>; 3] = [
        ("safe", &run_safe),
        ("unsafe", &run_unsafe),
        ("mutant", &run_mutant),
    ];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
