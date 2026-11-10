use untwist::{Options, Pipeline};
use untwist_core::budget::{check_points_seen, stop_at_check_point};

const SITES: usize = 16;
const DECODED: [&str; 2] = ["\"Hello World!\"", "\"split\""];

fn string_array_program() -> String {
    let sites: String = (0..SITES)
        .map(|index| format!("console.log(_0x44bc({}));\n", index % 2))
        .collect();
    format!(
        "function _0x44bc(_0x3fc3a6, _0x44bc45) {{\n  var _0x2004c0 = _0x3fc3();\n  _0x44bc = function (_0x4338c0, _0x393a93) {{\n    _0x4338c0 = _0x4338c0 - 0x0;\n    return _0x2004c0[_0x4338c0];\n  }};\n  return _0x44bc(_0x3fc3a6, _0x44bc45);\n}}\nfunction _0x3fc3() {{\n  var _0x1e397 = ['Hello\\x20World!', 'split', 'log'];\n  _0x3fc3 = function () {{\n    return _0x1e397;\n  }};\n  return _0x3fc3();\n}}\n{sites}"
    )
}

fn run(source: &str, stop: Option<u64>) -> String {
    stop_at_check_point(stop);
    let output = Pipeline::new(Options {
        unpack: false,
        jobs: std::num::NonZeroUsize::new(1),
        ..Options::default()
    })
    .run(source);
    output.code().to_string()
}

#[derive(Debug, PartialEq, Eq)]
enum Unit {
    Untouched,
    Decoded,
}

fn decoded_sites(code: &str) -> usize {
    DECODED
        .iter()
        .map(|text| code.matches(&format!("console.log({text})")).count())
        .sum()
}

fn unit_of(code: &str) -> Option<Unit> {
    let calls = code.matches("console.log(_0x44bc(").count();
    let machinery = ["function _0x44bc(", "function _0x3fc3(", "_0x1e397"]
        .iter()
        .filter(|name| code.contains(*name))
        .count();
    let decoded = decoded_sites(code);
    match (calls, machinery, decoded) {
        (SITES, 3, 0) => Some(Unit::Untouched),
        (0, 0, SITES) if !code.contains("_0x") => Some(Unit::Decoded),
        _ => None,
    }
}

#[test]
fn a_budget_stop_at_any_check_point_leaves_the_unit_whole_or_fully_decoded() {
    let source = string_array_program();
    let full = run(&source, None);
    let points = check_points_seen();
    assert_eq!(unit_of(&full), Some(Unit::Decoded), "{full}");
    assert!(points > 0);
    let mut seen = [0usize; 2];
    for point in 0..points {
        let code = run(&source, Some(point));
        match unit_of(&code) {
            Some(Unit::Untouched) => seen[0] += 1,
            Some(Unit::Decoded) => seen[1] += 1,
            None => panic!(
                "a stop at check point {point} of {points} left a half-applied unit:\n{code}"
            ),
        }
    }
    assert!(seen[0] > 0 && seen[1] > 0, "{seen:?} of {points}");
}
