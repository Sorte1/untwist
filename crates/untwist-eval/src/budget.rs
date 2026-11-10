use untwist_core::depth::WASM_V8_ROOM;

use crate::bail::{Bail, EvalResult};

pub const STEP_BUDGET: u64 = 50_000_000;
pub const UNSAFE_STEP_BUDGET: u64 = 1_000_000_000;
pub const SAFE_STEPS_PER_DISTINCT_CALL: u64 = 500_000;

pub fn default_fuel(unsafe_transforms: bool) -> u64 {
    if unsafe_transforms {
        UNSAFE_STEP_BUDGET
    } else {
        STEP_BUDGET
    }
}

pub fn safe_unit_fuel(distinct_calls: usize) -> u64 {
    u64::try_from(distinct_calls)
        .unwrap_or(u64::MAX)
        .saturating_mul(SAFE_STEPS_PER_DISTINCT_CALL)
        .saturating_add(STEP_BUDGET)
        .min(UNSAFE_STEP_BUDGET)
}
#[cfg(not(target_arch = "wasm32"))]
pub const CALL_DEPTH_CAP: u32 = 1_000;
#[cfg(target_arch = "wasm32")]
pub const CALL_DEPTH_CAP: u32 = WASM_CALL_DEPTH_CAP;
#[cfg(not(target_arch = "wasm32"))]
pub const NESTING_DEPTH_CAP: u32 = 100_000;
#[cfg(target_arch = "wasm32")]
pub const NESTING_DEPTH_CAP: u32 = WASM_NESTING_DEPTH_CAP;
pub const WASM_CALL_BYTES: u32 = 3_043;
pub const WASM_NESTING_BYTES: u32 = 976;
pub const WASM_CALL_DEPTH_CAP: u32 = WASM_V8_ROOM / WASM_CALL_BYTES;
pub const WASM_NESTING_DEPTH_CAP: u32 = WASM_V8_ROOM / WASM_NESTING_BYTES;
pub const STRING_LEN_CAP: usize = 16 << 20;
pub const ARRAY_LEN_CAP: usize = 1 << 20;
pub const BOUND_CHAIN_CAP: usize = 64;
pub const MIN_LOOP_ITERATIONS: u64 = 100_000;
pub const CALL_STACK_RED_ZONE: usize = 64 * 1024;
pub const CALL_STACK_GROWTH: usize = 1024 * 1024;
pub const REGEXP_STEP_CAP: usize = 1 << 22;
pub const REGEXP_STEPS_PER_FUEL: usize = 4;
pub const HEAP_BUDGET_FLOOR: u64 = 64 << 20;
pub const HEAP_BYTES_PER_INPUT_BYTE: u64 = 16;
pub const STRING_UNIT_BYTES: u64 = 2;
pub const STRING_HEADER_BYTES: u64 = 96;
pub const ARRAY_SLOT_BYTES: u64 = 16;
pub const ARRAY_HEADER_BYTES: u64 = 16;
pub const OBJECT_BYTES: u64 = 256;
pub const CLOSURE_BYTES: u64 = 80;
pub const PROP_BYTES: u64 = 48;
pub const PROPS_HEADER_BYTES: u64 = 64;
pub const ENV_BYTES: u64 = 192;
pub const BINDING_BYTES: u64 = 48;

#[cfg(target_arch = "wasm32")]
static WASM_CAP_BAILS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[cfg(target_arch = "wasm32")]
pub fn wasm_cap_bails() -> u32 {
    WASM_CAP_BAILS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(target_arch = "wasm32")]
fn note_wasm_cap_bail() {
    WASM_CAP_BAILS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub fn heap_budget(input_len: usize) -> u64 {
    u64::try_from(input_len)
        .unwrap_or(u64::MAX)
        .saturating_mul(HEAP_BYTES_PER_INPUT_BYTE)
        .max(HEAP_BUDGET_FLOOR)
}

pub fn props_bytes(before: usize, after: usize) -> u64 {
    if after <= before {
        return 0;
    }
    let header = if before == 0 { PROPS_HEADER_BYTES } else { 0 };
    ((after - before) as u64)
        .saturating_mul(PROP_BYTES)
        .saturating_add(header)
}

pub fn loop_cap(array_len: usize) -> u64 {
    let scaled = (array_len as u64).saturating_mul(4);
    scaled.max(MIN_LOOP_ITERATIONS)
}

const _: () = assert!(CALL_DEPTH_CAP <= u16::MAX as u32);

pub const STEP_SLICE: u64 = 1 << 20;

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    steps_left: u64,
    steps_held: u64,
    steps_total: u64,
    depth: u16,
    peak_depth: u16,
    nesting: u32,
}

