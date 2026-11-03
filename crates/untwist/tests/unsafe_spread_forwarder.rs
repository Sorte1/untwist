use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{node_or_skip, printed};

const FORWARDED: &str = "\
function emit(handler, ...args) {
  var _0x3a1b2c = {
    'hhQdt': function (a, b) {
      return a(b);
    },
    'FXgTh': function (a, b) {
      return a + b;
    }
  };
  _0x3a1b2c['hhQdt'](handler, ...args);
  return _0x3a1b2c['FXgTh'](1, 2);
}
console.log(emit(console.log, 'hit'));
";

const ORIGINAL: &str = "\
function emit(handler, ...args) {
  handler(...args);
  return 1 + 2;
}
console.log(emit(console.log, 'hit'));
";

fn run(unsafe_transforms: bool) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(FORWARDED).code().to_string()
}

#[test]
fn under_unsafe_a_spread_into_a_forwarder_is_passed_on_and_the_object_removed() {
    let code = run(true);
    assert!(code.contains("handler(...args)"), "{code}");
    assert!(!code.contains("hhQdt"), "{code}");
}

#[test]
fn the_default_lane_keeps_the_forwarder_a_spread_reaches() {
    let code = run(false);
    assert!(code.contains("hhQdt"), "{code}");
}

#[test]
fn the_unsafe_output_prints_what_the_original_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let expected = printed(&node, dir.path(), ORIGINAL);
    let actual = printed(&node, dir.path(), &run(true));
    assert!(
        expected.is_ok() && expected == actual,
        "{expected:?} vs {actual:?}"
    );
}
