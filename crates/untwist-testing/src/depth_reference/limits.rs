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
    u32::try_from(units)
        .unwrap_or(u32::MAX)
        .saturating_sub(C_P1)
        .min(NESTING_CEILING)
}
