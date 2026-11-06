use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const DECODED: &[&str] = &[
    "object-keys.js",
    "string-argument.js",
    "local-object-key.js",
];

fn lane(unsafe_transforms: bool) -> impl Fn(&str) -> String {
    move |source| {
        Pipeline::new(Options {
            unsafe_transforms,
            ..Options::default()
        })
        .run(source)
        .code()
        .to_string()
    }
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = js_files_in("fixtures/spelled-name");
    assert_eq!(fixtures.len(), 9);
    fixtures
}

fn is_decoded(label: &str) -> bool {
    DECODED.iter().any(|name| label.ends_with(name))
}

#[test]
fn every_spelled_name_fixture_prints_what_node_prints_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let default_lane = lane(false);
    let runs: [Mode<'_>; 1] = [("default", &default_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_decoded_spelled_name_fixture_prints_what_node_prints_in_the_unsafe_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let decoded: Vec<(String, String)> = fixtures()
        .into_iter()
        .filter(|(label, _)| is_decoded(label))
        .collect();
    let unsafe_lane = lane(true);
    let runs: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &decoded, &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn only_spellings_that_cannot_reach_the_global_object_leave_the_unit_decodable() {
    for unsafe_transforms in [false, true] {
        for (label, source) in fixtures() {
            let code = lane(unsafe_transforms)(&source);
            let decoded =
                !code.contains("d(0x") && !code.contains("d(0)") && !code.contains("d(1)");
            if is_decoded(&label) {
                assert!(decoded, "{label} [unsafe {unsafe_transforms}]:\n{code}");
            } else if !unsafe_transforms {
                assert!(!decoded, "{label} [default]:\n{code}");
            }
        }
    }
}

#[test]
fn every_refused_spelled_name_fixture_diverges_from_node_once_the_reach_watch_is_off() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let refused: Vec<(String, String)> = fixtures()
        .into_iter()
        .filter(|(label, _)| !is_decoded(label))
        .collect();
    let unwatched = lane(true);
    let runs: [Mode<'_>; 1] = [("unsafe", &unwatched)];
    let wrong = node_rows::mismatches(&node, &refused, &runs);
    for (label, _) in &refused {
        assert!(
            wrong.iter().any(|row| row.starts_with(label.as_str())),
            "{label} did not diverge from node without the reach watch"
        );
    }
}
