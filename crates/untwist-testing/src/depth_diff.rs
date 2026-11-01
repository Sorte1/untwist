use std::path::{Path, PathBuf};

use crate::depth_classes::{generate, repeatchain, ALL};
use crate::depth_edges;
use crate::depth_mix::{self, Mix};
use crate::depth_reference as reference;
use crate::depth_soup::Soup;
use untwist_core::depth::{self, DepthRefusal, DepthScan, Goal, Limits, UNLIMITED};

pub const SOUP_SEED: u64 = 0x5eed_1c0d;
pub const MIX_SEED: u64 = 0x5eed_1e0d;
pub const EDGE_SEED: u64 = 0x5eed_2a0d;
pub const CLASS_DEPTHS: [usize; 7] = [0, 1, 2, 3, 7, 50, 1_000];
const FULL_BYTES: usize = 1 << 20;
const KEPT: usize = 20;
const SHOWN: usize = 400;

type Fields = [u64; 7];
type Outcome = Result<Fields, u8>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Soup,
    Multi,
    Grammar,
    Mutate,
    Edges,
}

impl Class {
    pub fn from_name(name: &str) -> Option<Class> {
        Some(match name {
            "soup" => Class::Soup,
            "multi" => Class::Multi,
            "grammar" => Class::Grammar,
            "mutate" => Class::Mutate,
            "edges" => Class::Edges,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Entry {
    Program,
    Module,
    Script,
}

const ENTRIES: [Entry; 3] = [Entry::Program, Entry::Module, Entry::Script];

#[derive(Default, Debug)]
pub struct Tally {
    pub programs: u64,
    pub bytes: u64,
    pub scans: u64,
    pub accepted: u64,
    pub refused: [u64; 4],
    pub mismatches: u64,
    pub examples: Vec<String>,
}

impl Tally {
    pub fn merge(&mut self, other: Tally) {
        self.programs += other.programs;
        self.bytes += other.bytes;
        self.scans += other.scans;
        self.accepted += other.accepted;
        for (mine, theirs) in self.refused.iter_mut().zip(other.refused) {
            *mine += theirs;
        }
        self.mismatches += other.mismatches;
        let room = KEPT.saturating_sub(self.examples.len());
        self.examples.extend(other.examples.into_iter().take(room));
    }

    pub fn summary(&self) -> String {
        format!(
            "programs={} bytes={} scans={} accepted={} refused nesting={} speculative={} print={} typescript={} mismatches={}",
            self.programs,
            self.bytes,
            self.scans,
            self.accepted,
            self.refused[0],
            self.refused[1],
            self.refused[2],
            self.refused[3],
            self.mismatches
        )
    }

    fn count(&mut self, outcome: &Outcome) {
        self.scans += 1;
        match outcome {
            Ok(_) => self.accepted += 1,
            Err(kind) => self.refused[usize::from(*kind)] += 1,
        }
    }
}

fn fields(scan: &DepthScan) -> Fields {
    [
        u64::from(scan.nesting),
        u64::from(scan.speculative_parens),
        scan.speculative_pairs,
        scan.print_weight,
        u64::from(scan.regexes),
        u64::from(scan.templates),
        u64::from(scan.jsx_elements),
    ]
}

fn reference_fields(scan: &reference::DepthScan) -> Fields {
    [
        u64::from(scan.nesting),
        u64::from(scan.speculative_parens),
        scan.speculative_pairs,
        scan.print_weight,
        u64::from(scan.regexes),
        u64::from(scan.templates),
        u64::from(scan.jsx_elements),
    ]
}

fn kind(refusal: DepthRefusal) -> u8 {
    match refusal {
        DepthRefusal::Nesting => 0,
        DepthRefusal::SpeculativeParens => 1,
        DepthRefusal::PrintSize => 2,
        DepthRefusal::Typescript => 3,
    }
}

fn reference_kind(refusal: reference::DepthRefusal) -> u8 {
    match refusal {
        reference::DepthRefusal::Nesting => 0,
        reference::DepthRefusal::SpeculativeParens => 1,
        reference::DepthRefusal::PrintSize => 2,
        reference::DepthRefusal::Typescript => 3,
    }
}

fn current(source: &str, limits: &Limits, entry: Entry) -> Outcome {
    let result = match entry {
        Entry::Program => depth::scan(source, limits),
        Entry::Module => depth::scan_as(source, limits, Goal::Module),
        Entry::Script => depth::scan_as(source, limits, Goal::Script),
    };
    result
        .as_ref()
        .map(fields)
        .map_err(|refusal| kind(*refusal))
}

fn referenced(source: &str, limits: &Limits, entry: Entry) -> Outcome {
    let limits = reference::Limits {
        nesting: limits.nesting,
        speculative_parens: limits.speculative_parens,
        speculative_pairs: limits.speculative_pairs,
        print_bytes: limits.print_bytes,
    };
    let result = match entry {
        Entry::Program => reference::scan(source, &limits),
        Entry::Module => reference::scan_as(source, &limits, reference::Goal::Module),
        Entry::Script => reference::scan_as(source, &limits, reference::Goal::Script),
    };
    result
        .as_ref()
        .map(reference_fields)
        .map_err(|refusal| reference_kind(*refusal))
}

fn estimate(found: &Fields, length: usize) -> u64 {
    let scan = DepthScan {
        print_weight: found[3],
        ..DepthScan::default()
    };
    depth::print_estimate(&scan, length)
}

fn at(found: &Fields, length: usize) -> Limits {
    Limits {
        nesting: u32::try_from(found[0]).unwrap_or(u32::MAX),
        speculative_parens: u32::try_from(found[1]).unwrap_or(u32::MAX),
        speculative_pairs: found[2],
        print_bytes: estimate(found, length),
    }
}

fn lowered(limits: Limits, field: usize, by: impl Fn(u64) -> u64) -> Limits {
    let mut next = UNLIMITED;
    match field {
        0 => next.nesting = u32::try_from(by(u64::from(limits.nesting))).unwrap_or(u32::MAX),
        1 => {
            next.speculative_parens =
                u32::try_from(by(u64::from(limits.speculative_parens))).unwrap_or(u32::MAX);
        }
        2 => next.speculative_pairs = by(limits.speculative_pairs),
        _ => next.print_bytes = by(limits.print_bytes),
    }
    next
}

fn variants(found: Option<&Fields>, length: usize, full: bool) -> Vec<Limits> {
    let mut out = vec![Limits {
        nesting: 0,
        ..UNLIMITED
    }];
    if full {
        out.extend([1, 3].map(|nesting| Limits {
            nesting,
            ..UNLIMITED
        }));
    }
    let Some(found) = found else {
        return out;
    };
    let measured = at(found, length);
    for field in 0..4 {
        out.push(lowered(measured, field, |value| value));
        out.push(lowered(measured, field, |value| value.saturating_sub(1)));
        if full {
            out.push(lowered(measured, field, |value| value / 2));
        }
    }
    if full {
        out.push(measured);
        let below = |value: u32| value.saturating_sub(1);
        out.push(Limits {
            nesting: below(measured.nesting),
            speculative_parens: below(measured.speculative_parens),
            speculative_pairs: measured.speculative_pairs.saturating_sub(1),
            print_bytes: measured.print_bytes.saturating_sub(1),
        });
    }
    out
}

fn check(label: &str, source: &str, limits: &Limits, entry: Entry, tally: &mut Tally) -> Outcome {
    let expected = referenced(source, limits, entry);
    let actual = current(source, limits, entry);
    tally.count(&expected);
    if actual != expected {
        tally.mismatches += 1;
        if tally.examples.len() < KEPT {
            let shown: String = source.chars().take(SHOWN).collect();
            tally.examples.push(format!(
                "{label} {entry:?} {limits:?}: current {actual:?} reference {expected:?}\n{shown:?}"
            ));
        }
    }
    expected
}

pub fn compare(label: &str, source: &str, full: bool, tally: &mut Tally) {
    tally.programs += 1;
    tally.bytes += source.len() as u64;
    let mut widest = None;
    for entry in ENTRIES {
        let outcome = check(label, source, &UNLIMITED, entry, tally);
        if entry == Entry::Program {
            widest = outcome.ok();
        }
    }
    let entries: &[Entry] = if full { &ENTRIES } else { &ENTRIES[..1] };
    for limits in variants(widest.as_ref(), source.len(), full) {
        for &entry in entries {
            let _ = check(label, source, &limits, entry, tally);
        }
    }
}

fn generated(class: Class, seed: u64, index: u64) -> String {
    match class {
        Class::Soup => Soup::new(seed.wrapping_mul(1_000_003).wrapping_add(index)).program(),
        Class::Multi => depth_mix::program(Mix::Multi, seed, index).0,
        Class::Grammar => depth_mix::program(Mix::Grammar, seed, index).0,
        Class::Mutate => depth_mix::program(Mix::Mutate, seed, index).0,
        Class::Edges => depth_edges::program(seed, index),
    }
}

pub fn run_class(class: Class, seed: u64, range: (u64, u64), threads: usize) -> Tally {
    let threads = threads.max(1) as u64;
    let span = range.1.saturating_sub(range.0);
    let share = span.div_ceil(threads);
    let mut total = Tally::default();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let from = range.0 + worker * share;
                let to = (from + share).min(range.1);
                scope.spawn(move || {
                    let mut tally = Tally::default();
                    for index in from..to {
                        let label = format!("{class:?} seed {seed} index {index}");
                        compare(&label, &generated(class, seed, index), true, &mut tally);
                    }
                    tally
                })
            })
            .collect();
        for worker in workers {
            total.merge(worker.join().expect("a differential worker"));
        }
    });
    total
}

pub fn run_soup_stream(seed: u64, count: usize) -> Tally {
    let mut soup = Soup::new(seed);
    let mut tally = Tally::default();
    for index in 0..count {
        let label = format!("soup seed {seed} program {index}");
        compare(&label, &soup.program(), true, &mut tally);
    }
    tally
}

pub fn run_depth_classes() -> Tally {
    let mut tally = Tally::default();
    for &class in ALL {
        for n in CLASS_DEPTHS {
            let label = format!("class {} at {n}", class.name());
            compare(&label, &generate(class, n), true, &mut tally);
        }
    }
    compare("repeatchain 3", &repeatchain(3), true, &mut tally);
    tally
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx"))
        {
            out.push(path);
        }
    }
}

pub fn fixture_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&workspace().join("fixtures"), &mut out);
    out
}

pub fn run_fixtures(paths: &[PathBuf]) -> (Tally, usize) {
    let mut tally = Tally::default();
    let mut unreadable = 0;
    for path in paths {
        let Ok(text) = std::fs::read_to_string(path) else {
            unreadable += 1;
            continue;
        };
        let label = path.display().to_string();
        compare(&label, &text, text.len() <= FULL_BYTES, &mut tally);
    }
    (tally, unreadable)
}
