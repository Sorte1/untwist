use untwist_eval::bail::Bail;
use untwist_eval::budget::{
    loop_cap, Budget, CALL_DEPTH_CAP, MIN_LOOP_ITERATIONS, NESTING_DEPTH_CAP, STEP_BUDGET,
};

#[test]
fn budget_spends_down_to_zero_then_bails() {
    let mut budget = Budget::new(10);
    assert!(budget.spend(4).is_ok());
    assert_eq!(budget.steps_used(), 4);
    assert!(budget.spend(6).is_ok());
    assert!(matches!(budget.spend(1), Err(Bail::Fuel)));
}

#[test]
fn call_depth_is_capped() {
    let mut budget = Budget::default();
    for _ in 0..CALL_DEPTH_CAP {
        assert!(budget.enter_call().is_ok());
    }
    assert!(matches!(budget.enter_call(), Err(Bail::CallDepth)));
    budget.leave_call();
    assert!(budget.enter_call().is_ok());
}

#[test]
fn nesting_depth_is_capped() {
    let mut budget = Budget::default();
    for _ in 0..NESTING_DEPTH_CAP {
        assert!(budget.enter_nesting().is_ok());
    }
    assert!(matches!(budget.enter_nesting(), Err(Bail::NestingDepth)));
    budget.leave_nesting();
    assert!(budget.enter_nesting().is_ok());
}

#[test]
fn loop_cap_is_four_times_length_with_a_floor() {
    assert_eq!(loop_cap(0), MIN_LOOP_ITERATIONS);
    assert_eq!(loop_cap(10), MIN_LOOP_ITERATIONS);
    assert_eq!(loop_cap(1_000_000), 4_000_000);
}

#[test]
fn default_budget_uses_the_spec_step_count() {
    let mut budget = Budget::default();
    assert!(budget.spend(STEP_BUDGET).is_ok());
    assert!(matches!(budget.spend(1), Err(Bail::Fuel)));
}

#[test]
fn bail_reasons_render_for_diagnostics() {
    assert_eq!(Bail::Fuel.to_string(), "step budget exhausted");
    assert_eq!(
        Bail::Unsupported(&"class declaration").to_string(),
        "unsupported construct: class declaration"
    );
}

#[test]
fn an_unlimited_budget_neither_overflows_nor_wraps() {
    let mut budget = Budget::new(u64::MAX);
    assert_eq!(budget.steps_used(), 0);
    assert!(budget.spend(u64::MAX - 1).is_ok());
    assert_eq!(budget.steps_used(), u64::MAX - 1);
    assert_eq!(budget.steps_left(), 1);
    assert!(matches!(budget.spend(2), Err(Bail::Fuel)));
    assert_eq!(budget.steps_left(), 0);
    assert_eq!(budget.steps_used(), u64::MAX);

    let mut fresh = Budget::new(u64::MAX);
    for _ in 0..1_000 {
        assert!(fresh.spend(STEP_BUDGET).is_ok());
    }
    assert_eq!(fresh.steps_used(), STEP_BUDGET * 1_000);
    assert!(matches!(fresh.spend(u64::MAX), Err(Bail::Fuel)));
}

#[test]
fn the_peak_call_depth_is_the_deepest_the_budget_went() {
    let mut budget = Budget::new(10);
    assert_eq!(budget.peak_call_depth(), 0);
    budget.enter_call().unwrap();
    budget.enter_call().unwrap();
    budget.enter_call().unwrap();
    budget.leave_call();
    budget.leave_call();
    budget.enter_call().unwrap();
    assert_eq!(budget.call_depth(), 2);
    assert_eq!(budget.peak_call_depth(), 3);
}

#[test]
fn decode_stats_sum_the_steps_and_keep_the_peaks_until_taken() {
    let _ = untwist_eval::stats::take();
    let mut first = Budget::new(100);
    first.spend(40).unwrap();
    first.enter_call().unwrap();
    let mut second = Budget::new(100);
    second.spend(70).unwrap();
    second.enter_call().unwrap();
    second.enter_call().unwrap();
    untwist_eval::stats::record(&first);
    untwist_eval::stats::record(&second);
    let taken = untwist_eval::stats::take();
    assert_eq!(taken.steps, 110);
    assert_eq!(taken.peak_unit_steps, 70);
    assert_eq!(taken.units, 2);
    assert_eq!(taken.peak_call_depth, 2);
    assert_eq!(
        untwist_eval::stats::take(),
        untwist_eval::stats::DecodeStats::default()
    );
}
