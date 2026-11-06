use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const DECODED: &[&str] = &[
    "object-keys.js",
    "string-argument.js",
    "local-object-key.js",
];
const DECODED_UNSAFE: &[&str] = &["object-keys.js"];
const UNSAFE_DIVERGES: &[&str] = &["enumerated-keys.js", "object-keys-assigned.js"];

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
    assert_eq!(fixtures.len(), 10);
    fixtures
}

fn is_decoded(label: &str, unsafe_transforms: bool) -> bool {
    let decoded = if unsafe_transforms {
        DECODED_UNSAFE
    } else {
        DECODED
    };
    decoded.iter().any(|name| label.ends_with(name))
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
fn every_spelled_name_fixture_but_the_unwatched_writes_prints_what_node_prints_in_the_unsafe_lane()
{
    let Some(node) = node_or_skip() else {
        return;
    };
    let checked: Vec<(String, String)> = fixtures()
        .into_iter()
        .filter(|(label, _)| !UNSAFE_DIVERGES.iter().any(|name| label.ends_with(name)))
        .collect();
    let unsafe_lane = lane(true);
    let runs: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &checked, &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn only_spellings_that_cannot_reach_the_global_object_leave_the_unit_decodable() {
    for unsafe_transforms in [false, true] {
        for (label, source) in fixtures() {
            let code = lane(unsafe_transforms)(&source);
            let decoded =
                !code.contains("d(0x") && !code.contains("d(0)") && !code.contains("d(1)");
            if UNSAFE_DIVERGES.iter().any(|name| label.ends_with(name)) && unsafe_transforms {
                continue;
            }
            let expected = is_decoded(&label, unsafe_transforms);
            assert_eq!(
                decoded, expected,
                "{label} [unsafe {unsafe_transforms}]:\n{code}"
            );
        }
    }
}
