#[cfg(not(target_arch = "wasm32"))]
use super::DepthRefusal;
use super::Limits;

#[cfg(not(debug_assertions))]
pub const UNIT_STACK_BYTES: usize = 4096;
#[cfg(debug_assertions)]
pub const UNIT_STACK_BYTES: usize = 8192;

pub const C_P1: u32 = 8;

pub const LIFT_DEPTH_MAX: usize = 64;

const NESTING_CEILING: u32 = 100_000;
const SPECULATIVE_PARENS: u32 = 512;
const SPECULATIVE_PAIRS_FLOOR: u64 = 1 << 17;
const SPECULATIVE_PAIRS_DIVISOR: u64 = 64;
const PRINT_FLOOR: u64 = 64 << 20;
const PRINT_BYTES_PER_INPUT_BYTE: u64 = 32;
const PRINT_CEILING: u64 = (1 << 31) - 1;

pub const WASM_V8_STACK_BYTES: u32 = 984 << 10;
pub const WASM_V8_ENTRY_BYTES: u32 = 23 << 10;
pub const WASM_V8_UNIT_BYTES: u32 = 1_470;
pub const WASM_SHADOW_STACK_BYTES: u32 = 1 << 20;
pub const WASM_SHADOW_ENTRY_BYTES: u32 = 8 << 10;
pub const WASM_SHADOW_UNIT_BYTES: u32 = 1_325;
pub const WASM_V8_ROOM: u32 = WASM_V8_STACK_BYTES - WASM_V8_ENTRY_BYTES;
pub const WASM_NESTING: u32 = smaller(
    stack_units(WASM_V8_ROOM, WASM_V8_UNIT_BYTES),
    stack_units(
        WASM_SHADOW_STACK_BYTES - WASM_SHADOW_ENTRY_BYTES,
        WASM_SHADOW_UNIT_BYTES,
    ),
);

#[cfg(not(target_arch = "wasm32"))]
#[inline(always)]
pub(super) fn noted<T>(
    outcome: Result<T, DepthRefusal>,
    _limits: &Limits,
) -> Result<T, DepthRefusal> {
    outcome
}

const fn stack_units(room: u32, unit: u32) -> u32 {
    (room / unit).saturating_sub(C_P1)
}

const fn smaller(a: u32, b: u32) -> u32 {
    if a < b {
        a
    } else {
        b
    }
}

pub fn limits_for(room: usize, input_len: usize) -> Limits {
    let len = u64::try_from(input_len).unwrap_or(u64::MAX);
    Limits {
        nesting: nesting_for(room),
        speculative_parens: SPECULATIVE_PARENS,
        speculative_pairs: (len / SPECULATIVE_PAIRS_DIVISOR).max(SPECULATIVE_PAIRS_FLOOR),
        print_bytes: len
            .saturating_mul(PRINT_BYTES_PER_INPUT_BYTE)
            .clamp(PRINT_FLOOR, PRINT_CEILING),
    }
}

fn nesting_for(room: usize) -> u32 {
    let units = room / (2 * UNIT_STACK_BYTES);
    let nesting = u32::try_from(units)
        .unwrap_or(u32::MAX)
        .saturating_sub(C_P1)
        .min(NESTING_CEILING);
    #[cfg(target_arch = "wasm32")]
    let nesting = nesting.min(WASM_NESTING);
    nesting
}

#[cfg(test)]
mod tests {
    use super::*;

    const V8_WORST_UNIT_BYTES: u32 = 639;
    const V8_WORST_ENTRY_BYTES: u32 = 23_173;
    const SHADOW_WORST_UNIT_BYTES: u32 = 576;
    const SHADOW_WORST_ENTRY_BYTES: u32 = 7_618;

    fn holds_the_margin(unit: u32, worst: u32) -> bool {
        u64::from(unit) * 10 >= u64::from(worst) * 23
    }

    #[test]
    fn each_wasm_unit_is_at_least_two_point_three_times_its_worst_measured_unit() {
        assert!(holds_the_margin(WASM_V8_UNIT_BYTES, V8_WORST_UNIT_BYTES));
        assert!(holds_the_margin(
            WASM_SHADOW_UNIT_BYTES,
            SHADOW_WORST_UNIT_BYTES
        ));
        assert!(!holds_the_margin(
            WASM_V8_UNIT_BYTES - 1,
            V8_WORST_UNIT_BYTES
        ));
        assert!(!holds_the_margin(
            WASM_SHADOW_UNIT_BYTES - 1,
            SHADOW_WORST_UNIT_BYTES
        ));
    }

    #[test]
    fn each_wasm_entry_allowance_covers_the_measured_entry() {
        let covered = [
            (WASM_V8_ENTRY_BYTES, V8_WORST_ENTRY_BYTES),
            (WASM_SHADOW_ENTRY_BYTES, SHADOW_WORST_ENTRY_BYTES),
        ];
        for (allowance, measured) in covered {
            assert!(allowance >= measured, "{allowance} < {measured}");
        }
    }

    #[test]
    fn the_wasm_budget_is_v8s_room_over_its_unit_and_the_shadow_stack_does_not_bind() {
        assert_eq!(WASM_NESTING, 661);
        assert_eq!(WASM_NESTING, WASM_V8_ROOM / WASM_V8_UNIT_BYTES - C_P1);
        assert!(
            stack_units(
                WASM_SHADOW_STACK_BYTES - WASM_SHADOW_ENTRY_BYTES,
                WASM_SHADOW_UNIT_BYTES
            ) > WASM_NESTING
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_native_limit_is_untouched_by_the_wasm_budget() {
        for room in [16_usize << 20, 1 << 30] {
            let units = u32::try_from(room / (2 * UNIT_STACK_BYTES)).unwrap_or(u32::MAX);
            let native = (units - C_P1).min(NESTING_CEILING);
            assert_eq!(limits_for(room, 0).nesting, native);
            assert!(native > WASM_NESTING);
        }
    }
}
