use std::io::Write;

use crate::depth_grid::{program, sampled, Kind};
use crate::depth_mix::{self, Mix};
use crate::depth_names;
use oxc_allocator::Allocator;
use untwist_core::depth::{scan, scan_as, DepthRefusal, Goal, UNLIMITED};
use untwist_core::parse::{has_error, parse_with, SourceType};

pub const UNIT: u64 = 4096;
pub const SLACK: u64 = 1 << 20;
pub const FLAG_SLACK: u64 = 128 << 10;
pub const STACK_TUNABLE: (&str, &str) = ("GLIBC_TUNABLES", "glibc.pthread.stack_cache_size=0");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Class(Kind),
    Sample,
    Mix(Mix),
    MixSample,
    Names,
}

impl Source {
    pub fn from_name(name: &str) -> Option<Source> {
        match name {
            "sample" => return Some(Source::Sample),
            "mix" => return Some(Source::MixSample),
            "names" => return Some(Source::Names),
            _ => {}
        }
        if let Some(mix) = Mix::from_name(name) {
            return Some(Source::Mix(mix));
        }
        Kind::from_name(name).map(Source::Class)
    }

    pub fn program(self, seed: u64, index: u64) -> (String, String) {
        match self {
            Source::Class(kind) => program(kind, seed, index),
            Source::Sample => sampled(seed, index),
            Source::Mix(mix) => depth_mix::program(mix, seed, index),
            Source::MixSample => depth_mix::sampled(seed, index),
            Source::Names => depth_names::program(seed, index),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Program,
    Module,
    Script,
}

impl Mode {
    fn letter(self) -> char {
        match self {
            Mode::Program => 'P',
            Mode::Module => 'M',
            Mode::Script => 'S',
        }
    }
}

fn stack_rss() -> u64 {
    let marker = 0u8;
    let here = std::ptr::addr_of!(marker) as usize;
    let Ok(maps) = std::fs::read_to_string("/proc/thread-self/smaps") else {
        return 0;
    };
    let mut inside = false;
    for line in maps.lines() {
        let first = line.split_whitespace().next().unwrap_or("");
        if let Some((low, high)) = first.split_once('-') {
            if let (Ok(low), Ok(high)) = (
                usize::from_str_radix(low, 16),
                usize::from_str_radix(high, 16),
            ) {
                inside = low <= here && here < high;
                continue;
            }
        }
        if let (true, Some(rest)) = (inside, line.strip_prefix("Rss:")) {
            let kb: u64 = rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0);
            return kb * 1024;
        }
    }
    0
}

fn parse_in(alloc: &Allocator, source: &str, module: bool) -> bool {
    let source_type = if module {
        SourceType::mjs()
    } else {
        SourceType::cjs()
    };
    has_error(&parse_with(alloc, source, source_type.with_jsx(true)).diagnostics)
}

const TYPESCRIPT_ONLY: &[&str] = &["8002", "8005", "8011", "8012", "8016", "8037"];
const EVIDENCE_NESTING: u64 = 500_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Evidence {
    Clean,
    Typescript,
    Earlier,
    Unrelated,
}

impl Evidence {
    fn valid(self) -> bool {
        matches!(self, Evidence::Typescript | Evidence::Earlier)
    }
}

fn first_error(alloc: &Allocator, source: &str, source_type: SourceType) -> Option<(u32, bool)> {
    let parsed = parse_with(alloc, source, source_type);
    if !has_error(&parsed.diagnostics) {
        return None;
    }
    let only = parsed.diagnostics.iter().any(|diagnostic| {
        diagnostic.code.scope.as_deref() == Some("TS")
            && diagnostic
                .code
                .number
                .as_deref()
                .is_some_and(|number| TYPESCRIPT_ONLY.contains(&number))
    });
    let at = parsed
        .diagnostics
        .iter()
        .flat_map(|diagnostic| diagnostic.labels.iter().map(|label| label.offset()))
        .min()
        .unwrap_or(u32::MAX);
    Some((at, only))
}

const COMPLETIONS: &[&str] = &[" T", "T> {}", " T {}", " a;", " I {}", " T]: U }"];

fn typescript_at(alloc: &Allocator, source: &str, module: bool) -> bool {
    let script = if module {
        SourceType::mjs()
    } else {
        SourceType::cjs()
    };
    let Some((javascript, only)) = first_error(alloc, source, script.with_jsx(true)) else {
        return false;
    };
    let typescript = SourceType::ts().with_module(module).with_jsx(true);
    only || first_error(alloc, source, typescript).is_none_or(|(at, _)| at > javascript)
}

fn completed(source: &str, site: usize, completion: &str) -> String {
    let mut end = site.min(source.len());
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{completion}", &source[..end])
}

fn judge_refusal(source: &str, module: bool, site: usize) -> Evidence {
    let alloc = Allocator::default();
    let script = if module {
        SourceType::mjs()
    } else {
        SourceType::cjs()
    };
    let Some((javascript, only)) = first_error(&alloc, source, script.with_jsx(true)) else {
        return Evidence::Clean;
    };
    if only {
        return Evidence::Typescript;
    }
    let completes = COMPLETIONS
        .iter()
        .any(|completion| typescript_at(&alloc, &completed(source, site, completion), module));
    if completes {
        return Evidence::Typescript;
    }
    if usize::try_from(javascript).is_ok_and(|at| at < site) {
        return Evidence::Earlier;
    }
    Evidence::Unrelated
}

pub fn refusal_evidence(source: &str, module: bool, site: usize) -> Evidence {
    let nesting = u64::try_from(source.len())
        .unwrap_or(u64::MAX)
        .min(EVIDENCE_NESTING);
    let Ok(size) = usize::try_from(UNIT * nesting + SLACK) else {
        return Evidence::Unrelated;
    };
    let source = source.to_owned();
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(move || judge_refusal(&source, module, site))
        .ok()
        .and_then(|handle| handle.join().ok())
        .unwrap_or(Evidence::Unrelated)
}

fn refuses(source: &str, mode: Mode, end: usize) -> bool {
    let mut end = end.min(source.len());
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    scanned(&source[..end], mode) == Err(DepthRefusal::Typescript)
}

pub fn refusal_site(source: &str, module: Option<bool>) -> usize {
    let mode = match module {
        None => Mode::Program,
        Some(true) => Mode::Module,
        Some(false) => Mode::Script,
    };
    let (mut low, mut high) = (0, source.len());
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if refuses(source, mode, middle) {
            high = middle;
        } else {
            low = middle;
        }
    }
    high
}

