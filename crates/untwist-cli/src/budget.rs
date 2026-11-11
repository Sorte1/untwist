use std::alloc::System;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use untwist::{BudgetHit, CountingAlloc};

pub const DEFAULT_MAX_MEMORY: &str = "4GiB";
pub const EXIT_OVER_MEMORY: u8 = 120;
pub const EXIT_OVER_TIME: u8 = 121;
pub const TIME_LEFT: &str = "UNTWIST_INTERNAL_TIME_LEFT_MS";
const SOFT_SHARE_NUMERATOR: u64 = 3;
const SOFT_SHARE_DENOMINATOR: u64 = 4;
const GRACE_FLOOR: Duration = Duration::from_secs(2);
const GRACE_DIVISOR: u32 = 4;

static PIPELINE_DONE: AtomicBool = AtomicBool::new(false);

pub type Counting = CountingAlloc<System>;

pub const fn counting() -> Counting {
    CountingAlloc(System)
}

fn over_memory() {
    die(EXIT_OVER_MEMORY);
}

#[cfg(unix)]
pub fn die(code: u8) -> ! {
    unsafe { libc::_exit(i32::from(code)) }
}

#[cfg(not(unix))]
pub fn die(code: u8) -> ! {
    std::process::exit(i32::from(code))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Limits {
    pub max_memory: Option<u64>,
    #[serde(serialize_with = "seconds")]
    pub timeout: Option<Duration>,
}

fn seconds<S: serde::Serializer>(timeout: &Option<Duration>, out: S) -> Result<S::Ok, S::Error> {
    match timeout {
        Some(timeout) => out.serialize_some(&timeout.as_secs_f64()),
        None => out.serialize_none(),
    }
}

impl Limits {
    pub fn soft_memory(&self) -> Option<u64> {
        self.max_memory
            .map(|bytes| bytes / SOFT_SHARE_DENOMINATOR * SOFT_SHARE_NUMERATOR)
    }
}

pub fn time_left_from(value: Option<std::ffi::OsString>) -> Option<Duration> {
    value
        .and_then(|value| value.into_string().ok())
        .and_then(|ms| ms.parse::<u64>().ok())
        .map(Duration::from_millis)
}

pub fn arm_hard_memory(limits: &Limits) {
    if let Some(bytes) = limits.max_memory {
        untwist::set_hard_memory_limit(bytes, over_memory);
    }
}

pub fn hard_deadline(timeout: Duration) -> Duration {
    timeout + (timeout / GRACE_DIVISOR).max(GRACE_FLOOR)
}

pub fn arm_watchdog(timeout: Option<Duration>) {
    let Some(timeout) = timeout else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("untwist-watchdog".to_string())
        .stack_size(64 * 1024)
        .spawn(move || {
            std::thread::sleep(hard_deadline(timeout));
            if !PIPELINE_DONE.load(Ordering::SeqCst) {
                die(EXIT_OVER_TIME);
            }
        });
}

pub fn disarm_watchdog() {
    PIPELINE_DONE.store(true, Ordering::SeqCst);
}

pub fn hit_of_exit(code: u8) -> Option<BudgetHit> {
    match code {
        EXIT_OVER_MEMORY => Some(BudgetHit::Memory),
        EXIT_OVER_TIME => Some(BudgetHit::Time),
        _ => None,
    }
}

pub fn bytes_text(bytes: u64) -> String {
    const GIB: f64 = (1u64 << 30) as f64;
    const MIB: f64 = (1u64 << 20) as f64;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else {
        format!("{:.1} MiB", bytes / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::Limits;

    #[test]
    fn the_soft_memory_budget_is_three_quarters_of_the_cap() {
        let limits = Limits {
            max_memory: Some(4 << 30),
            timeout: None,
        };
        assert_eq!(limits.soft_memory(), Some(3 << 30));
        assert_eq!(Limits::default().soft_memory(), None);
    }
}
