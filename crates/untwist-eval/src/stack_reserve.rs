use std::collections::BTreeSet;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

pub const STACK_RESERVE_BYTES: u64 = 1 << 30;

pub const DECODED_TEXT_STACK_BYTES: u64 = 8 << 20;

pub struct BytePool {
    budget: u64,
    used: Mutex<u64>,
    freed: Condvar,
}

static PROCESS_POOL: BytePool = BytePool::new(STACK_RESERVE_BYTES);

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl BytePool {
    pub const fn new(budget: u64) -> Self {
        Self {
            budget,
            used: Mutex::new(0),
            freed: Condvar::new(),
        }
    }

    pub fn process() -> &'static BytePool {
        &PROCESS_POOL
    }

    fn clamped(&self, bytes: u64) -> u64 {
        bytes.min(self.budget)
    }

    fn take(&self, bytes: u64) -> StackReservation<'_> {
        let bytes = self.clamped(bytes);
        let mut used = locked(&self.used);
        while self.budget - *used < bytes {
            used = self
                .freed
                .wait(used)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *used += bytes;
        StackReservation { pool: self, bytes }
    }

    pub fn try_reserve(&self, bytes: u64) -> Option<StackReservation<'_>> {
        let bytes = self.clamped(bytes);
        let mut used = locked(&self.used);
        if self.budget - *used < bytes {
            return None;
        }
        *used += bytes;
        Some(StackReservation { pool: self, bytes })
    }
}

#[must_use]
pub struct StackReservation<'p> {
    pool: &'p BytePool,
    bytes: u64,
}

impl StackReservation<'_> {
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Drop for StackReservation<'_> {
    fn drop(&mut self) {
        let mut used = locked(&self.pool.used);
        *used = used.saturating_sub(self.bytes);
        drop(used);
        self.pool.freed.notify_all();
    }
}

#[derive(Default)]
struct Turns {
    next: usize,
    passed: BTreeSet<usize>,
}

impl Turns {
    fn pass(&mut self, index: usize) {
        if index != self.next {
            if index > self.next {
                self.passed.insert(index);
            }
            return;
        }
        self.next += 1;
        while self.passed.remove(&self.next) {
            self.next += 1;
        }
    }
}

pub struct StackReserve<'p> {
    pool: &'p BytePool,
    turns: Mutex<Turns>,
    turn_passed: Condvar,
}

impl Default for StackReserve<'static> {
    fn default() -> Self {
        Self::on(BytePool::process())
    }
}

impl<'p> StackReserve<'p> {
    pub fn on(pool: &'p BytePool) -> Self {
        Self {
            pool,
            turns: Mutex::new(Turns::default()),
            turn_passed: Condvar::new(),
        }
    }

    pub fn reserve(&self, index: usize, bytes: u64) -> StackReservation<'p> {
        self.wait_for_turn(index);
        let reservation = self.pool.take(bytes);
        self.give_up(index);
        reservation
    }

    pub fn give_up(&self, index: usize) {
        locked(&self.turns).pass(index);
        self.turn_passed.notify_all();
    }

    pub fn try_reserve(&self, bytes: u64) -> Option<StackReservation<'p>> {
        self.pool.try_reserve(bytes)
    }

    fn wait_for_turn(&self, index: usize) {
        let mut turns = locked(&self.turns);
        while turns.next < index {
            turns = self
                .turn_passed
                .wait(turns)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BytePool, StackReserve, STACK_RESERVE_BYTES};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Barrier;
    use std::time::Duration;

    const MIB: u64 = 1 << 20;
    const SETTLE: Duration = Duration::from_millis(100);

    #[test]
    fn a_second_six_hundred_mib_request_waits_until_the_first_drops() {
        let pool = BytePool::new(STACK_RESERVE_BYTES);
        let reserve = StackReserve::on(&pool);
        let first_held = Barrier::new(2);
        let second_granted = AtomicBool::new(false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let held = reserve.reserve(0, 600 * MIB);
                first_held.wait();
                std::thread::sleep(SETTLE);
                assert!(!second_granted.load(Ordering::SeqCst));
                drop(held);
            });
            scope.spawn(|| {
                first_held.wait();
                let _held = reserve.reserve(1, 600 * MIB);
                second_granted.store(true, Ordering::SeqCst);
            });
        });
        assert!(second_granted.load(Ordering::SeqCst));
    }

    fn a_later_index_waits_for_the_earlier_turn(earlier_obtains: bool) {
        let pool = BytePool::new(STACK_RESERVE_BYTES);
        let reserve = StackReserve::on(&pool);
        let later_asking = Barrier::new(2);
        let later_granted = AtomicBool::new(false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                later_asking.wait();
                std::thread::sleep(SETTLE);
                assert!(!later_granted.load(Ordering::SeqCst));
                if earlier_obtains {
                    drop(reserve.reserve(0, MIB));
                } else {
                    reserve.give_up(0);
                }
            });
            scope.spawn(|| {
                later_asking.wait();
                let _held = reserve.reserve(1, MIB);
                later_granted.store(true, Ordering::SeqCst);
            });
        });
        assert!(later_granted.load(Ordering::SeqCst));
    }

    #[test]
    fn index_one_is_not_granted_before_index_zero_obtains_even_when_its_bytes_fit() {
        a_later_index_waits_for_the_earlier_turn(true);
    }

    #[test]
    fn index_one_is_not_granted_before_index_zero_gives_up_its_turn() {
        a_later_index_waits_for_the_earlier_turn(false);
    }

    #[test]
    fn a_two_gib_request_is_clamped_to_the_reserve_and_granted_alone() {
        let pool = BytePool::new(STACK_RESERVE_BYTES);
        let reserve = StackReserve::on(&pool);
        let held = reserve.reserve(0, 2 << 30);
        assert_eq!(held.bytes(), STACK_RESERVE_BYTES);
        assert!(reserve.try_reserve(1).is_none());
        drop(held);
        assert!(reserve.try_reserve(1).is_some());
    }

    #[test]
    fn try_reserve_on_a_full_reserve_returns_none_at_once() {
        let pool = BytePool::new(STACK_RESERVE_BYTES);
        let reserve = StackReserve::on(&pool);
        let _full = reserve.reserve(0, STACK_RESERVE_BYTES);
        assert!(reserve.try_reserve(MIB).is_none());
        assert!(pool.try_reserve(1).is_none());
    }

    #[test]
    fn turns_given_up_out_of_order_are_skipped_when_the_turn_reaches_them() {
        let pool = BytePool::new(STACK_RESERVE_BYTES);
        let reserve = StackReserve::on(&pool);
        reserve.give_up(2);
        reserve.give_up(1);
        drop(reserve.reserve(0, MIB));
        let _fourth = reserve.reserve(3, MIB);
    }
}
