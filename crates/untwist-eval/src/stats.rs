use std::cell::Cell;

use crate::budget::Budget;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecodeStats {
    pub steps: u64,
    pub peak_unit_steps: u64,
    pub units: u32,
    pub peak_call_depth: u32,
}

impl DecodeStats {
    pub fn merged(self, other: DecodeStats) -> DecodeStats {
        DecodeStats {
            steps: self.steps.saturating_add(other.steps),
            peak_unit_steps: self.peak_unit_steps.max(other.peak_unit_steps),
            units: self.units.saturating_add(other.units),
            peak_call_depth: self.peak_call_depth.max(other.peak_call_depth),
        }
    }
}

thread_local! {
    static RECORDED: Cell<DecodeStats> = const {
        Cell::new(DecodeStats {
            steps: 0,
            peak_unit_steps: 0,
            units: 0,
            peak_call_depth: 0,
        })
    };
}

pub fn record(budget: &Budget) {
    let steps = budget.steps_used();
    let unit = DecodeStats {
        steps,
        peak_unit_steps: steps,
        units: 1,
        peak_call_depth: budget.peak_call_depth(),
    };
    RECORDED.with(|recorded| recorded.set(recorded.get().merged(unit)));
}

pub fn take() -> DecodeStats {
    RECORDED.with(|recorded| recorded.replace(DecodeStats::default()))
}
