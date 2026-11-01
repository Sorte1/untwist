use std::cell::Cell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicU8;

use crate::budget::{CALL_STACK_GROWTH, CALL_STACK_RED_ZONE};

const FLOOR_MARGIN: usize = CALL_STACK_RED_ZONE;
pub(crate) const SPAN_BYTES: usize = CALL_STACK_GROWTH / 2;

static NEVER_A_STACK: [AtomicU8; SPAN_BYTES] = [const { AtomicU8::new(0) }; SPAN_BYTES];
const NO_SPAN: *const u8 = (&raw const NEVER_A_STACK).cast::<u8>();

thread_local! {
    static FLOOR: Cell<*const u8> = const { Cell::new(NO_SPAN) };
    static SEGMENTS_ENTERED: Cell<usize> = const { Cell::new(0) };
}

#[doc(hidden)]
pub fn segments_entered_on_this_thread_for_tests() -> usize {
    SEGMENTS_ENTERED.get()
}

#[inline(always)]
pub fn has_room() -> bool {
    let marker = MaybeUninit::<u8>::uninit();
    if marker.as_ptr().addr().wrapping_sub(FLOOR.get().addr()) < SPAN_BYTES {
        return true;
    }
    stackers_answer()
}

#[inline(always)]
pub fn has_room_for(bytes: usize) -> bool {
    let marker = MaybeUninit::<u8>::uninit();
    let above = marker.as_ptr().addr().wrapping_sub(FLOOR.get().addr());
    if above < SPAN_BYTES && above >= bytes {
        return true;
    }
    stackers_room_for(bytes)
}

#[cold]
#[inline(never)]
fn stackers_room_for(bytes: usize) -> bool {
    stacker::remaining_stack().is_some_and(|left| left >= bytes.saturating_add(CALL_STACK_RED_ZONE))
}

#[cold]
#[inline(never)]
fn stackers_answer() -> bool {
    stacker::remaining_stack().is_some_and(|bytes| bytes >= CALL_STACK_RED_ZONE)
}

#[inline(never)]
fn floor_below(top: usize) -> *const u8 {
    let here = psm::stack_pointer().addr();
    match floor_from(top, here, stacker::remaining_stack()) {
        Some(floor) => NO_SPAN.with_addr(floor),
        None => NO_SPAN,
    }
}

fn floor_from(top: usize, here: usize, remaining: Option<usize>) -> Option<usize> {
    let lowest = remaining
        .filter(|bytes| *bytes > 0)
        .and_then(|bytes| here.checked_sub(bytes))
        .and_then(|limit| limit.checked_add(CALL_STACK_RED_ZONE + FLOOR_MARGIN))?;
    top.checked_sub(SPAN_BYTES).filter(|floor| *floor >= lowest)
}

struct RestoreFloorOnExit(*const u8);

impl Drop for RestoreFloorOnExit {
    fn drop(&mut self) {
        FLOOR.set(self.0);
    }
}

pub fn grow<R>(stack_size: usize, callback: impl FnOnce() -> R) -> R {
    let _restore_on_exit = RestoreFloorOnExit(FLOOR.get());
    stacker::grow(stack_size, || {
        let entry = MaybeUninit::<u8>::uninit();
        FLOOR.set(floor_below(entry.as_ptr().addr()));
        SEGMENTS_ENTERED.set(SEGMENTS_ENTERED.get().saturating_add(1));
        callback()
    })
}

pub fn on_this_threads_stack<R>(body: impl FnOnce() -> R) -> R {
    let _restore_on_exit = RestoreFloorOnExit(FLOOR.get());
    let entry = MaybeUninit::<u8>::uninit();
    FLOOR.set(floor_below(entry.as_ptr().addr()));
    body()
}

pub fn room() -> usize {
    stacker::remaining_stack().map_or(0, |bytes| bytes.saturating_sub(SPAN_BYTES))
}

#[inline(never)]
pub fn maybe_grow_dyn(callback: &mut dyn FnMut()) {
    if has_room() {
        callback();
    } else {
        grow(CALL_STACK_GROWTH, callback);
    }
}

