use std::path::PathBuf;

use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/uglify-shapes")
}

fn read(name: &str) -> String {
    let path = dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn programs() -> Vec<(String, String)> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".js"))
        .collect();
    names.push("hand/probes.js".to_string());
    names.sort();
    names
        .into_iter()
        .map(|name| (name.clone(), read(&name)))
        .collect()
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn safe(source: &str) -> String {
    transformed(&Options::default(), source)
}

fn unsafe_lane(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    transformed(&options, source)
}

#[test]
fn uglify_output_prints_the_same_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = programs();
    assert!(programs.len() >= 7, "found {} programs", programs.len());
    let modes: [Mode<'_>; 2] = [("default", &safe), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn assert_has(code: &str, parts: &[&str]) {
    for part in parts {
        assert!(code.contains(part), "missing {part:?} in:\n{code}");
    }
}

fn assert_lacks(code: &str, parts: &[&str]) {
    for part in parts {
        assert!(!code.contains(part), "still has {part:?} in:\n{code}");
    }
}

#[test]
fn merged_variables_are_split_in_the_default_lane() {
    let code = safe(&read("merge-vars.min.js"));
    assert_has(
        &code,
        &[
            "const o_2__ = /([a-z]+)/.exec(t);",
            "const t_2__ = o == \"object\"",
            "t_3__.color = \"red\";",
        ],
    );
    assert_lacks(&code, &["var o = /([a-z]+)/", "t = t.style;"]);
}

#[test]
fn leading_assignments_become_statements() {
    let code = safe(&read("collapse.min.js"));
    assert_has(&code, &["cache.hits++;", "typeof o === \"number\""]);
    assert_lacks(&code, &["(cache = cache || { hits: 0 }).hits++"]);
}

#[test]
fn probes_keep_what_they_cannot_prove() {
    let code = safe(&read("hand/probes.js"));
    assert_has(
        &code,
        &[
            "t = t.style;\n  t.color = \"red\";",
            "const f = function () {\n    return k;\n  };\n  k = e * 3;",
        ],
    );
}
