mod common;

use oxc_allocator::Allocator;
use rayon::prelude::*;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{heap_budget, Budget, NESTING_DEPTH_CAP};
use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::heap::HeapLedger;
use untwist_eval::interp::eval_script_in;
use untwist_eval::test_hooks::{
    compile_census, frozen, nesting_at_first_bail, record_units, CompileCensus, ConstantRun,
    UnitRecord,
};

const DEEP_STACK: usize = 1 << 30;
const FUEL: &str = "step budget exhausted";
const ELEVEN_NODES: &str = "~(-0x1da3+0x4*0x56f+0x2b*0x35)";

type Outcome = (Vec<UnitRecord>, Option<u32>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unit {
    Script,
    Decode,
}

fn evaluate(source: &str, unit: Unit, fuel: Option<u64>) -> Outcome {
    untwist_eval::stack::grow(DEEP_STACK, || evaluate_here(source, unit, fuel))
}

fn evaluate_here(source: &str, unit: Unit, fuel: Option<u64>) -> Outcome {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = &parsed.program;
    let budget = || fuel.map_or_else(Budget::default, Budget::new);
    let mut records = Vec::new();
    let nesting = nesting_at_first_bail(|| {
        records = record_units(|| match unit {
            Unit::Script => eval_script_in(program, budget(), |_, _| ()),
            Unit::Decode => {
                let decoder = common::decoder_program(program, "_0x4d6b", &["_0x4c3e"], Some(0));
                let calls = common::decoder_calls_anywhere(program, &["_0x4c3e"]);
                let mut evaluator = InterpEvaluator::new(budget());
                evaluator.heap = Some(HeapLedger::new(heap_budget(source.len())));
                let _ = evaluator.decode(&decoder, &calls);
            }
        });
    });
    (records, nesting)
}

fn both(source: &str, unit: Unit, fuel: Option<u64>) -> (Outcome, CompileCensus) {
    let mut compiled = (Vec::new(), None);
    let census = compile_census(|| compiled = evaluate(source, unit, fuel));
    frozen(
        "fuel_exact",
        &format!("{unit:?}\n{fuel:?}\n{source}"),
        std::slice::from_ref(&compiled),
    );
    (compiled, census)
}

fn steps(outcome: &Outcome) -> u64 {
    outcome.0.iter().map(|unit| unit.steps_used).sum()
}

fn returned(constant: &str) -> String {
    format!("(function () {{ return {constant}; }})();")
}

#[test]
fn a_one_step_sweep_across_an_eleven_node_constant() {
    let source = returned(ELEVEN_NODES);
    let (full, census) = both(&source, Unit::Script, None);
    assert_eq!(census.constants, 1, "{census:?}");
    let run = census.constant_runs[0];
    assert_eq!((run.weight, run.depth), (17, 4), "{run:?}");
    let mut fallbacks = 0;
    let mut bails = 0;
    for cap in 0..=steps(&full) + 1 {
        let (outcome, census) = both(&source, Unit::Script, Some(cap));
        fallbacks += census.constant_fallbacks;
        bails += usize::from(outcome.1.is_some());
    }
    assert_eq!(
        fallbacks,
        run.weight - 1,
        "one fallback per cap inside the constant"
    );
    assert!(bails > 0);
}

fn wrapped(levels: usize) -> String {
    returned(&format!(
        "{}{ELEVEN_NODES}{}",
        "(0, ".repeat(levels),
        ")".repeat(levels)
    ))
}

fn root_run(levels: usize) -> ConstantRun {
    let (_, census) = both(&wrapped(levels), Unit::Script, None);
    census.constant_runs[0]
}

fn levels_over_the_cap_by(over: i64) -> usize {
    let near = root_run(20);
    assert_eq!(root_run(21).nesting, near.nesting + 1);
    let at = i64::from(near.nesting) + i64::from(near.depth) - i64::from(NESTING_DEPTH_CAP);
    usize::try_from(20 + over - at).expect("a level count")
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: constants 100,000 levels deep"
)]
fn the_nesting_cap_inside_a_constant_bails_as_the_walker_at_the_same_depth() {
    let base = levels_over_the_cap_by(0);
    for over in -2..=4i64 {
        let levels = usize::try_from(base as i64 + over).expect("levels");
        let (outcome, census) = both(&wrapped(levels), Unit::Script, None);
        let run = census.constant_runs[0];
        let reached = i64::from(run.nesting) + i64::from(run.depth) - i64::from(NESTING_DEPTH_CAP);
        assert_eq!(reached, over);
        let bail = outcome.0[0].bail.as_deref();
        assert_eq!(
            census.constant_fallbacks,
            u64::from(over > 0),
            "over {over}"
        );
        match over > 0 {
            true => {
                assert_eq!(bail, Some("nesting depth cap exceeded"), "over {over}");
                assert_eq!(outcome.1, Some(NESTING_DEPTH_CAP), "over {over}");
            }
            false => assert_eq!(bail, None, "over {over}"),
        }
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: constants 100,000 levels deep"
)]
fn the_nesting_cap_and_the_fuel_exhausted_together_bail_on_nesting_as_the_walker() {
    let source = wrapped(levels_over_the_cap_by(3));
    let (full, census) = both(&source, Unit::Script, None);
    let start = census.constant_runs[0].steps;
    let record = &full.0[0];
    assert_eq!(record.bail.as_deref(), Some("nesting depth cap exceeded"));
    let at_bail = record.steps_used;
    for cap in start.saturating_sub(2)..=at_bail + 2 {
        let (outcome, census) = both(&source, Unit::Script, Some(cap));
        let expected = if cap < at_bail {
            FUEL
        } else {
            "nesting depth cap exceeded"
        };
        assert_eq!(outcome.0[0].bail.as_deref(), Some(expected), "cap {cap}");
        assert_eq!(
            census.constant_fallbacks,
            u64::from(cap >= start),
            "cap {cap}"
        );
    }
}

