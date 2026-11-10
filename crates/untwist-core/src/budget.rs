use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use crate::clock::StageInstant;

pub const MEMORY_REFUSAL: &str = "budget-memory";
pub const TIME_REFUSAL: &str = "budget-time";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hit {
    Memory,
    Time,
}

impl Hit {
    pub fn as_str(self) -> &'static str {
        match self {
            Hit::Memory => MEMORY_REFUSAL,
            Hit::Time => TIME_REFUSAL,
        }
    }

    fn code(self) -> u8 {
        match self {
            Hit::Memory => 1,
            Hit::Time => 2,
        }
    }

    fn from_code(code: u8) -> Option<Hit> {
        match code {
            1 => Some(Hit::Memory),
            2 => Some(Hit::Time),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    start: StageInstant,
    length: Duration,
}

impl Deadline {
    pub fn after(length: Duration) -> Self {
        Self {
            start: StageInstant::now(),
            length,
        }
    }

    pub fn passed(&self) -> bool {
        self.start.elapsed() >= self.length
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Limits {
    pub memory: Option<u64>,
    pub deadline: Option<Deadline>,
}

const DRIFT: i64 = 256 << 10;

struct Meter {
    used: Cell<i64>,
    high: Cell<i64>,
    low: Cell<i64>,
    flushed: Cell<i64>,
    peak: Cell<i64>,
    limit: Cell<i64>,
    hit: Cell<u8>,
    deadline: Cell<Option<Deadline>>,
}

thread_local! {
    static METER: Meter = const {
        Meter {
            used: Cell::new(0),
            high: Cell::new(DRIFT),
            low: Cell::new(-DRIFT),
            flushed: Cell::new(0),
            peak: Cell::new(0),
            limit: Cell::new(i64::MAX),
            hit: Cell::new(0),
            deadline: Cell::new(None),
        }
    };
}

static TOTAL: AtomicI64 = AtomicI64::new(0);
static HARD: AtomicI64 = AtomicI64::new(i64::MAX);
static HARD_ACTION: OnceLock<fn()> = OnceLock::new();

pub fn set_hard_limit(bytes: u64, action: fn()) {
    let _ = HARD_ACTION.set(action);
    HARD.store(signed_u64(bytes), Ordering::Relaxed);
}

pub fn process_total() -> u64 {
    u64::try_from(TOTAL.load(Ordering::Relaxed)).unwrap_or(0)
}

fn signed(bytes: usize) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}

impl Meter {
    #[inline]
    fn grew(&self, bytes: usize) {
        let used = self.used.get().wrapping_add(signed(bytes));
        self.used.set(used);
        if used > self.high.get() {
            self.settle(used);
        }
    }

    #[inline]
    fn shrank(&self, bytes: usize) {
        let used = self.used.get().wrapping_sub(signed(bytes));
        self.used.set(used);
        if used < self.low.get() {
            self.settle(used);
        }
    }

    #[cold]
    fn settle(&self, used: i64) {
        self.high.set(next_high(used, self.limit.get()));
        self.low.set(used.saturating_sub(DRIFT));
        let delta = used.wrapping_sub(self.flushed.replace(used));
        let total = TOTAL
            .fetch_add(delta, Ordering::Relaxed)
            .wrapping_add(delta);
        if total > HARD.load(Ordering::Relaxed) {
            if let Some(action) = HARD_ACTION.get() {
                action();
            }
        }
        if used > self.peak.get() {
            self.peak.set(used);
        }
        if used > self.limit.get() && self.hit.get() == 0 {
            self.hit.set(Hit::Memory.code());
        }
    }
}

fn next_high(used: i64, limit: i64) -> i64 {
    let next = used.saturating_add(DRIFT);
    if used < limit {
        next.min(limit)
    } else {
        next
    }
}

#[inline]
fn grew(bytes: usize) {
    METER.with(|meter| meter.grew(bytes));
}

#[inline]
fn shrank(bytes: usize) {
    METER.with(|meter| meter.shrank(bytes));
}

pub struct CountingAlloc<A>(pub A);

unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAlloc<A> {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let block = unsafe { self.0.alloc(layout) };
        if !block.is_null() {
            grew(layout.size());
        }
        block
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let block = unsafe { self.0.alloc_zeroed(layout) };
        if !block.is_null() {
            grew(layout.size());
        }
        block
    }

    #[inline]
    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(block, layout) };
        shrank(layout.size());
    }

    #[inline]
    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { self.0.realloc(block, layout, new_size) };
        if !moved.is_null() {
            match new_size.checked_sub(layout.size()) {
                Some(more) => grew(more),
                None => shrank(layout.size() - new_size),
            }
        }
        moved
    }
}