impl Budget {
    pub fn new(steps: u64) -> Self {
        let slice = steps.min(STEP_SLICE);
        Self {
            steps_left: slice,
            steps_held: steps - slice,
            steps_total: steps,
            depth: 0,
            peak_depth: 0,
            nesting: 0,
        }
    }

    #[inline]
    pub fn spend(&mut self, steps: u64) -> EvalResult<()> {
        match self.steps_left.checked_sub(steps) {
            Some(left) => {
                self.steps_left = left;
                Ok(())
            }
            None => self.spend_past_the_slice(steps),
        }
    }

    #[cold]
    fn spend_past_the_slice(&mut self, steps: u64) -> EvalResult<()> {
        let Some(after) = self.steps_left().checked_sub(steps) else {
            self.steps_left = 0;
            self.steps_held = 0;
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_bail_nesting(self.nesting);
            return Err(Bail::Fuel);
        };
        crate::heap::spent_run_budget()?;
        self.steps_left = after.min(STEP_SLICE);
        self.steps_held = after - self.steps_left;
        Ok(())
    }

    pub fn enter_call(&mut self) -> EvalResult<()> {
        if u32::from(self.depth) >= CALL_DEPTH_CAP {
            #[cfg(target_arch = "wasm32")]
            note_wasm_cap_bail();
            return Err(Bail::CallDepth);
        }
        self.depth += 1;
        self.peak_depth = self.peak_depth.max(self.depth);
        Ok(())
    }

    pub fn leave_call(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    pub fn enter_nesting(&mut self) -> EvalResult<()> {
        if self.nesting >= NESTING_DEPTH_CAP {
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_bail_nesting(self.nesting);
            #[cfg(target_arch = "wasm32")]
            note_wasm_cap_bail();
            return Err(Bail::NestingDepth);
        }
        self.nesting += 1;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn deepen(&mut self) {
        self.nesting += 1;
    }

    #[inline(always)]
    pub(crate) fn restore_nesting(&mut self, level: u32) {
        self.nesting = level;
    }

    pub fn leave_nesting(&mut self) {
        self.nesting = self.nesting.saturating_sub(1);
    }

    pub fn call_depth(&self) -> u32 {
        u32::from(self.depth)
    }

    pub fn peak_call_depth(&self) -> u32 {
        u32::from(self.peak_depth)
    }

    pub fn nesting_depth(&self) -> u32 {
        self.nesting
    }

    pub fn steps_used(&self) -> u64 {
        self.steps_total - self.steps_left()
    }

    pub fn steps_left(&self) -> u64 {
        self.steps_left + self.steps_held
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::new(STEP_BUDGET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WASM_WORST_CALL_BYTES: u32 = 1_323;
    const WASM_WORST_NESTING_BYTES: u32 = 424;

    fn holds_the_margin(bytes: u32, worst: u32) -> bool {
        u64::from(bytes) * 10 >= u64::from(worst) * 23
    }

    #[test]
    fn the_wasm_caps_hold_two_point_three_times_the_worst_measured_frame() {
        assert!(holds_the_margin(WASM_CALL_BYTES, WASM_WORST_CALL_BYTES));
        assert!(holds_the_margin(
            WASM_NESTING_BYTES,
            WASM_WORST_NESTING_BYTES
        ));
        assert!(!holds_the_margin(
            WASM_CALL_BYTES - 1,
            WASM_WORST_CALL_BYTES
        ));
        assert!(!holds_the_margin(
            WASM_NESTING_BYTES - 1,
            WASM_WORST_NESTING_BYTES
        ));
        assert_eq!((WASM_CALL_DEPTH_CAP, WASM_NESTING_DEPTH_CAP), (323, 1_008));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_native_caps_are_untouched_by_the_wasm_caps() {
        assert_eq!((CALL_DEPTH_CAP, NESTING_DEPTH_CAP), (1_000, 100_000));
    }
}
