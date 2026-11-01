use untwist::{Options, Output, Pipeline};
use untwist_testing::node_rows::{
    mismatches, node_or_skip, patched_after_rotator, with_machinery_restored, Mode,
};

const DECODER_CALL: &str = "console.log(b(";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Safe,
    Unsafe,
}

fn run(source: &str, lane: Lane) -> Output {
    Pipeline::new(Options {
        unsafe_transforms: lane == Lane::Unsafe,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn decoded(source: &str, lane: Lane) -> bool {
    !run(source, lane).code().contains(DECODER_CALL)
}

const RECURSIVE_HELPER: &str =
    "function walk(n, f) { f(n.v); for (var i = 0; i < n.c.length; i++) walk(n.c[i], f); }
function visit(t) { walk(t, function (x) { console.log(x); }); }
visit({ v: 1, c: [{ v: 2, c: [] }] });
console.log(typeof globalThis.setTimeout);";

const RECURSIVE_GLOBAL_READ: &str = "(function () {
  function read(o, k, d) { if (d) return read(globalThis, k, 0); return o[k]; }
  var t = read({}, (11).toString(36), 1);
  console.log(typeof t);
})();";

const DYNAMIC_PATHS: [(&str, &str); 3] = [
    (
        "an unknown-key defineProperty on a maybe-global parameter",
        "function d(o, k, g) { Object.defineProperty(o, k, { get: g }); }\nd({}, 'x', function () { return 1; });\nconsole.log(typeof Object.getOwnPropertyDescriptor(globalThis, String.fromCharCode(98)));",
    ),
    (
        "an enumeration of the global object",
        "console.log(Object.values(globalThis).filter(function (v) { return typeof v === 'function'; }).length > 0);",
    ),
    (
        "an unknown-key read of the global object",
        "var k = String.fromCharCode(98);\nconsole.log(typeof globalThis[k]);",
    ),
];

const A_L1_WITNESSES: [&str; 1] = ["an unknown-key read of the global object"];

const MACHINERY_NAME_WITNESSES: [&str; 1] =
    ["an unknown-key defineProperty on a maybe-global parameter"];

#[test]
fn a_recursive_helper_inside_an_inlined_helper_no_longer_blocks_decoding() {
    let source = patched_after_rotator("true", RECURSIVE_HELPER);
    assert!(
        decoded(&source, Lane::Safe),
        "{}",
        run(&source, Lane::Safe).code()
    );
    assert!(decoded(&source, Lane::Unsafe));
}

#[test]
fn a_recursive_helper_that_reads_the_global_object_still_refuses_by_default() {
    let source = patched_after_rotator("true", RECURSIVE_GLOBAL_READ);
    assert!(
        !decoded(&source, Lane::Safe),
        "{}",
        run(&source, Lane::Safe).code()
    );
}

#[test]
fn unsafe_assumes_dynamic_paths_to_the_decoder_away_and_the_safe_lane_keeps_them() {
    for (label, patch) in DYNAMIC_PATHS {
        let source = patched_after_rotator("true", patch);
        assert!(
            decoded(&source, Lane::Unsafe),
            "{label}: {}",
            run(&source, Lane::Unsafe).code()
        );
        assert!(!decoded(&source, Lane::Safe), "{label}");
    }
}

fn lane_runner(lane: Lane) -> impl Fn(&str) -> String {
    move |source: &str| run(source, lane).code().to_string()
}

#[test]
fn node_agrees_by_default_and_unsafe_diverges_only_on_its_a_l1_witnesses() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs: Vec<(String, String)> = DYNAMIC_PATHS
        .iter()
        .map(|(label, patch)| (label.to_string(), patched_after_rotator("true", patch)))
        .collect();
    programs.push((
        "a recursive helper".to_string(),
        patched_after_rotator("true", RECURSIVE_HELPER),
    ));
    programs.push((
        "a recursive helper that reads the global object".to_string(),
        patched_after_rotator("true", RECURSIVE_GLOBAL_READ),
    ));
    let safe = lane_runner(Lane::Safe);
    let modes: [Mode<'_>; 1] = [("safe", &safe)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let unsafe_lane = lane_runner(Lane::Unsafe);
    let modes: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let mut diverged: Vec<String> = mismatches(&node, &programs, &modes)
        .into_iter()
        .filter_map(|row| row.split(" (unsafe)").next().map(str::to_string))
        .collect();
    diverged.sort();
    let mut want: Vec<String> = A_L1_WITNESSES
        .iter()
        .chain(MACHINERY_NAME_WITNESSES.iter())
        .map(|w| w.to_string())
        .collect();
    want.push("a recursive helper that reads the global object".to_string());
    want.sort();
    assert_eq!(diverged, want);
    let restored = |source: &str| with_machinery_restored(source, &unsafe_lane(source));
    let modes: [Mode<'_>; 1] = [("unsafe, machinery restored", &restored)];
    let named: Vec<(String, String)> = programs
        .iter()
        .filter(|(label, _)| MACHINERY_NAME_WITNESSES.contains(&label.as_str()))
        .cloned()
        .collect();
    let wrong = mismatches(&node, &named, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
}
