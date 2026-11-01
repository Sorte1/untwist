use untwist::{Options, Pipeline};
use untwist_deobfuscate::notes;
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const BAILED: &str = "decoding bailed";

const EXPECTED: &[(&str, Option<(&str, &str)>)] = &[
    (
        "patch-before-rotator.js",
        Some((notes::CODE_BEFORE_ROTATOR, "")),
    ),
    ("rotator-only-after.js", None),
    ("unbounded-read-global.js", None),
    ("unbounded-read-hits-global.js", Some((BAILED, ""))),
    (
        "unbounded-read-hits-method.js",
        Some((notes::BUILTIN_PATCHED, "Math")),
    ),
    ("unbounded-read-hits-namespace.js", Some((BAILED, ""))),
    (
        "unbounded-read-hits-prototype.js",
        Some((notes::BUILTIN_PATCHED, "String.prototype.charAt")),
    ),
    ("unbounded-read-prototype.js", None),
];

fn default_lane(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = js_files_in("fixtures/builtin-replaced");
    assert_eq!(fixtures.len(), EXPECTED.len());
    fixtures
}

fn expected(label: &str) -> Option<(&'static str, &'static str)> {
    EXPECTED
        .iter()
        .find(|(name, _)| label.ends_with(name))
        .unwrap_or_else(|| panic!("{label} has no expectation"))
        .1
}

#[test]
fn every_builtin_replaced_fixture_prints_what_node_prints_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 1] = [("default", &default_lane)];
    let wrong = node_rows::mismatches(&node, &fixtures(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_refused_unit_names_the_property_written_and_the_others_decode() {
    for (label, source) in fixtures() {
        let out = Pipeline::new(Options::default()).run(&source);
        let undecoded = out.code().contains("b(1)");
        match expected(&label) {
            None => assert!(!undecoded, "{label}:\n{}", out.code()),
            Some((note, named)) => {
                assert!(undecoded, "{label}:\n{}", out.code());
                let wanted = if named.is_empty() {
                    note.to_string()
                } else {
                    format!("{note}: {named}")
                };
                assert!(
                    out.notes.iter().any(|seen| seen.message.contains(&wanted)),
                    "{label}: {wanted} not in {:?}",
                    out.notes
                );
            }
        }
    }
}
