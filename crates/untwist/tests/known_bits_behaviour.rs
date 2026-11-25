use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

fn lane(source: &str, unsafe_transforms: bool) -> String {
    Pipeline::new(Options {
        unsafe_transforms,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

fn default_lane(source: &str) -> String {
    lane(source, false)
}

fn unsafe_lane(source: &str) -> String {
    lane(source, true)
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = js_files_in("fixtures/known-bits");
    assert_eq!(fixtures.len(), 7);
    fixtures
}

#[test]
fn every_known_bits_fixture_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 2] = [("default", &default_lane), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn predicates_decided_by_known_bits_fold_in_both_lanes() {
    let (label, source) = fixtures()
        .into_iter()
        .find(|(label, _)| label.ends_with("folded.js"))
        .expect("folded.js");
    for code in [default_lane(&source), unsafe_lane(&source)] {
        assert!(!code.contains("Math.imul"), "{label}:\n{code}");
        assert!(!code.contains(") === "), "{label}:\n{code}");
    }
}

#[test]
fn predicates_known_bits_cannot_decide_stay_in_the_default_lane() {
    for (label, source) in fixtures() {
        if label.ends_with("folded.js") {
            continue;
        }
        let code = default_lane(&source);
        assert!(code.contains(") === "), "{label}:\n{code}");
    }
}
