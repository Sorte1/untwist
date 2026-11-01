use untwist::{Options, Output, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Safe,
    Unsafe,
    Conservative,
}

fn run(source: &str, lane: Lane) -> Output {
    Pipeline::new(Options {
        unsafe_transforms: lane == Lane::Unsafe,
        extended: lane != Lane::Conservative,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn folded(source: &str, lane: Lane) -> bool {
    !run(source, lane).code().contains("fromCharCode(")
}

const FOLDS: [(&str, &str); 6] = [
    (
        "a fold read by a chained method",
        "console.log(String.fromCharCode(99, 104, 111, 110, 107, 101, 114, 99, 104, 101, 97, 116, 115).toLowerCase());",
    ),
    (
        "a lone surrogate",
        "console.log(String.fromCharCode(0xD800).charCodeAt(0), String.fromCharCode(0xD83D, 0xDE00).length);",
    ),
    (
        "a code unit past 0xFFFF is masked",
        "console.log(String.fromCharCode(0x10041, 65601), String.fromCharCode(-1).charCodeAt(0));",
    ),
    (
        "NaN and Infinity become zero",
        "console.log(String.fromCharCode(NaN, Infinity).charCodeAt(1), String.fromCharCode(NaN).length);",
    ),
    (
        "a chain of string methods",
        "console.log(String.fromCharCode(104, 101, 108, 108, 111).slice(1, 4).toUpperCase().padStart(6, '*'), String.fromCharCode(97, 98).repeat(3), String.fromCharCode(32, 120, 32).trim().indexOf('x'));",
    ),
    (
        "search methods",
        "console.log(String.fromCharCode(102, 111, 111).includes('o'), String.fromCharCode(102, 111, 111).startsWith('fo'), String.fromCharCode(102, 111, 111).endsWith('x'), String.fromCharCode(102, 111, 111).at(-1));",
    ),
];

const METHOD_PATCHED: &str = "String.prototype.toUpperCase = function () { return 'upper'; };\nconsole.log(String.fromCharCode(104, 105).toUpperCase());";

const PATCHED: [(&str, &str); 3] = [
    (
        "fromCharCode replaced",
        "String.fromCharCode = function () { return 'patched'; };\nconsole.log(String.fromCharCode(104, 105).toUpperCase());",
    ),
    (
        "String replaced through the global object",
        "globalThis.String = { fromCharCode: function () { return 'global'; } };\nconsole.log(String.fromCharCode(104, 105));",
    ),
    (
        "fromCharCode redefined",
        "Object.defineProperty(String, 'fromCharCode', { value: function () { return 'defined'; } });\nconsole.log(String.fromCharCode(104, 105));",
    ),
];

const SHADOWED: &str = "var String = { fromCharCode: function () { return 'local'; } };\nconsole.log(String.fromCharCode(104, 105));";

#[test]
fn constant_char_code_chains_fold_extended() {
    for (label, source) in FOLDS {
        for lane in [Lane::Safe, Lane::Unsafe] {
            assert!(
                folded(source, lane),
                "{label} ({lane:?}): {}",
                run(source, lane).code()
            );
        }
    }
}

#[test]
fn the_safe_lane_refuses_when_a_write_may_reach_the_built_ins() {
    for (label, source) in PATCHED {
        assert!(
            !folded(source, Lane::Safe),
            "{label}: {}",
            run(source, Lane::Safe).code()
        );
        assert!(
            folded(source, Lane::Unsafe),
            "{label}: {}",
            run(source, Lane::Unsafe).code()
        );
    }
}

#[test]
fn a_patched_chained_method_stays_a_call_on_the_folded_string() {
    for lane in [Lane::Safe, Lane::Unsafe] {
        let code = run(METHOD_PATCHED, lane).code().to_string();
        assert!(!code.contains("fromCharCode("), "{lane:?}: {code}");
        assert!(code.contains(".toUpperCase()"), "{lane:?}: {code}");
    }
}

#[test]
fn a_local_string_binding_blocks_every_lane() {
    for lane in [Lane::Safe, Lane::Unsafe, Lane::Conservative] {
        assert!(
            !folded(SHADOWED, lane),
            "{lane:?}: {}",
            run(SHADOWED, lane).code()
        );
    }
}

fn programs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, source)| (label.to_string(), source.to_string()))
        .collect()
}

#[test]
fn node_agrees_in_every_lane_and_each_patch_diverges_when_folded_anyway() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = |source: &str| run(source, Lane::Safe).code().to_string();
    let unsafe_lane = |source: &str| run(source, Lane::Unsafe).code().to_string();
    let conservative = |source: &str| run(source, Lane::Conservative).code().to_string();
    let mut all = programs(&FOLDS);
    all.push(("shadowed".to_string(), SHADOWED.to_string()));
    all.push(("method patched".to_string(), METHOD_PATCHED.to_string()));
    let lanes: [Mode<'_>; 3] = [
        ("safe", &safe),
        ("unsafe", &unsafe_lane),
        ("conservative", &conservative),
    ];
    let wrong = mismatches(&node, &all, &lanes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let patched = programs(&PATCHED);
    let guarded: [Mode<'_>; 2] = [("safe", &safe), ("conservative", &conservative)];
    let wrong = mismatches(&node, &patched, &guarded);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let folded_anyway: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    assert_eq!(
        mismatches(&node, &patched, &folded_anyway).len(),
        PATCHED.len()
    );
}
