use std::cell::Cell;
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    static EDITS: Cell<u64> = const { Cell::new(0) };
}

#[cfg(feature = "test-hooks")]
static FULL_ROUNDS: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
static STALE: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
static EVERY_LIST: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn walk_every_list_for_tests(on: bool) {
    EVERY_LIST.store(on, Ordering::SeqCst);
}

#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn full_rounds_for_tests(on: bool) {
    FULL_ROUNDS.store(on, Ordering::SeqCst);
}

#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn stale_settled_for_tests(on: bool) {
    STALE.store(on, Ordering::SeqCst);
}

pub fn edited() {
    #[cfg(feature = "test-hooks")]
    if STALE.load(Ordering::SeqCst) {
        return;
    }
    EDITS.with(|edits| edits.set(edits.get().wrapping_add(1)));
}

pub fn edits() -> u64 {
    EDITS.with(Cell::get)
}

pub fn replay_allowed() -> bool {
    #[cfg(feature = "test-hooks")]
    return !FULL_ROUNDS.load(Ordering::SeqCst);
    #[cfg(not(feature = "test-hooks"))]
    return true;
}

pub(crate) fn parking_allowed() -> bool {
    #[cfg(feature = "test-hooks")]
    return replay_allowed() && !EVERY_LIST.load(Ordering::SeqCst);
    #[cfg(not(feature = "test-hooks"))]
    return true;
}

#[cfg(test)]
mod tests {
    use super::{edited, edits};

    #[test]
    fn every_counted_edit_moves_the_count() {
        let before = edits();
        edited();
        assert_ne!(edits(), before);
    }
}
