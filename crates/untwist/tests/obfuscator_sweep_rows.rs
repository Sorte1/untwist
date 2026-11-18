use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{node_or_skip, printed};

const DECODED_UNDER_UNSAFE: &[&str] = &[
    "cfo-dead-branch",
    "stateful-outside-calls",
    "decoder-not-callee",
    "alias-before-declaration",
];

const FINGERPRINTS: &[&str] = &[
    "(((.+)+)+)+$",
    "debu",
    "gger",
    "return (function() ",
    "\"exception\"",
    "fromCharCode",
    "shift()",
];

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/obfuscator_sweep")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(data().join(name)).unwrap()
}

fn unsafe_lane(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn dead_code_and_control_flow_cases_decode_fully_under_unsafe() {
    for label in DECODED_UNDER_UNSAFE {
        let output = unsafe_lane(&read(&format!("{label}.js")));
        let left: Vec<&&str> = FINGERPRINTS
            .iter()
            .filter(|print| output.contains(**print))
            .collect();
        assert!(left.is_empty(), "{label} keeps {left:?}:\n{output}");
    }
}

#[test]
fn dead_code_and_control_flow_cases_print_the_source_output_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for label in DECODED_UNDER_UNSAFE {
        let source = read(&format!("{label}.source.js"));
        let output = unsafe_lane(&read(&format!("{label}.js")));
        let expected = printed(&node, dir.path(), &source);
        let actual = printed(&node, dir.path(), &output);
        assert!(
            expected.is_ok() && expected == actual,
            "{label}: {expected:?} vs {actual:?}"
        );
    }
}

#[test]
fn a_self_defending_block_a_later_layer_put_in_a_control_flow_function_is_removed() {
    let output = unsafe_lane(&read("sd-controller-survives.js"));
    assert!(!output.contains("_0x1977fa"), "{output}");
    assert!(!output.contains("_0x59eeab"), "{output}");
    assert!(!output.contains("setInterval"), "{output}");
}