fn high_source() -> String {
    common::read_fixture("obfuscator.io-high.js")
}

fn caps_below(total: u64, constants: &[ConstantRun]) -> Vec<u64> {
    let mut caps: Vec<u64> = (1..=64).map(|below| total.saturating_sub(below)).collect();
    caps.extend((2..=64).map(|step| total.saturating_sub(step * 64)));
    let stride = (constants.len() / 16).max(1);
    for run in constants.iter().step_by(stride).take(16) {
        let start = run.steps;
        caps.extend([
            start.saturating_sub(1),
            start,
            start + 1,
            start + run.weight / 2,
            start + run.weight.saturating_sub(2),
            start + run.weight - 1,
        ]);
    }
    caps.sort_unstable();
    caps.dedup();
    caps
}

fn high_preset_caps(unit: Unit) {
    let source = high_source();
    let (full, census) = both(&source, unit, None);
    assert!(census.constants > 0, "{unit:?}: no constant folded");
    let firsts: Vec<ConstantRun> = census
        .constant_runs
        .iter()
        .copied()
        .filter(|run| run.steps < steps(&full) && run.weight > 2)
        .collect();
    assert!(
        firsts.len() >= 16,
        "{unit:?}: {} constant runs",
        firsts.len()
    );
    let caps = caps_below(steps(&full), &firsts);
    let fallbacks: u64 = caps
        .par_iter()
        .map(|cap| both(&source, unit, Some(*cap)).1.constant_fallbacks)
        .sum();
    println!(
        "{unit:?}: {} steps, {} caps, {} constant runs, {} fallbacks, 0 differ",
        steps(&full),
        caps.len(),
        census.constant_runs.len(),
        fallbacks
    );
    assert!(fallbacks > 0, "{unit:?}: no cap landed inside a constant");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset under about 230 fuel caps"
)]
fn the_high_presets_decode_units_equal_the_walker_at_caps_near_the_end_and_inside_constants() {
    high_preset_caps(Unit::Decode);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset under about 230 fuel caps"
)]
fn the_high_preset_as_a_script_equals_the_walker_at_caps_near_the_end_and_inside_constants() {
    high_preset_caps(Unit::Script);
}
