use std::time::{Duration, Instant};

use untwist::{Options, Pipeline};
use untwist_eval::budget::{SAFE_STEPS_PER_DISTINCT_CALL, STEP_BUDGET};

const RUNAWAY_DECODER: &str = "function a() { var h = ['log', 'Hello World!', 'console', 'deobfuscate']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
for (var x = 0; x < 90000; x++) { for (var y = 0; y < 90000; y++) {} }
var i = e[f];
return i;
}), b(c, d);
}
";

const COSTLY_DECODER: &str = "function a() { var h = ['log', 'Hello World!', 'console', 'deobfuscate']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
for (var x = 0; x < 30000; x++) {}
var i = e[f % 4];
return i;
}), b(c, d);
}
";

const COSTLY_DISTINCT_CALLS: u64 = 300;

const RUNAWAY_DISTINCT_CALLS: u64 = 4;
const RUNAWAY_WALL: Duration = Duration::from_secs(5);

fn with_calls(decoder: &str, distinct: u64) -> String {
    let calls: String = (0..distinct)
        .flat_map(|index| [index, index])
        .map(|index| format!("console.log(b({index}));\n"))
        .collect();
    format!("{decoder}{calls}")
}

const A_ROTATOR: &str = concat!(
    include_str!("../../../fuzz/artifacts/evaluator/leak-3968f0c1984eb067dd5f0aa3136c4da24f3d77e8"),
    "console.log(b(1));\n"
);

fn options(fuel: Option<u64>) -> Options {
    Options {
        fuel,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

#[test]
fn the_default_fuel_decodes_the_call_site() {
    let out = Pipeline::new(options(None)).run(A_ROTATOR);
    assert!(
        out.code().contains("console.log(\"Hello World!\")"),
        "{}",
        out.code()
    );
}

#[test]
fn one_step_of_fuel_bails_the_decode_and_keeps_the_machinery() {
    let out = Pipeline::new(options(Some(1))).run(A_ROTATOR);
    assert!(
        !out.code().contains("console.log(\"Hello World!\")"),
        "{}",
        out.code()
    );
    assert!(
        out.notes
            .iter()
            .any(|note| note.message.contains("step budget exhausted")),
        "{:?}",
        out.notes
    );
}

#[test]
fn guard_budget_runaway_decoder_bails_the_safe_lane_at_its_scaled_budget() {
    let started = Instant::now();
    let out =
        Pipeline::new(options(None)).run(&with_calls(RUNAWAY_DECODER, RUNAWAY_DISTINCT_CALLS));
    let elapsed = started.elapsed();
    assert_eq!(
        out.stats.decode.peak_unit_steps,
        STEP_BUDGET + RUNAWAY_DISTINCT_CALLS * SAFE_STEPS_PER_DISTINCT_CALL
    );
    assert!(
        out.notes
            .iter()
            .any(|note| note.message.contains("step budget exhausted")),
        "{:?}",
        out.notes
    );
    assert!(out.code().contains("b(3)"), "{}", out.code());
    if cfg!(not(debug_assertions)) {
        assert!(elapsed <= RUNAWAY_WALL, "the runaway took {elapsed:?}");
    }
}

#[test]
fn a_unit_past_the_flat_budget_decodes_in_the_safe_lane_when_its_calls_pay_for_it() {
    let out = Pipeline::new(options(None)).run(&with_calls(COSTLY_DECODER, COSTLY_DISTINCT_CALLS));
    assert!(
        out.stats.decode.peak_unit_steps > STEP_BUDGET,
        "{}",
        out.stats.decode.peak_unit_steps
    );
    assert!(
        out.code().contains("console.log(\"Hello World!\")"),
        "{}",
        out.code()
    );
    assert!(!out.code().contains("b(1)"), "{}", out.code());
}