type Parsed = (u64, bool, Allocator);

fn run_parse(
    source: String,
    module: bool,
    nesting: u32,
    alloc: Allocator,
) -> Result<Parsed, String> {
    let size = UNIT * u64::from(nesting) + SLACK;
    let size = usize::try_from(size).map_err(|e| format!("size {e}"))?;
    let handle = std::thread::Builder::new()
        .stack_size(size)
        .spawn(move || {
            let mut alloc = alloc;
            alloc.reset();
            let errors = parse_in(&alloc, &source, module);
            (stack_rss(), errors, alloc)
        })
        .map_err(|e| format!("spawn {e}"))?;
    handle.join().map_err(|_| "panic".to_owned())
}

fn scanned(source: &str, mode: Mode) -> Result<u32, DepthRefusal> {
    let result = match mode {
        Mode::Program => scan(source, &UNLIMITED),
        Mode::Module => scan_as(source, &UNLIMITED, Goal::Module),
        Mode::Script => scan_as(source, &UNLIMITED, Goal::Script),
    };
    result.map(|found| found.nesting)
}

struct Probe<'a, W: Write> {
    alloc: Option<Allocator>,
    index: u64,
    tag: &'a str,
    out: &'a mut W,
    ratio: f64,
}

impl<W: Write> Probe<'_, W> {
    fn fail(&mut self, mode: Mode, what: &str) {
        let _ = writeln!(
            self.out,
            "F {} {} {what} {}",
            self.index,
            mode.letter(),
            self.tag
        );
    }

    fn judge(&mut self, mode: Mode, nesting: u32, touched: u64) {
        let bound = UNIT * u64::from(nesting.max(1));
        if touched > bound + FLAG_SLACK {
            self.fail(mode, &format!("deep nesting={nesting} touched={touched}"));
        }
        let budget = (bound + SLACK) as f64;
        self.ratio = self.ratio.max(touched as f64 / budget);
    }

    fn refused(&mut self, source: &str, mode: Mode) {
        let (modules, site): (&[bool], usize) = match mode {
            Mode::Program => (&[true, false], refusal_site(source, None)),
            Mode::Module => (&[true], refusal_site(source, Some(true))),
            Mode::Script => (&[false], refusal_site(source, Some(false))),
        };
        let found: Vec<Evidence> = modules
            .iter()
            .map(|&module| refusal_evidence(source, module, site))
            .collect();
        let valid = found
            .iter()
            .find(|evidence| evidence.valid())
            .filter(|_| !found.contains(&Evidence::Clean));
        match valid {
            None => self.fail(mode, &format!("false-refusal at {site} {found:?}")),
            Some(evidence) if mode == Mode::Program => {
                let _ = writeln!(self.out, "Z {} {evidence:?}", self.index);
            }
            Some(_) => {}
        }
    }

    fn goal(&mut self, source: &str, mode: Mode, program: u32) -> Option<(u64, bool)> {
        let module = mode == Mode::Module;
        let nesting = match scanned(source, mode) {
            Ok(nesting) => nesting,
            Err(DepthRefusal::Typescript) => {
                self.refused(source, mode);
                return None;
            }
            Err(_) => {
                self.fail(mode, "scan-refused");
                return None;
            }
        };
        let _ = writeln!(self.out, "G {} {}", mode.letter(), nesting.max(program));
        let _ = self.out.flush();
        let alloc = self.alloc.take().unwrap_or_default();
        match run_parse(source.to_owned(), module, nesting.max(program), alloc) {
            Ok((touched, errors, alloc)) => {
                self.alloc = Some(alloc);
                self.judge(mode, nesting, touched);
                Some((touched, errors))
            }
            Err(error) => {
                self.fail(mode, &error);
                None
            }
        }
    }
}

