use std::num::NonZeroUsize;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rayon::{ThreadPool, ThreadPoolBuilder};
use untwist_eval::stack;

use super::boundary::Boundary;
use super::input::InputResult;
use super::ladder::{on_a_worker_stack, Ladder};
use super::{Batch, Pipeline};

pub(super) fn width(requested: Option<NonZeroUsize>, inputs: usize) -> usize {
    let wanted = requested
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, NonZeroUsize::get);
    wanted.min(inputs).max(1)
}

pub(super) struct WorkerPool {
    pool: ThreadPool,
    stack_bytes: usize,
}

fn segments_fit(stack_bytes: usize, count: usize) -> bool {
    if count == 0 {
        return true;
    }
    let mut fit = false;
    let mapped = catch_unwind(AssertUnwindSafe(|| {
        stack::grow(stack_bytes, || fit = segments_fit(stack_bytes, count - 1));
    }));
    mapped.is_ok() && fit
}

pub(super) fn worker_pool(width: usize, ladder: &Ladder) -> Option<WorkerPool> {
    let stack_bytes = on_a_worker_stack(ladder, |obtained| obtained);
    if stack_bytes == 0 {
        return None;
    }
    let _silenced = Boundary::enter();
    if !segments_fit(stack_bytes, width) {
        return None;
    }
    ThreadPoolBuilder::new()
        .num_threads(width)
        .stack_size(stack_bytes)
        .build()
        .ok()
        .map(|pool| WorkerPool { pool, stack_bytes })
}

fn on_each_worker<R: Send>(pool: &WorkerPool, body: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let stack_bytes = pool.stack_bytes;
    pool.pool
        .broadcast(|_| stack::on_this_threads_stack(|| body(stack_bytes)))
}

type Slot = Mutex<Option<InputResult>>;

fn store(slot: &Slot, result: InputResult) {
    let mut guard = match slot.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(result);
}

fn take(slot: Slot) -> Option<InputResult> {
    match slot.into_inner() {
        Ok(inner) => inner,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl Pipeline {
    pub(super) fn run_pooled(
        &self,
        batch: &Batch<'_>,
        ladder: &Ladder,
    ) -> Option<Vec<InputResult>> {
        let width = width(self.options.jobs, batch.sources.len());
        (width > 1)
            .then(|| self.run_in_parallel(width, batch, ladder))
            .flatten()
    }

    fn run_in_parallel(
        &self,
        width: usize,
        batch: &Batch<'_>,
        ladder: &Ladder,
    ) -> Option<Vec<InputResult>> {
        let pool = worker_pool(width, ladder)?;
        let sources = &batch.sources;
        let next = AtomicUsize::new(0);
        let results: Vec<Slot> = sources.iter().map(|_| Mutex::new(None)).collect();
        on_each_worker(&pool, |_| loop {
            let index = next.fetch_add(1, Ordering::SeqCst);
            let Some(&source) = sources.get(index) else {
                break;
            };
            let result = self.run_input(index, source, batch.reservation(index));
            if let Some(slot) = results.get(index) {
                store(slot, result);
            }
        });
        drop(pool);
        self.inputs_finished(sources.len());
        Some(
            results
                .into_iter()
                .enumerate()
                .map(|(index, slot)| {
                    take(slot).unwrap_or_else(|| {
                        InputResult::source_only(sources.get(index).copied().unwrap_or(""))
                    })
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::ladder::{Ladder, LADDER};
    use super::{on_each_worker, width, worker_pool};
    use crate::options::TestHooks;
    use std::num::NonZeroUsize;
    use untwist_eval::stack::room;

    const MIB: usize = 1 << 20;
    const SIXTY_FOUR_MIB: usize = 64 << 20;

    #[test]
    fn a_pool_workers_body_receives_the_pools_stack_size_and_room_within_two_mib() {
        let pool = worker_pool(2, &Ladder::new(TestHooks::default()))
            .expect("a two-thread pool builds on the ladder");
        let seen = on_each_worker(&pool, |obtained| (obtained, room()));
        assert!(LADDER.contains(&pool.stack_bytes), "{}", pool.stack_bytes);
        assert_eq!(seen.len(), 2);
        for (obtained, left) in seen {
            assert_eq!(obtained, pool.stack_bytes);
            assert!(left <= obtained && left + 2 * MIB >= obtained, "{left}");
        }
    }

    #[test]
    fn segments_that_cannot_all_be_mapped_at_once_do_not_fit() {
        assert!(super::segments_fit(SIXTY_FOUR_MIB, 3));
        assert!(!super::segments_fit(usize::MAX / 2, 1));
    }

    #[test]
    fn a_pool_takes_the_rung_the_sequential_path_would_take() {
        let refusing = Ladder::new(TestHooks {
            ladder_refuse: 1,
            ..TestHooks::default()
        });
        let pool = worker_pool(2, &refusing).expect("a pool on the second rung");
        assert_eq!(pool.stack_bytes, 256 << 20);
    }

    #[test]
    fn one_input_never_widens() {
        assert_eq!(width(NonZeroUsize::new(8), 1), 1);
        assert_eq!(width(None, 1), 1);
    }

    #[test]
    fn the_width_is_capped_at_the_input_count_and_floored_at_one() {
        assert_eq!(width(NonZeroUsize::new(4), 2), 2);
        assert_eq!(width(NonZeroUsize::new(1), 8), 1);
        assert!(width(None, 8) >= 1);
    }

    #[test]
    fn no_pool_is_built_when_the_ladder_refuses_every_rung() {
        let every = Ladder::new(TestHooks {
            ladder_refuse: 4,
            ..TestHooks::default()
        });
        assert!(worker_pool(2, &every).is_none());
    }
}
