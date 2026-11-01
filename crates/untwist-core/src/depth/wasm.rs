use std::sync::atomic::{AtomicU32, Ordering};

use super::limits::WASM_NESTING;
use super::{DepthRefusal, Limits};

const NO_TEXT_CAP: u32 = u32::MAX;

static BUDGET_REFUSALS: AtomicU32 = AtomicU32::new(0);
static INPUT_NESTING: AtomicU32 = AtomicU32::new(WASM_NESTING);
static BINDING_TEXT_CAP: AtomicU32 = AtomicU32::new(NO_TEXT_CAP);

pub fn wasm_budget_refusals() -> u32 {
    BUDGET_REFUSALS.load(Ordering::Relaxed)
}

pub fn set_wasm_input_nesting(nesting: u32) {
    INPUT_NESTING.store(nesting, Ordering::Relaxed);
}

pub fn capped_above_the_input(nesting: u32, shares: u32) -> u32 {
    let room = WASM_NESTING.saturating_sub(INPUT_NESTING.load(Ordering::Relaxed));
    let cap = room / shares.max(1);
    let binding = if cap < nesting { cap } else { NO_TEXT_CAP };
    BINDING_TEXT_CAP.store(binding, Ordering::Relaxed);
    nesting.min(cap)
}

fn set_by_the_wasm_budget(limits: &Limits) -> bool {
    let binding = BINDING_TEXT_CAP.swap(NO_TEXT_CAP, Ordering::Relaxed);
    limits.nesting == WASM_NESTING || limits.nesting == binding
}

pub(super) fn noted<T>(
    outcome: Result<T, DepthRefusal>,
    limits: &Limits,
) -> Result<T, DepthRefusal> {
    let binding = set_by_the_wasm_budget(limits);
    if binding && matches!(outcome, Err(DepthRefusal::Nesting)) {
        BUDGET_REFUSALS.fetch_add(1, Ordering::Relaxed);
    }
    outcome
}
