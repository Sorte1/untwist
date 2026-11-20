use std::path::PathBuf;

use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, printed};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/r7nx12");

const SELF_DEFENDING: &str = "(((.+)+)+)+$";

const STATICS_READ: [&str; 21] = [
    "f3a",
    "f3b",
    "f3c",
    "l1-lastMatch",
    "l6-num-key",
    "m1-unk",
    "m4-var-cat",
    "m5-let-cat",
    "m8-call",
    "m9-tpl",
    "m10-lastMatch-cat",
    "m13-inc",
    "m15-reflect",
    "m19-ctor",
    "m20-opt",
    "m24-global-cat",
    "m25-const-key",
    "m26-var-key",
    "m27-name-cat",
    "m28-name-cat-call",
    "c-static",
    "n6-ctor-static",
    "n7-ctor-var",
    "n8-ctor-lastMatch",
];

const NUMERIC_LOOKING_KEYS: [&str; 28] = [
    "f1a",
    "f1d",
    "f2a",
    "f2c",
    "f2d",
    "a1-arguments",
    "a1b-arguments-fn",
    "a1c-arguments-led",
    "a2-annexb",
    "a2b-annexb-if",
    "a5-pattern",
    "a6-switchfn",
    "a9-proto",
    "a10-number",
    "g30-exports",
    "g31-top-arguments",
    "g32-module",
    "h31-if-fn",
    "h32-nested-block",
    "h34-args-block",
    "h35-args-for",
    "h39-args-join",
    "h40-args-led",
    "h41-annexb-led",
    "h42-annexb-inc",
    "h43-annexb-arrow",
    "h46-annexb-inblock-read",
    "h47-catch-fn",
];

fn lane(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        ..Options::default()
    }
}

fn transformed(unsafe_transforms: bool, source: &str) -> String {
    Pipeline::new(lane(unsafe_transforms))
        .run(source)
        .code()
        .to_string()
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
fn every_r7_nx12_probe_prints_what_node_prints_by_default() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = probes();
    assert!(programs.len() >= 60, "found {} probes", programs.len());
    let run = |source: &str| transformed(false, source);
    let modes: Vec<node_rows::Mode<'_>> = vec![("default", &run)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn under_unsafe_a_probe_prints_what_node_prints_unless_a_listed_premise_covers_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for (label, source) in probes() {
        let listed = STATICS_READ.contains(&label.as_str())
            || NUMERIC_LOOKING_KEYS.contains(&label.as_str());
        let expected = printed(&node, dir.path(), &source);
        let actual = printed(&node, dir.path(), &transformed(true, &source));
        if expected.is_ok() && expected == actual {
            continue;
        }
        if !listed && !patches_a_built_in(&source) {
            wrong.push(format!("{label}: {expected:?} vs {actual:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn patches_a_built_in(source: &str) -> bool {
    [".prototype.", ").isArray = "]
        .iter()
        .any(|patch| source.contains(patch))
}

#[test]
fn a_regexp_static_read_keeps_the_self_defending_block_by_default() {
    for label in STATICS_READ {
        let code = transformed(false, &probe(label));
        assert!(code.contains(SELF_DEFENDING), "{label}: {code}");
    }
}

#[test]
fn a_key_named_arguments_or_overwritten_by_a_block_function_keeps_the_block_by_default() {
    for label in NUMERIC_LOOKING_KEYS {
        let code = transformed(false, &probe(label));
        assert!(code.contains(SELF_DEFENDING), "{label}: {code}");
    }
}

#[test]
fn the_unsafe_lane_still_removes_the_self_defending_block() {
    for label in STATICS_READ.iter().chain(&NUMERIC_LOOKING_KEYS) {
        let code = transformed(true, &probe(label));
        assert!(!code.contains(SELF_DEFENDING), "{label}: {code}");
    }
}

#[test]
fn a_hole_key_keeps_the_direct_eval_and_the_spread_helper_by_default() {
    for label in ["f1b", "f2b", "e1-h1-evaltext", "e1-h2-evaltext"] {
        let code = transformed(false, &probe(label));
        assert!(code.contains("eval("), "{label}: {code}");
    }
    for label in ["f1c", "s1-h1-spread"] {
        let code = transformed(false, &probe(label));
        assert!(code.contains("_toConsumableArray"), "{label}: {code}");
    }
}
