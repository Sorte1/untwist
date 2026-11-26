use untwist::{Options, Pipeline};
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const KEPT: [(&str, &str); 9] = [
    ("kept-integrity-key-read.js", "fnObject"),
    ("kept-altered-decoder.js", "_MAIN_STR("),
    ("kept-bom.js", "_MAIN_STR("),
    ("kept-early-call.js", "_MAIN_STR("),
    ("kept-packed-reassigned.js", "_MAIN_STR("),
    ("kept-calculator-key.js", "_calc("),
    ("kept-literal-array-read.js", "_dlrArray"),
    ("kept-global-shadowed.js", "_getGlobal("),
    ("patched-predicate.js", "_dummyFunction"),
];

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
    let fixtures = js_files_in("fixtures/js-confuser-decode");
    assert_eq!(fixtures.len(), 13);
    fixtures
}

fn named(fixtures: &[(String, String)], name: &str) -> String {
    fixtures
        .iter()
        .find(|(label, _)| label.ends_with(name))
        .map(|(_, source)| source.clone())
        .unwrap_or_else(|| panic!("no fixture {name}"))
}

fn unpatched(fixtures: Vec<(String, String)>) -> Vec<(String, String)> {
    fixtures
        .into_iter()
        .filter(|(label, _)| {
            !label
                .rsplit('/')
                .next()
                .is_some_and(|n| n.starts_with("patched-"))
        })
        .collect()
}

#[test]
fn every_js_confuser_fixture_prints_what_node_prints_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 1] = [("default", &default_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_unpatched_js_confuser_fixture_prints_what_node_prints_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &unpatched(fixtures()), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn concealed_strings_decode_and_their_machinery_goes_in_both_lanes() {
    let source = named(&fixtures(), "decoded.js");
    for code in [default_lane(&source), unsafe_lane(&source)] {
        assert!(code.contains("hello, "), "{code}");
        assert!(!code.contains("_MAIN_STR("), "{code}");
        assert!(!code.contains("TextDecoder"), "{code}");
    }
}

#[test]
fn an_escaping_getter_is_kept_while_its_calls_decode() {
    let source = named(&fixtures(), "getter-escapes.js");
    for code in [default_lane(&source), unsafe_lane(&source)] {
        assert!(code.contains("\"done here\""), "{code}");
        assert!(code.contains("TextDecoder"), "{code}");
    }
}

#[test]
fn near_misses_keep_their_machinery_in_the_default_lane() {
    let fixtures = fixtures();
    for (name, marker) in KEPT {
        let code = default_lane(&named(&fixtures, name));
        assert!(code.contains(marker), "{name}:\n{code}");
    }
}

#[test]
fn a_patched_built_in_keeps_the_default_lane_and_not_the_unsafe_lane() {
    let source = named(&fixtures(), "patched-slice.js");
    assert!(default_lane(&source).contains("_MAIN_STR("));
    assert!(!unsafe_lane(&source).contains("_MAIN_STR("));
}

#[test]
fn an_integrity_check_that_passes_on_the_input_goes_in_both_lanes() {
    let source = named(&fixtures(), "integrity.js");
    for code in [default_lane(&source), unsafe_lane(&source)] {
        assert!(!code.contains("fnObject"), "{code}");
        assert!(!code.contains("while (true)"), "{code}");
    }
}