#[inline(always)]
pub fn maybe_grow<R>(callback: impl FnOnce() -> R) -> R {
    if has_room() {
        callback()
    } else {
        grow(CALL_STACK_GROWTH, callback)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        floor_from, grow, has_room, on_this_threads_stack, FLOOR, NEVER_A_STACK, NO_SPAN,
        SPAN_BYTES,
    };
    use crate::budget::{CALL_STACK_GROWTH, CALL_STACK_RED_ZONE};
    use std::mem::MaybeUninit;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    const A_FLOOR_THAT_CLAIMS_ROOM_NOWHERE_ON_A_STACK: *const u8 = NO_SPAN;

    #[inline(never)]
    fn a_local_address() -> usize {
        let marker = MaybeUninit::<u8>::uninit();
        std::hint::black_box(marker.as_ptr()).addr()
    }

    fn a_foreign_floor() -> *const u8 {
        NO_SPAN.wrapping_add(1)
    }

    fn in_the_span(here: usize) -> bool {
        here.wrapping_sub(FLOOR.get().addr()) < SPAN_BYTES
    }

    fn absent_or_above_two_red_zones() -> bool {
        let floor = FLOOR.get();
        let here = a_local_address();
        floor == NO_SPAN
            || stacker::remaining_stack()
                .and_then(|bytes| here.checked_sub(bytes))
                .is_some_and(|limit| floor.addr() >= limit + 2 * CALL_STACK_RED_ZONE)
    }

    fn on_a_rayon_worker<R: Send>(stack_bytes: usize, body: impl FnOnce() -> R + Send) -> R {
        match rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .stack_size(stack_bytes)
            .build()
        {
            Ok(pool) => pool.install(body),
            Err(error) => panic!("a one-thread pool builds: {error}"),
        }
    }

    #[test]
    fn a_floor_sits_one_span_below_the_segments_entry_and_two_red_zones_above_stackers_limit() {
        let limit = 0x7000_0000;
        let here = limit + 0x0080_0000;
        let top = here + 0x100;
        assert_eq!(
            floor_from(top, here, Some(here - limit)),
            Some(top - SPAN_BYTES)
        );
        let small_top = limit + 2 * CALL_STACK_RED_ZONE + SPAN_BYTES;
        assert_eq!(
            floor_from(
                small_top,
                small_top - 0x100,
                Some(small_top - 0x100 - limit)
            ),
            Some(limit + 2 * CALL_STACK_RED_ZONE)
        );
        assert_eq!(
            floor_from(
                small_top - 1,
                small_top - 0x100,
                Some(small_top - 0x100 - limit)
            ),
            None
        );
    }

    #[test]
    fn a_floor_that_cannot_be_measured_answers_nothing_from_the_cache() {
        assert_eq!(floor_from(0x10_0000, 0x1000, None), None);
        assert_eq!(floor_from(0x10_0000, 0x1000, Some(0)), None);
        assert_eq!(floor_from(0x10_0000, 0x1000, Some(0x2000)), None);
        assert_eq!(floor_from(usize::MAX, usize::MAX, Some(1)), None);
        assert_eq!(floor_from(0x1000, 0x800, Some(0x100)), None);
    }

    #[test]
    fn the_span_without_a_segment_is_a_static_so_no_stack_address_is_in_it() {
        let start = NEVER_A_STACK.as_ptr().addr();
        assert_eq!(NO_SPAN.addr(), start);
        assert_eq!(std::mem::size_of_val(&NEVER_A_STACK), SPAN_BYTES);
        assert!(a_local_address().wrapping_sub(start) >= SPAN_BYTES);
    }

    #[test]
    fn a_grown_segment_installs_its_own_floor_and_restores_the_callers_on_exit() {
        FLOOR.set(a_foreign_floor());
        let (inside, here, remaining) = grow(CALL_STACK_GROWTH, || {
            (
                FLOOR.get().addr(),
                a_local_address(),
                stacker::remaining_stack(),
            )
        });
        let restored = FLOOR.get();
        FLOOR.set(NO_SPAN);
        assert_eq!(restored, a_foreign_floor());
        let limit = remaining.and_then(|bytes| here.checked_sub(bytes));
        assert!(limit.is_some_and(|limit| inside >= limit + 2 * CALL_STACK_RED_ZONE));
        assert!(here.wrapping_sub(inside) < SPAN_BYTES);
    }

    #[test]
    fn a_segment_grown_inside_a_segment_hands_the_outer_floor_back() {
        let (outer, inner, restored) = grow(CALL_STACK_GROWTH, || {
            let outer = FLOOR.get();
            let inner = grow(CALL_STACK_GROWTH, || FLOOR.get());
            (outer, inner, FLOOR.get())
        });
        assert_ne!(outer, NO_SPAN);
        assert_ne!(inner, NO_SPAN);
        assert_ne!(outer, inner);
        assert_eq!(restored, outer);
        assert_eq!(FLOOR.get(), NO_SPAN);
    }

    #[test]
    fn a_segment_too_small_for_a_span_answers_as_stacker_does() {
        let inside = grow(SPAN_BYTES, || (FLOOR.get(), has_room()));
        assert_eq!(inside, (A_FLOOR_THAT_CLAIMS_ROOM_NOWHERE_ON_A_STACK, true));
    }

    #[test]
    fn a_panic_out_of_a_grown_segment_restores_the_callers_floor() {
        FLOOR.set(a_foreign_floor());
        let caught = catch_unwind(AssertUnwindSafe(|| {
            grow(CALL_STACK_GROWTH, || {
                assert!(has_room());
                std::panic::panic_any(FLOOR.get().addr())
            })
        }));
        let restored = FLOOR.get();
        FLOOR.set(NO_SPAN);
        let inside = caught
            .err()
            .and_then(|payload| payload.downcast::<usize>().ok());
        assert!(inside.is_some_and(|floor| *floor != NO_SPAN.addr()));
        assert_eq!(restored, a_foreign_floor());
    }

    #[test]
    fn a_threads_own_stack_is_never_cached_and_answers_as_stacker_does() {
        let answered = std::thread::spawn(|| (has_room(), FLOOR.get().addr()))
            .join()
            .map_err(|_| "the answering thread completed");
        assert_eq!(answered, Ok((true, NO_SPAN.addr())));
    }

    #[test]
    fn a_rayon_workers_first_check_on_its_own_stack_is_answered_from_the_span() {
        let seen = on_a_rayon_worker(4 * CALL_STACK_GROWTH, || {
            let before = FLOOR.get().addr();
            let inside = on_this_threads_stack(|| {
                let seeded = FLOOR.get() != NO_SPAN && absent_or_above_two_red_zones();
                (in_the_span(a_local_address()), seeded, has_room())
            });
            (before, inside, FLOOR.get().addr())
        });
        assert_eq!(seen, (NO_SPAN.addr(), (true, true, true), NO_SPAN.addr()));
    }

    #[test]
    fn a_thread_stack_too_small_for_a_span_seeds_nothing_below_two_red_zones() {
        let seen = on_a_rayon_worker(SPAN_BYTES, || {
            on_this_threads_stack(|| (absent_or_above_two_red_zones(), has_room()))
        });
        assert_eq!(seen, (true, true));
    }

    #[test]
    fn a_foreign_switch_inside_the_seeded_stack_answers_as_stacker_does() {
        let seen = on_a_rayon_worker(4 * CALL_STACK_GROWTH, || {
            on_this_threads_stack(|| {
                let seeded = FLOOR.get();
                let foreign = stacker::grow(CALL_STACK_GROWTH, || {
                    (in_the_span(a_local_address()), FLOOR.get() == seeded)
                });
                (foreign, in_the_span(a_local_address()))
            })
        });
        assert_eq!(seen, ((false, true), true));
    }

    #[test]
    fn a_segment_grown_inside_the_seeded_stack_hands_the_seeded_floor_back() {
        let seen = on_a_rayon_worker(4 * CALL_STACK_GROWTH, || {
            on_this_threads_stack(|| {
                let seeded = FLOOR.get();
                let inner = grow(CALL_STACK_GROWTH, || FLOOR.get());
                (seeded != NO_SPAN, inner != seeded, FLOOR.get() == seeded)
            })
        });
        assert_eq!(seen, (true, true, true));
    }

    #[test]
    fn a_panic_out_of_the_seeded_stack_restores_the_callers_floor() {
        let seen = on_a_rayon_worker(4 * CALL_STACK_GROWTH, || {
            FLOOR.set(a_foreign_floor());
            let caught = catch_unwind(AssertUnwindSafe(|| {
                on_this_threads_stack(|| std::panic::panic_any(FLOOR.get().addr()))
            }));
            let restored = FLOOR.get();
            FLOOR.set(NO_SPAN);
            let inside = caught
                .err()
                .and_then(|payload| payload.downcast::<usize>().ok())
                .map(|floor| *floor);
            (
                inside.is_some_and(|floor| floor != NO_SPAN.addr()),
                restored.addr(),
            )
        });
        assert_eq!(seen, (true, a_foreign_floor().addr()));
    }
}
