use std::path::Path;

use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_testing::node::assert_node_agrees;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/libraries")
        .join(name);
    std::fs::read_to_string(path).expect("reading a fixture")
}

fn run_unsafe(source: &str) -> Output {
    let options = Options {
        unsafe_transforms: true,
        rename: RenameMode::Smart,
        ..Options::default()
    };
    Pipeline::new(options).run(source)
}

const SELF_TEXT_READ: &str = "
class Probe {
  show() {
    return typeof this.toString();
  }
}
console.log(new Probe().show());
";

#[test]
fn inlined_babel_helpers_take_their_library_names() {
    let source = fixture("babel-helpers/inline.rollup-cjs.terser.js");
    let output = run_unsafe(&source);
    for expected in [
        "function _arrayLikeToArray(",
        "function asyncGeneratorStep(",
        "function _defineProperties(",
    ] {
        assert!(output.code().contains(expected), "missing {expected}");
    }
    assert_node_agrees(&source, output.code());
}

#[test]
fn inlined_swc_helpers_take_their_library_names() {
    let source = fixture("swc-inline/inline.esbuild.min.js");
    let output = run_unsafe(&source);
    for expected in [
        "function _arrayLikeToArray(",
        "function _asyncToGenerator(",
        "function _possibleConstructorReturn(",
    ] {
        assert!(output.code().contains(expected), "missing {expected}");
    }
    assert_node_agrees(&source, output.code());
}

#[test]
fn an_instance_reading_its_own_text_does_not_stop_helper_names_under_unsafe() {
    let source = fixture("babel-helpers/inline.rollup-cjs.terser.js") + SELF_TEXT_READ;
    let output = run_unsafe(&source);
    assert!(
        output.code().contains("function _arrayLikeToArray("),
        "{}",
        output.code()
    );
    assert_node_agrees(&source, output.code());
}