pub fn check(
    alloc: &mut Option<Allocator>,
    index: u64,
    source: &str,
    tag: &str,
    out: &mut impl Write,
) -> f64 {
    let mut probe = Probe {
        alloc: alloc.take(),
        index,
        tag,
        out,
        ratio: 0.0,
    };
    let program = match scanned(source, Mode::Program) {
        Ok(program) => program,
        Err(DepthRefusal::Typescript) => {
            probe.refused(source, Mode::Program);
            return 0.0;
        }
        Err(_) => {
            probe.fail(Mode::Program, "scan-refused");
            return 0.0;
        }
    };
    let module = probe.goal(source, Mode::Module, program);
    let script = probe.goal(source, Mode::Script, program);
    if let (Some((module, errors)), Some((script, _))) = (module, script) {
        let touched = if errors { module.max(script) } else { module };
        probe.judge(Mode::Program, program, touched);
    }
    *alloc = probe.alloc.take();
    probe.ratio
}

pub fn child(source: Source, seed: u64, start: u64, end: u64, out: &mut impl Write) {
    let mut ratio = 0f64;
    let mut alloc = None;
    let _ = writeln!(out);
    for index in start..end {
        let (text, tag) = source.program(seed, index);
        let _ = writeln!(out, "B {index}");
        let _ = out.flush();
        ratio = ratio.max(check(&mut alloc, index, &text, &tag, out));
    }
    let _ = writeln!(out, "R {ratio}");
    let _ = writeln!(out, "E");
    let _ = out.flush();
}

#[derive(Default, Debug)]
pub struct Tally {
    pub programs: u64,
    pub refused: u64,
    pub typescript: u64,
    pub crashes: u64,
    pub flags: u64,
    pub max_ratio: f64,
    pub failures: Vec<String>,
}

impl Tally {
    pub fn clean(&self) -> bool {
        self.crashes == 0 && self.flags == 0 && self.failures.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "programs={} refused={} typescript_only={} crashes={} flags={} max_touched_over_budget={:.4}",
            self.programs, self.refused, self.typescript, self.crashes, self.flags, self.max_ratio
        )
    }
}

pub struct ChildRun {
    pub stdout: String,
    pub status: String,
}

struct Segment {
    last: Option<u64>,
    mode: String,
    nesting: String,
    finished: bool,
}

fn read_segment(stdout: &str, tally: &mut Tally) -> Segment {
    let mut segment = Segment {
        last: None,
        mode: String::new(),
        nesting: String::new(),
        finished: false,
    };
    for line in stdout.lines() {
        let mut parts = line.splitn(3, ' ');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("B"), Some(index), _) => {
                segment.last = index.parse().ok();
                tally.programs += 1;
            }
            (Some("G"), Some(mode), Some(nesting)) => {
                segment.mode = mode.to_owned();
                segment.nesting = nesting.to_owned();
            }
            (Some("Z"), _, kind) => {
                tally.refused += 1;
                tally.typescript +=
                    u64::from(kind.is_some_and(|kind| kind.ends_with("Typescript")));
            }
            (Some("F"), _, _) => {
                tally.flags += 1;
                tally.failures.push(line.to_owned());
            }
            (Some("R"), Some(ratio), _) => {
                tally.max_ratio = tally.max_ratio.max(ratio.parse().unwrap_or(0.0));
            }
            (Some("E"), _, _) => segment.finished = true,
            _ => {}
        }
    }
    segment
}

pub fn drive(
    source: Source,
    seed: u64,
    range: (u64, u64),
    chunk: u64,
    mut run: impl FnMut(u64, u64) -> ChildRun,
) -> Tally {
    let mut tally = Tally::default();
    let (mut next, end) = range;
    while next < end {
        let stop = end.min(next.saturating_add(chunk.max(1)));
        let outcome = run(next, stop);
        let segment = read_segment(&outcome.stdout, &mut tally);
        if segment.finished {
            next = stop;
            continue;
        }
        let Some(index) = segment.last else {
            tally
                .failures
                .push(format!("child died before any program: {}", outcome.status));
            return tally;
        };
        tally.crashes += 1;
        let (_, tag) = source.program(seed, index);
        tally.failures.push(format!(
            "CRASH {index} mode={} nesting={} {tag} {}",
            segment.mode, segment.nesting, outcome.status
        ));
        next = index + 1;
    }
    tally
}
