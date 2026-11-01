use std::panic::{catch_unwind, panic_any, resume_unwind, AssertUnwindSafe};

use untwist_eval::stack;

use super::boundary::Boundary;
use crate::options::TestHooks;

pub const PIPELINE_STACK_BYTES: usize = 1 << 30;
pub(super) const LADDER: [usize; 4] = [PIPELINE_STACK_BYTES, 256 << 20, 64 << 20, 16 << 20];
const BODY_LOST: &str = "the stack grower consumed the body without running it";
const RUNG_REFUSED: &str = "a test hook refused this rung";

#[derive(Clone, Copy, Debug)]
pub(super) struct Ladder {
    rungs: [usize; 4],
    refused: usize,
}

impl Ladder {
    pub(super) fn new(hooks: TestHooks) -> Self {
        let mut rungs = LADDER;
        if let Some(first) = hooks.ladder_first {
            rungs[0] = first;
        }
        Self {
            rungs,
            refused: hooks.ladder_refuse,
        }
    }

    fn grower(&self) -> impl FnMut(usize, &mut dyn FnMut()) {
        let mut refusals_left = self.refused;
        move |size, callback| {
            if refusals_left > 0 {
                refusals_left -= 1;
                panic_any(RUNG_REFUSED);
            }
            stack::grow(size, callback);
        }
    }
}

pub(super) fn on_a_worker_stack<R>(ladder: &Ladder, body: impl FnOnce(usize) -> R) -> R {
    on_a_ladder_from(&ladder.rungs, ladder.grower(), body)
}

