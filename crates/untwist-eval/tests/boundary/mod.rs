#![allow(dead_code)]

use std::hint::black_box;

use untwist_eval::budget::{CALL_STACK_GROWTH, CALL_STACK_RED_ZONE};
use untwist_eval::stack;

pub const FRAME_PAD_BYTES: usize = 1024;
pub const BOUNDARY_TOLERANCE_BYTES: usize = 8 * 1024;
pub const LEVEL_CAP: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boundary {
    pub levels: usize,
    pub remaining: Option<usize>,
}

pub fn stacker_says_room() -> bool {
    stacker::remaining_stack().is_some_and(|remaining| remaining >= CALL_STACK_RED_ZONE)
}

pub fn assert_the_answer_is_stackers(context: &str) {
    let room = stack::has_room();
    let remaining = stacker::remaining_stack();
    assert!(
        !room || stacker_says_room(),
        "room answered where stacker would grow, {context}: {remaining:?}"
    );
    assert!(
        room || remaining
            .is_none_or(|bytes| bytes < CALL_STACK_RED_ZONE + BOUNDARY_TOLERANCE_BYTES),
        "no room answered far above stacker's boundary, {context}: {remaining:?}"
    );
}

pub fn assert_at_the_grow_boundary(reached: &Boundary) {
    assert!(reached.levels > 0, "no level had room: {reached:?}");
    assert!(
        reached.levels < LEVEL_CAP,
        "never reached a boundary: {reached:?}"
    );
    assert!(
        reached
            .remaining
            .is_some_and(|bytes| bytes < CALL_STACK_RED_ZONE + BOUNDARY_TOLERANCE_BYTES),
        "stopped far above stacker's boundary: {reached:?}"
    );
}

pub fn descend_to_the_boundary() -> Boundary {
    let reached = descend_through_segments(0, 0);
    match reached.first() {
        Some(first) => *first,
        None => panic!("the descent reports its own boundary"),
    }
}

#[inline(never)]
pub fn descend_through_segments(level: usize, segments: usize) -> Vec<Boundary> {
    let pad = black_box([0_u8; FRAME_PAD_BYTES]);
    if stack::has_room() && level < LEVEL_CAP {
        assert!(
            stacker_says_room(),
            "room answered at level {level} where stacker would grow"
        );
        let below = descend_through_segments(level + 1, segments);
        black_box(&pad);
        return below;
    }
    let here = Boundary {
        levels: level,
        remaining: stacker::remaining_stack(),
    };
    let deeper = if segments == 0 {
        Vec::new()
    } else {
        stack::grow(CALL_STACK_GROWTH, || {
            descend_through_segments(0, segments - 1)
        })
    };
    assert_the_answer_is_stackers(&format!("after a grown segment returned to level {level}"));
    black_box(&pad);
    std::iter::once(here).chain(deeper).collect()
}