pub struct Scope {
    base: i64,
    peak: i64,
    limit: i64,
    hit: u8,
    deadline: Option<Deadline>,
}

pub fn enter(limits: Limits) -> Scope {
    METER.with(|meter| {
        let used = meter.used.get();
        let limit = meter.limit.get();
        let own = limits
            .memory
            .map_or(i64::MAX, |bytes| used.saturating_add(signed_u64(bytes)));
        meter.limit.set(own.min(limit));
        meter.high.set(next_high(used, meter.limit.get()));
        Scope {
            base: used,
            peak: meter.peak.replace(used),
            limit,
            hit: meter.hit.replace(0),
            deadline: meter.deadline.replace(limits.deadline),
        }
    })
}

fn signed_u64(bytes: u64) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}

impl Scope {
    pub fn peak(&self) -> u64 {
        METER.with(|meter| {
            let peak = meter.peak.get().max(meter.used.get());
            u64::try_from(peak.saturating_sub(self.base)).unwrap_or(0)
        })
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        METER.with(|meter| {
            meter.peak.set(meter.peak.get().max(self.peak));
            meter.limit.set(self.limit);
            meter.hit.set(self.hit);
            meter.deadline.set(self.deadline);
        });
    }
}

pub fn room() -> Option<u64> {
    METER.with(|meter| {
        let limit = meter.limit.get();
        (limit != i64::MAX)
            .then(|| u64::try_from(limit.saturating_sub(meter.used.get())).unwrap_or(0))
    })
}

pub fn hit() -> Option<Hit> {
    METER.with(|meter| Hit::from_code(meter.hit.get()))
}

#[inline]
pub fn tripped() -> bool {
    METER.with(|meter| meter.hit.get() != 0)
}

pub fn check() -> Result<(), Hit> {
    METER.with(|meter| {
        if let Some(hit) = Hit::from_code(meter.hit.get()) {
            return Err(hit);
        }
        match meter.deadline.get() {
            Some(deadline) if deadline.passed() => {
                meter.hit.set(Hit::Time.code());
                Err(Hit::Time)
            }
            _ => Ok(()),
        }
    })
}

pub fn trip(hit: Hit) {
    METER.with(|meter| {
        if meter.hit.get() == 0 {
            meter.hit.set(hit.code());
        }
    });
}

pub fn memory_size(word: &str) -> Result<u64, String> {
    let word = word.trim();
    if word == "unlimited" || word == "none" {
        return Ok(u64::MAX);
    }
    let split = word
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(word.len());
    let (number, unit) = word.split_at(split);
    let number: f64 = number
        .parse()
        .map_err(|_| format!("takes a size such as 512MiB, 4G or unlimited, not {word}"))?;
    let scale = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1u64,
        "k" | "kb" | "kib" => 1 << 10,
        "m" | "mb" | "mib" => 1 << 20,
        "g" | "gb" | "gib" => 1 << 30,
        "t" | "tb" | "tib" => 1 << 40,
        _ => return Err(format!("unknown size unit in {word}")),
    };
    let bytes = number * scale as f64;
    if !bytes.is_finite() || bytes < 1.0 {
        return Err("must be at least 1 byte".to_string());
    }
    Ok(if bytes >= u64::MAX as f64 {
        u64::MAX
    } else {
        bytes as u64
    })
}

pub fn timeout_secs(word: &str) -> Result<Duration, String> {
    let secs: f64 = word
        .trim()
        .parse()
        .map_err(|_| format!("takes a number of seconds, not {word}"))?;
    if !secs.is_finite() || secs <= 0.0 {
        return Err("must be a positive number of seconds".to_string());
    }
    Duration::try_from_secs_f64(secs).map_err(|_| format!("{word} seconds is too long"))
}

#[cfg(test)]
mod tests {
    use super::{memory_size, timeout_secs};
    use std::time::Duration;

    #[test]
    fn sizes_read_binary_units_and_unlimited() {
        assert_eq!(memory_size("4GiB"), Ok(4 << 30));
        assert_eq!(memory_size("4G"), Ok(4 << 30));
        assert_eq!(memory_size("512m"), Ok(512 << 20));
        assert_eq!(memory_size("1.5GiB"), Ok(3 << 29));
        assert_eq!(memory_size("1000"), Ok(1000));
        assert_eq!(memory_size("unlimited"), Ok(u64::MAX));
        assert!(memory_size("4X").is_err());
        assert!(memory_size("0").is_err());
        assert!(memory_size("lots").is_err());
    }

    #[test]
    fn timeouts_read_positive_seconds() {
        assert_eq!(timeout_secs("2.5"), Ok(Duration::from_millis(2500)));
        assert!(timeout_secs("0").is_err());
        assert!(timeout_secs("-1").is_err());
        assert!(timeout_secs("soon").is_err());
    }
}