fn on_a_ladder_from<R>(
    rungs: &[usize],
    mut grow: impl FnMut(usize, &mut dyn FnMut()),
    body: impl FnOnce(usize) -> R,
) -> R {
    let mut pending = Some(body);
    let silenced = Boundary::enter();
    for &size in rungs {
        let mut ran = None;
        let grown = catch_unwind(AssertUnwindSafe(|| {
            grow(size, &mut || {
                let _heard = silenced.lifted();
                if let Some(body) = pending.take() {
                    ran = Some(body(size));
                }
            });
        }));
        match (ran, pending.is_some(), grown) {
            (Some(value), _, _) => return value,
            (None, true, _) => {}
            (None, false, Err(payload)) => resume_unwind(payload),
            (None, false, Ok(())) => resume_unwind(Box::new(BODY_LOST)),
        }
    }
    drop(silenced);
    match pending {
        Some(body) => body(0),
        None => resume_unwind(Box::new(BODY_LOST)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::boundary::is_silenced;
    use super::{on_a_ladder_from, Ladder, LADDER, PIPELINE_STACK_BYTES};
    use crate::options::TestHooks;
    use std::cell::{Cell, RefCell};
    use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
    use untwist_core::depth::{limits_for, Limits, C_P1, UNIT_STACK_BYTES};
    use untwist_eval::stack::{self, room};

    const MIB: usize = 1 << 20;
    const SIXTY_FOUR_MIB: usize = 64 << 20;

    fn on_a_worker_stack<R>(body: impl FnOnce(usize) -> R) -> R {
        super::on_a_worker_stack(&Ladder::new(TestHooks::default()), body)
    }

    fn a_grower_that_cannot_map_its_stack(_: usize, _: &mut dyn FnMut()) {
        panic_any("no stack");
    }

    fn a_grower_that_passes_the_body_through(_: usize, body: &mut dyn FnMut()) {
        body();
    }

    fn a_grower_that_maps_at_most(most: usize) -> impl FnMut(usize, &mut dyn FnMut()) {
        move |size, body| {
            if size > most {
                panic_any("no stack");
            }
            stack::grow(size, body);
        }
    }

    fn nesting_for(room: usize) -> u32 {
        let units = room / (2 * UNIT_STACK_BYTES);
        u32::try_from(units).map_or(100_000, |units| units.saturating_sub(C_P1).min(100_000))
    }

    #[inline(never)]
    fn descend_until_the_floor_then_grow() -> usize {
        let pad = std::hint::black_box([0_u8; 4096]);
        if stack::has_room() {
            return descend_until_the_floor_then_grow() + usize::from(pad[0]);
        }
        stack::maybe_grow(room)
    }

    #[test]
    fn the_worker_stack_gives_the_body_a_gibibyte_segment() {
        let remaining = on_a_worker_stack(|_| stacker::remaining_stack());
        assert!(remaining.is_some_and(|bytes| bytes >= PIPELINE_STACK_BYTES - MIB));
    }

    #[test]
    fn the_body_is_told_the_size_it_obtained_and_room_sees_almost_all_of_it() {
        let (obtained, left) = on_a_worker_stack(|obtained| (obtained, room()));
        assert_eq!(obtained, PIPELINE_STACK_BYTES);
        assert!(left >= PIPELINE_STACK_BYTES - 2 * MIB, "{left}");
    }

    #[test]
    fn a_host_refusing_the_two_largest_rungs_runs_the_body_once_on_sixty_four_mib() {
        let runs = Cell::new(0);
        let (obtained, nesting) = on_a_ladder_from(
            &LADDER,
            a_grower_that_maps_at_most(SIXTY_FOUR_MIB),
            |obtained| {
                runs.set(runs.get() + 1);
                (obtained, limits_for(room(), 0).nesting)
            },
        );
        assert_eq!((obtained, runs.get()), (SIXTY_FOUR_MIB, 1));
        assert!(
            nesting >= nesting_for(SIXTY_FOUR_MIB - MIB) && nesting <= nesting_for(SIXTY_FOUR_MIB)
        );
        #[cfg(not(debug_assertions))]
        assert!((8_000..=8_192).contains(&nesting), "{nesting}");
        #[cfg(debug_assertions)]
        assert!((3_990..=4_088).contains(&nesting), "{nesting}");
    }

    #[test]
    fn refused_rungs_are_silenced_and_the_body_is_not() {
        let seen = RefCell::new(Vec::new());
        let outer = is_silenced();
        on_a_ladder_from(
            &LADDER,
            |size, body: &mut dyn FnMut()| {
                seen.borrow_mut().push(is_silenced());
                a_grower_that_maps_at_most(SIXTY_FOUR_MIB)(size, body);
            },
            |_| seen.borrow_mut().push(is_silenced()),
        );
        assert_eq!(*seen.borrow(), [true, true, true, outer]);
        assert_eq!(is_silenced(), outer);
    }

    #[test]
    fn a_host_refusing_every_rung_runs_the_body_inline_once_with_zero() {
        let runs = Cell::new(0);
        let caller = std::thread::current().id();
        let (obtained, ran_on, left, remaining) =
            on_a_ladder_from(&LADDER, a_grower_that_cannot_map_its_stack, |obtained| {
                runs.set(runs.get() + 1);
                (
                    obtained,
                    std::thread::current().id(),
                    room(),
                    stacker::remaining_stack(),
                )
            });
        assert_eq!((obtained, ran_on, runs.get()), (0, caller, 1));
        let remaining = remaining.expect("the test thread's stack is known");
        assert!(remaining < SIXTY_FOUR_MIB);
        assert!(
            left <= remaining && left + MIB >= remaining,
            "{left} {remaining}"
        );
    }

    #[test]
    fn limits_follow_the_room_and_the_input_length() {
        assert_eq!(
            limits_for(1 << 30, 10),
            Limits {
                nesting: nesting_for(1 << 30),
                speculative_parens: 512,
                speculative_pairs: 1 << 17,
                print_bytes: 64 << 20,
            }
        );
        assert_eq!(
            limits_for(16 << 20, 1 << 30),
            Limits {
                nesting: nesting_for(16 << 20),
                speculative_parens: 512,
                speculative_pairs: 1 << 24,
                print_bytes: (1 << 31) - 1,
            }
        );
        #[cfg(not(debug_assertions))]
        assert_eq!(
            (
                limits_for(1 << 30, 10).nesting,
                limits_for(16 << 20, 1 << 30).nesting
            ),
            (100_000, 2_040)
        );
        #[cfg(debug_assertions)]
        assert_eq!(
            (
                limits_for(1 << 30, 10).nesting,
                limits_for(16 << 20, 1 << 30).nesting
            ),
            (65_528, 1_016)
        );
        assert_eq!(limits_for(0, 0).nesting, 0);
        assert_eq!(
            limits_for(usize::MAX, usize::MAX).print_bytes,
            (1 << 31) - 1
        );
    }

    #[test]
    fn a_one_mib_segment_grown_inside_the_gibibyte_segment_sees_only_what_is_left() {
        let left = on_a_worker_stack(|_| stack::grow(MIB, descend_until_the_floor_then_grow));
        assert!(left <= MIB, "{left}");
        assert!(left >= MIB / 2 - 64 * 1024, "{left}");
    }

    #[test]
    fn stackers_own_refusal_to_map_a_stack_runs_the_body_inline_once() {
        let runs = Cell::new(0);
        let value = on_a_ladder_from(
            &LADDER,
            |_, callback| stack::grow(usize::MAX, callback),
            |obtained| {
                runs.set(runs.get() + 1);
                (obtained, stacker::remaining_stack())
            },
        );
        assert_eq!(runs.get(), 1);
        assert_eq!(value.0, 0);
        assert!(value.1.is_some_and(|bytes| bytes < PIPELINE_STACK_BYTES));
    }

    #[test]
    fn a_grower_that_never_calls_the_body_still_gets_it_run_once() {
        let runs = Cell::new(0);
        let value = on_a_ladder_from(
            &LADDER,
            |_, _| {},
            |_| {
                runs.set(runs.get() + 1);
                7
            },
        );
        assert_eq!((value, runs.get()), (7, 1));
    }

    #[test]
    fn a_panicking_body_propagates_its_own_payload_and_runs_once() {
        let runs = Cell::new(0);
        let caught = catch_unwind(AssertUnwindSafe(|| {
            on_a_ladder_from(&LADDER, a_grower_that_passes_the_body_through, |_| {
                runs.set(runs.get() + 1);
                panic_any(41_u32)
            })
        }));
        let payload = caught
            .err()
            .and_then(|payload| payload.downcast::<u32>().ok());
        assert_eq!((payload.map(|boxed| *boxed), runs.get()), (Some(41), 1));
    }

    #[test]
    fn a_panicking_body_on_the_worker_stack_propagates_its_own_payload_and_runs_once() {
        let runs = Cell::new(0);
        let caught = catch_unwind(AssertUnwindSafe(|| {
            on_a_worker_stack(|_| {
                runs.set(runs.get() + 1);
                panic_any(43_u32)
            })
        }));
        let payload = caught
            .err()
            .and_then(|payload| payload.downcast::<u32>().ok());
        assert_eq!((payload.map(|boxed| *boxed), runs.get()), (Some(43), 1));
    }

    #[test]
    fn the_worker_stack_runs_the_body_on_the_callers_thread() {
        let caller = std::thread::current().id();
        assert_eq!(on_a_worker_stack(|_| std::thread::current().id()), caller);
    }

    #[test]
    fn the_ladder_hooks_refuse_rungs_and_replace_the_first() {
        let refusing = Ladder::new(TestHooks {
            ladder_refuse: 2,
            ..TestHooks::default()
        });
        assert_eq!(
            super::on_a_worker_stack(&refusing, |obtained| obtained),
            SIXTY_FOUR_MIB
        );
        let every = Ladder::new(TestHooks {
            ladder_refuse: 4,
            ..TestHooks::default()
        });
        assert_eq!(super::on_a_worker_stack(&every, |obtained| obtained), 0);
        let first = Ladder::new(TestHooks {
            ladder_first: Some(SIXTY_FOUR_MIB),
            ..TestHooks::default()
        });
        assert_eq!(
            super::on_a_worker_stack(&first, |obtained| obtained),
            SIXTY_FOUR_MIB
        );
    }
}
