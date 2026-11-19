mod common;

use std::path::PathBuf;

use common::scaling::assert_scales_linearly;

use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/a33rr10");

const FLAG_IN_DEAD_ZONE: [&str; 5] = ["200", "201", "202", "203", "438"];

const PATCHED_WITHOUT_A_NAME: [&str; 23] = [
    "300", "301", "302", "303", "304", "305", "306", "308", "309", "310", "321", "323", "324",
    "326", "332", "333", "335", "336", "345", "357", "454", "455", "456",
];

type Run = Box<dyn Fn(&str) -> String>;

fn default_lane() -> Options {
    Options::default()
}

fn unsafe_lane() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn transformed(options: Options, source: &str) -> String {
    Pipeline::new(options).run(source).code().to_string()
}

fn probe(label: &str) -> String {
    std::fs::read_to_string(PathBuf::from(DATA).join(format!("{label}.js"))).expect("read probe")
}

fn probes() -> Vec<(String, String)> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(DATA)
        .expect("probe dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|path| {
            let label = path.file_stem().unwrap().to_string_lossy().to_string();
            (label, std::fs::read_to_string(&path).expect("read probe"))
        })
        .collect()
}

#[test]
fn every_a33_rr10_probe_prints_what_node_prints_by_default() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = probes();
    assert!(programs.len() >= 100, "found {} probes", programs.len());
    let run: Run = Box::new(|source| transformed(default_lane(), source));
    let modes: Vec<node_rows::Mode<'_>> = vec![("default", run.as_ref())];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_let_flag_that_may_still_be_in_its_dead_zone_keeps_the_construct_in_both_lanes() {
    for label in FLAG_IN_DEAD_ZONE {
        for (name, options) in [("default", default_lane()), ("unsafe", unsafe_lane())] {
            let code = transformed(options, &probe(label));
            assert!(code.contains("apply"), "{label} ({name}): {code}");
        }
    }
}

#[test]
fn a_built_in_patched_without_naming_it_keeps_the_construct_by_default() {
    for label in PATCHED_WITHOUT_A_NAME {
        let code = transformed(default_lane(), &probe(label));
        assert!(code.contains("apply"), "{label}: {code}");
    }
}

#[test]
fn a_let_flag_declared_first_in_the_same_list_or_a_var_flag_is_still_removed() {
    for label in ["430", "436"] {
        for (name, options) in [("default", default_lane()), ("unsafe", unsafe_lane())] {
            let code = transformed(options, &probe(label));
            assert!(!code.contains("apply"), "{label} ({name}): {code}");
        }
    }
}

const RETURNER: &str = "(function (n, t) { var e = o ? function () { if (t) { var r = t.apply(n, arguments); t = null; return r; } } : function () {}; o = false; return e; })(this, function () {})();";

fn flags_and_returners(sets: usize) -> String {
    format!(
        "(function () {{ var o; {}{} }})();",
        "o = true;".repeat(sets),
        RETURNER.repeat(sets / 100)
    )
}

#[test]
fn many_flag_sets_beside_many_returners_scale_linearly() {
    assert_scales_linearly(
        "flag sets beside returners",
        2_000,
        flags_and_returners,
        default_lane,
    );
}

fn returners_among_statements(returners: usize) -> String {
    format!(
        "(function () {{ var o; o = true; {}{} }})();",
        RETURNER.repeat(returners),
        "var x = 1;".repeat(returners * 50)
    )
}

#[test]
fn many_returner_calls_among_many_statements_scale_linearly() {
    assert_scales_linearly(
        "returner calls among other statements",
        100,
        returners_among_statements,
        default_lane,
    );
}
