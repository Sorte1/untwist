#[allow(dead_code, unused_imports)]
#[path = "../src/main.rs"]
mod cli;

use cli::{
    args, base_options, channel, finish, hooks, in_process, preflight_warning, reruns, worker,
    Session,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::os::unix::fs::FileExt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

const LARGE: usize = 64 << 10;
const EVERY: usize = 1024;
const PAGE: u64 = 4096;
const STATUS_KEYS: [&str; 4] = ["VmRSS", "RssAnon", "RssFile", "VmHWM"];

static STATM: OnceLock<std::fs::File> = OnceLock::new();
static ARMED: AtomicBool = AtomicBool::new(false);
static OPS: AtomicUsize = AtomicUsize::new(0);
static PEAK_PAGES: AtomicUsize = AtomicUsize::new(0);
static SAMPLES: AtomicUsize = AtomicUsize::new(0);
const TRACE: usize = 8192;
static TRACED: [AtomicUsize; TRACE] = [const { AtomicUsize::new(0) }; TRACE];
static TRACING: AtomicBool = AtomicBool::new(false);

fn resident_pages(text: &[u8]) -> Option<usize> {
    let resident = text.split(|byte| *byte == b' ').nth(1)?;
    resident
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .try_fold(0usize, |total, byte| {
            total.checked_mul(10)?.checked_add(usize::from(byte - b'0'))
        })
}

fn sample() {
    let Some(file) = STATM.get() else {
        return;
    };
    let mut buffer = [0u8; 128];
    let Ok(read) = file.read_at(&mut buffer, 0) else {
        return;
    };
    let Some(pages) = buffer.get(..read).and_then(resident_pages) else {
        return;
    };
    let index = SAMPLES.fetch_add(1, Ordering::Relaxed);
    if TRACING.load(Ordering::Relaxed) {
        if let Some(slot) = TRACED.get(index) {
            slot.store(pages, Ordering::Relaxed);
        }
    }
    PEAK_PAGES.fetch_max(pages, Ordering::Relaxed);
}

fn tick(size: usize, releases: bool) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    let ops = OPS.fetch_add(1, Ordering::Relaxed);
    if (releases && size >= LARGE) || ops.is_multiple_of(EVERY) {
        sample();
    }
}

struct PeakProbe;

unsafe impl GlobalAlloc for PeakProbe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        tick(layout.size(), false);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        tick(layout.size(), true);
        System.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        tick(layout.size(), false);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        tick(layout.size(), true);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static PROBE: PeakProbe = PeakProbe;

fn file_mapping(line: &str) -> Option<(u64, u64)> {
    let mut fields = line.split_whitespace();
    let range = fields.next()?;
    let permissions = fields.next()?;
    let path = fields.nth(3)?;
    if !permissions.starts_with('r') || !path.starts_with('/') {
        return None;
    }
    let (low, high) = range.split_once('-')?;
    let low = u64::from_str_radix(low, 16).ok()?;
    let high = u64::from_str_radix(high, 16).ok()?;
    Some((low, high))
}

fn prefault_file_pages() {
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
    let Ok(memory) = std::fs::File::open("/proc/self/mem") else {
        return;
    };
    let mut byte = [0u8; 1];
    for (low, high) in maps.lines().filter_map(file_mapping) {
        let mut at = low;
        while at < high {
            let _ = memory.read_at(&mut byte, at);
            at += PAGE;
        }
    }
}

static PAD: OnceLock<Vec<u8>> = OnceLock::new();

fn pad_heap() {
    let bytes = std::env::var("UNTWIST_PEAK_PAD")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if bytes > 0 {
        let _ = PAD.set(vec![1u8; bytes]);
    }
}

fn prefault() {
    pad_heap();
    prefault_file_pages();
    let _ = writeln!(std::io::stderr(), "peak-start {}", status_fields());
    TRACING.store(
        std::env::var_os("UNTWIST_PEAK_TRACE").is_some(),
        Ordering::Relaxed,
    );
    if let Ok(file) = std::fs::File::open("/proc/self/statm") {
        let _ = STATM.set(file);
    }
    ARMED.store(true, Ordering::Relaxed);
}

fn status_fields() -> String {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let fields: Vec<String> = text
        .lines()
        .filter(|line| STATUS_KEYS.iter().any(|key| line.starts_with(key)))
        .map(|line| {
            line.split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    fields.join(" ")
}

fn heap_kb() -> String {
    let text = std::fs::read_to_string("/proc/self/smaps").unwrap_or_default();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with(|c: char| c.is_ascii_hexdigit()) && line.contains('-') {
            inside = line.ends_with("[heap]");
        } else if inside && line.starts_with("Rss:") {
            return line.split_whitespace().nth(1).unwrap_or("?").to_string();
        }
    }
    "none".to_string()
}

fn write_trace() {
    let Ok(path) = std::env::var("UNTWIST_PEAK_TRACE") else {
        return;
    };
    let count = SAMPLES.load(Ordering::Relaxed).min(TRACE);
    let lines: Vec<String> = TRACED[..count]
        .iter()
        .map(|slot| (slot.load(Ordering::Relaxed) * 4).to_string())
        .collect();
    let _ = std::fs::write(path, lines.join("\n"));
}

fn report() {
    sample();
    write_trace();
    let peak = PEAK_PAGES.load(Ordering::Relaxed) * 4;
    let samples = SAMPLES.load(Ordering::Relaxed);
    let _ = writeln!(
        std::io::stderr(),
        "peak peak_kb {peak} samples {samples} heap_kb {} {}",
        heap_kb(),
        status_fields()
    );
}

fn main() -> std::process::ExitCode {
    cli::main_with(&prefault, &report)
}
