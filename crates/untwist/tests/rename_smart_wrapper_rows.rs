use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const WRAPPER_VALUES: &str = "var hasRequiredModule1;
var module1;
function requireModule1() {
  if (hasRequiredModule1) { return module1; }
  hasRequiredModule1 = 1;
  module1 = { v: 1 };
  return module1;
}
function requireGetBuiltIn() { return { n: 2 }; }
function use() {
  var _0x1 = requireModule1();
  var _0x2 = requireGetBuiltIn();
  var _0x3 = requireModule1();
  return _0x1.v + _0x2.n + _0x3.v;
}
console.log(use(), use());";

fn smart_in(unsafe_transforms: bool) -> impl Fn(&str) -> String {
    move |source: &str| {
        let options = Options {
            rename: RenameMode::Smart,
            unsafe_transforms,
            unpack: false,
            jsx: false,
            ..Options::default()
        };
        Pipeline::new(options).run(source).code().to_string()
    }
}

#[test]
fn a_wrapper_call_result_is_named_after_the_wrapper_value() {
    let code = smart_in(false)(WRAPPER_VALUES);
    for want in [
        "module1 = requireModule1()",
        "getBuiltIn = requireGetBuiltIn()",
        "module12 = requireModule1()",
    ] {
        assert!(code.contains(want), "missing {want:?} in:\n{code}");
    }
    assert!(!code.contains("requireModule12"), "{code}");
}

#[test]
fn node_agrees_on_wrapper_value_names_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = vec![("wrapper values".to_string(), WRAPPER_VALUES.to_string())];
    let safe = smart_in(false);
    let unsafe_lane = smart_in(true);
    let modes: [Mode<'_>; 2] = [("smart", &safe), ("unsafe smart", &unsafe_lane)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
