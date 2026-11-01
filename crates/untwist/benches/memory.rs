use std::alloc::{GlobalAlloc, Layout, System};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::print::pretty_codegen;
use untwist_core::reflow::reflow;
use untwist_core::semantic::build_scoping;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static COMPILED_BLOCKS: AtomicUsize = AtomicUsize::new(0);
static COMPILED_BYTES: AtomicUsize = AtomicUsize::new(0);
static COMPILED_CHUNKS: AtomicUsize = AtomicUsize::new(0);

const GLIBC_MIN_CHUNK: usize = 32;
const GLIBC_HEADER: usize = 8;
const GLIBC_ALIGN: usize = 16;

fn glibc_chunk(size: usize) -> usize {
    ((size + GLIBC_HEADER + GLIBC_ALIGN - 1) & !(GLIBC_ALIGN - 1)).max(GLIBC_MIN_CHUNK)
}

fn note_compiled(size: usize, grow: bool) {
    if !untwist_eval::test_hooks::compiling() {
        return;
    }
    let (op, chunk) = (if grow { 1 } else { usize::MAX }, glibc_chunk(size));
    COMPILED_BLOCKS.fetch_add(op, Ordering::Relaxed);
    COMPILED_BYTES.fetch_add(size.wrapping_mul(op), Ordering::Relaxed);
    COMPILED_CHUNKS.fetch_add(chunk.wrapping_mul(op), Ordering::Relaxed);
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(live, Ordering::Relaxed);
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        note_compiled(layout.size(), true);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        note_compiled(layout.size(), false);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn resolve(path: &str) -> PathBuf {
    let given = Path::new(path);
    if given.is_absolute() {
        given.to_path_buf()
    } else {
        root().join(given)
    }
}

fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
}

fn report_point(path: &str, label: &str, input: usize, bytes: usize) {
    println!(
        "{path}: {label}: {bytes} B, {:.2}x",
        bytes as f64 / input as f64
    );
}

fn floor(path: &str, source: &str) {
    let input = source.len();
    let baseline = live();
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    report_point(path, "after_parse", input, live().saturating_sub(baseline));
    PEAK.store(live(), Ordering::Relaxed);
    let sem = build_scoping(&parsed.program);
    let ctors = untwist_unminify::prepare_ctors();
    let mut group = PassGroup::from_ctors(&ctors, PassOptions::default());
    let (_sem, _report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    report_point(
        path,
        "prepare_peak",
        input,
        PEAK.load(Ordering::Relaxed).saturating_sub(baseline),
    );
    report_point(
        path,
        "after_prepare",
        input,
        live().saturating_sub(baseline),
    );
    let code = pretty_codegen(&parsed.program);
    report_point(
        path,
        "after_codegen",
        input,
        live().saturating_sub(baseline),
    );
    PEAK.store(live(), Ordering::Relaxed);
    let code = reflow(code);
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(baseline);
    report_point(path, "reflow_peak", input, peak);
    report_point(path, "after_reflow", input, live().saturating_sub(baseline));
    drop(code);
}

fn pipeline(path: &str, source: &str, deobfuscate: bool) {
    let input = source.len();
    PEAK.store(live(), Ordering::Relaxed);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let out = untwist::Pipeline::new(untwist::Options {
        deobfuscate,
        unminify: deobfuscate,
        jsx: false,
        ..untwist::Options::default()
    })
    .run(source);
    let peak = PEAK.load(Ordering::Relaxed);
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;
    println!(
        "{path}: input {input} B, peak {peak} B, ratio {:.2}x, modules {}, allocations {allocations}",
        peak as f64 / input as f64,
        out.bundle.map_or(0, |bundle| bundle.modules.len())
    );
}

fn nodes(path: &str, source: &str) {
    let census = untwist_eval::test_hooks::compile_census(|| {
        pipeline(path, source, true);
    });
    let nodes = (census.nodes + census.statements).max(1) as usize;
    let blocks = COMPILED_BLOCKS.load(Ordering::Relaxed);
    let bytes = COMPILED_BYTES.load(Ordering::Relaxed);
    let chunks = COMPILED_CHUNKS.load(Ordering::Relaxed);
    println!(
        "{path}: compiled nodes {nodes}, live compiled blocks {blocks}, bytes {bytes}, glibc chunk bytes {chunks}, NODE_BYTES {:.1} requested / {:.1} with the chunk",
        bytes as f64 / nodes as f64,
        chunks as f64 / nodes as f64
    );
}

fn usage() -> ! {
    eprintln!("usage: memory <file.js> [--deobfuscate | --floor | --nodes] | memory --frames");
    std::process::exit(2);
}

fn allocations_in(body: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    body();
    ALLOCATIONS.load(Ordering::Relaxed) - before
}

fn frames() {
    use untwist_eval::frame::{Frame, FrameLayout, SlotKind};
    let text: String = (0..130).map(|n| format!("v{n} ")).collect();
    let mut names = untwist_eval::names::Names::for_source(&text);
    let all: Vec<_> = text
        .split_whitespace()
        .map(|word| names.intern(word))
        .collect();
    let root = Frame::dynamic(None, None);
    let mut counts = Vec::new();
    for width in [0, 1, 3, 12, 64, 65, 130] {
        let slots = all[..width]
            .iter()
            .map(|name| (*name, SlotKind::Mutable))
            .collect();
        let Some(layout) = FrameLayout::new(slots).map(std::rc::Rc::new) else {
            usage();
        };
        counts.push(allocations_in(|| {
            let frame = Frame::fixed(&layout, Some(&root), None);
            for name in &all[..width] {
                Frame::declare(&frame, *name, untwist_eval::value::Value::Null);
            }
        }));
    }
    let dynamic = allocations_in(|| {
        let frame = Frame::dynamic(Some(&root), None);
        Frame::declare(&frame, all[0], untwist_eval::value::Value::Null);
    });
    println!(
        "frames: fixed widths 0/1/3/12/64/65/130 {counts:?}, dynamic with one binding {dynamic}"
    );
}

fn main() {
    let mut args = std::env::args().skip(1).filter(|arg| arg != "--bench");
    let Some(path) = args.next() else {
        usage();
    };
    if path == "--frames" {
        frames();
        return;
    }
    let mut deobfuscate = false;
    let mut floor_only = false;
    let mut node_bytes = false;
    for flag in args {
        match flag.as_str() {
            "--deobfuscate" => deobfuscate = true,
            "--floor" => floor_only = true,
            "--nodes" => node_bytes = true,
            _ => usage(),
        }
    }
    if deobfuscate && floor_only {
        usage();
    }
    let resolved = resolve(&path);
    let source = match std::fs::read_to_string(&resolved) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("read {}: {err}", resolved.display());
            std::process::exit(1);
        }
    };
    if node_bytes {
        nodes(&path, &source);
    } else if floor_only {
        floor(&path, &source);
    } else {
        pipeline(&path, &source, deobfuscate);
    }
}
