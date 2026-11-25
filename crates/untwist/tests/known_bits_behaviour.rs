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
    assert_eq!(fixtures.len(), 21);
    fixtures
}

const UNSAFE_PREMISE: [&str; 2] = [
    "kept-checksum-global-call.js",
    "kept-checksum-global-write.js",
];

fn under_unsafe_premise(label: &str) -> bool {
    UNSAFE_PREMISE.iter().any(|name| label.ends_with(name))
}

#[test]
fn every_known_bits_fixture_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let (premise, both): (Vec<_>, Vec<_>) = fixtures()
        .into_iter()
        .partition(|(label, _)| under_unsafe_premise(label));
    assert_eq!(premise.len(), UNSAFE_PREMISE.len());
    let runs: [Mode<'_>; 2] = [("default", &default_lane), ("unsafe", &unsafe_lane)];
    let mut wrong = node_rows::mismatches(&node, &both, &runs);
    let default_only: [Mode<'_>; 1] = [("default", &default_lane)];
    wrong.extend(node_rows::mismatches(&node, &premise, &default_only));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn is_folded(label: &str) -> bool {
    label
        .rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("folded"))
}

#[test]
fn predicates_decided_by_known_bits_fold_in_both_lanes() {
    for (label, source) in fixtures().into_iter().filter(|(label, _)| is_folded(label)) {
        for code in [default_lane(&source), unsafe_lane(&source)] {
            assert!(!code.contains("Math.imul(seed"), "{label}:\n{code}");
            assert!(!code.contains(" ? \""), "{label}:\n{code}");
        }
    }
}

#[test]
fn predicates_known_bits_cannot_decide_stay_in_the_default_lane() {
    for (label, source) in fixtures() {
        if is_folded(&label) {
            continue;
        }
        let code = default_lane(&source);
        assert!(code.contains(") === "), "{label}:\n{code}");
    }
}
