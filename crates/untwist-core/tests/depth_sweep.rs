#![cfg(all(target_os = "linux", not(debug_assertions)))]

#[allow(dead_code)]
#[path = "../../untwist-testing/src/capped_child.rs"]
mod capped_child;
#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_classes.rs"]
mod depth_classes;
#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_grid.rs"]
mod depth_grid;
#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_mix.rs"]
mod depth_mix;
#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_names.rs"]
mod depth_names;
#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_oracle.rs"]
mod depth_oracle;
#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_soup.rs"]
mod depth_soup;

use std::ffi::OsString;
use std::time::{Duration, Instant};

use capped_child::{Cap, Outcome};
use depth_classes::{generate, Class, ALL};
use depth_oracle::{ChildRun, Source, STACK_TUNABLE};
use depth_soup::Soup;
use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXElement, JSXFragment, RegExpLiteral, TemplateLiteral};
use oxc_ast_visit::{walk, Visit};
use untwist_core::depth::{
    limits_for, scan, scan_as, DepthRefusal, Goal, Limits, UNIT_STACK_BYTES, UNLIMITED,
};
use untwist_core::parse::{has_error, parse_program, parse_with, SourceType};

const CHILD: &str = "UNTWIST_DEPTH_CHILD";
const PROBE_RATIO: f64 = 4.5;
const SOUP_SEED: u64 = 0x5eed_1c0d;
const SOUP_PROGRAMS: usize = 1_000_000;
const SOUP_SECONDS: u64 = 60;
const STACK_SEED: u64 = 0x5eed_1d0c;
const STACK_PROGRAMS: u64 = 50_000;
const STACK_SECONDS: u64 = 60;
const MIX_SEED: u64 = 0x5eed_1e0d;
const MIX_PROGRAMS: u64 = 30_000;
const MIX_SECONDS: u64 = 60;
const NAMES_SEED: u64 = 0x5eed_1f0d;
const NAMES_PROGRAMS: u64 = 20_000;
const NAMES_SECONDS: u64 = 60;
const SEGMENT_ROOM: usize = 1 << 30;
const SEGMENT_SLACK: usize = 1 << 20;
const SEGMENT_DEPTHS: [usize; 2] = [1_000, 10_000];

struct Oracle {
    label: &'static str,
    row: &'static str,
    source: Source,
    seed: u64,
    programs: u64,
    seconds: u64,
}

const STACK: Oracle = Oracle {
    label: "oracle stack",
    row: "stack",
    source: Source::Sample,
    seed: STACK_SEED,
    programs: STACK_PROGRAMS,
    seconds: STACK_SECONDS,
};

const MIX: Oracle = Oracle {
    label: "oracle mix",
    row: "mix",
    source: Source::MixSample,
    seed: MIX_SEED,
    programs: MIX_PROGRAMS,
    seconds: MIX_SECONDS,
};

const NAMES: Oracle = Oracle {
    label: "oracle names",
    row: "names",
    source: Source::Names,
    seed: NAMES_SEED,
    programs: NAMES_PROGRAMS,
    seconds: NAMES_SECONDS,
};

#[derive(Default, Debug, PartialEq)]
struct Literals {
    regexes: u32,
    templates: u32,
    jsx: u32,
}

impl<'a> Visit<'a> for Literals {
    fn visit_reg_exp_literal(&mut self, _it: &RegExpLiteral<'a>) {
        self.regexes += 1;
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.templates += 1;
        walk::walk_template_literal(self, it);
    }

    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        self.jsx += 1;
        walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
        self.jsx += 1;
        walk::walk_jsx_fragment(self, it);
    }
}

fn probe_source(name: &str, n: usize) -> String {
    match name {
        "jsxextends" => format!("x={}1{};", "<T extends U>{".repeat(n), "}</T>".repeat(n)),
        "jsxarrow" => format!("x={}1;", "<T extends U>(a)=>".repeat(n)),
        "conflict" => format!("<<<<<<< a\n{}=======\n>>>>>>> b\n", "x;\n".repeat(n)),
        _ => Class::from_name(name)
            .map(|class| generate(class, n))
            .unwrap_or_default(),
    }
}

fn literal_check(source: &str) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    if has_error(&parsed.diagnostics) {
        println!("skipped: the parse reports errors");
        return;
    }
    let mut ast = Literals::default();
    ast.visit_program(&parsed.program);
    let scanned = scan(source, &UNLIMITED).expect("an unlimited scan admits every input");
    let counted = Literals {
        regexes: scanned.regexes,
        templates: scanned.templates,
        jsx: scanned.jsx_elements,
    };
    assert_eq!(counted, ast);
}

fn goal_literals(source: &str, module: bool) -> Option<Literals> {
    let alloc = Allocator::default();
    let source_type = if module {
        SourceType::mjs()
    } else {
        SourceType::cjs()
    };
    let parsed = parse_with(&alloc, source, source_type.with_jsx(true));
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let mut ast = Literals::default();
    ast.visit_program(&parsed.program);
    Some(ast)
}

fn scanned_as(source: &str, module: bool) -> Result<Literals, DepthRefusal> {
    let goal = if module { Goal::Module } else { Goal::Script };
    let scanned = scan_as(source, &UNLIMITED, goal)?;
    Ok(Literals {
        regexes: scanned.regexes,
        templates: scanned.templates,
        jsx: scanned.jsx_elements,
    })
}

fn soup_check(seed: u64, count: usize) {
    let mut soup = Soup::new(seed);
    let mut compared = [0usize; 2];
    let mut mismatches = Vec::new();
    for _ in 0..count {
        let program = soup.program();
        for (index, module) in [true, false].into_iter().enumerate() {
            let Some(ast) = goal_literals(&program, module) else {
                continue;
            };
            compared[index] += 1;
            let counted = scanned_as(&program, module);
            if counted.as_ref() != Ok(&ast) {
                let goal = if module { "module" } else { "script" };
                mismatches.push(format!("{goal}: scan {counted:?} ast {ast:?}\n{program:?}"));
            }
        }
    }
    println!(
        "soup: {count} programs, {} module and {} script parses compared, {} mismatches",
        compared[0],
        compared[1],
        mismatches.len()
    );
    for mismatch in mismatches.iter().take(60) {
        println!("{mismatch}");
    }
    assert!(mismatches.is_empty());
    assert!(compared.iter().all(|&each| each >= count / 10));
}

fn parse_on_its_segment(source: &str) {
    let limits = Limits {
        print_bytes: u64::MAX,
        ..limits_for(SEGMENT_ROOM, source.len())
    };
    match scan(source, &limits) {
        Err(refusal) => println!("refused {refusal:?}"),
        Ok(found) => {
            let segment = UNIT_STACK_BYTES * found.nesting as usize + SEGMENT_SLACK;
            println!("nesting {} segment {segment}", found.nesting);
            let owned = source.to_owned();
            std::thread::Builder::new()
                .stack_size(segment)
                .spawn(move || parse_only(&owned))
                .expect("a thread on the scanned segment")
                .join()
                .expect("the parse fits its segment");
        }
    }
}

fn parse_only(source: &str) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    println!(
        "diagnostics {} arena {}",
        parsed.diagnostics.len(),
        alloc.used_bytes()
    );
}

#[test]
fn child_entry() {
    let Ok(spec) = std::env::var(CHILD) else {
        return;
    };
    let mut parts = spec.split(':');
    let (Some(row), Some(name), Some(n)) = (parts.next(), parts.next(), parts.next()) else {
        panic!("malformed {CHILD}: {spec}");
    };
    let n: usize = n.parse().expect("a depth");
    if row == "soup" {
        soup_check(name.parse().expect("a seed"), n);
        return;
    }
    if let Some(oracle) = [STACK, MIX, NAMES]
        .into_iter()
        .find(|oracle| oracle.row == row)
    {
        let seed = name.parse().expect("a seed");
        let mut out = std::io::stdout().lock();
        depth_oracle::child(oracle.source, seed, n as u64, oracle.programs, &mut out);
        return;
    }
    let source = probe_source(name, n);
    if row == "segment" {
        parse_on_its_segment(&source);
        return;
    }
    let row = row.to_owned();
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || match row.as_str() {
            "literals" => literal_check(&source),
            _ => parse_only(&source),
        })
        .expect("a 1 GiB stack thread")
        .join()
        .expect("the child row completed");
}

fn run_child(row: &str, name: &str, n: usize) -> (Outcome, Duration) {
    run_capped(row, name, n, Cap::DEFAULT)
}

fn run_capped(row: &str, name: &str, n: usize, cap: Cap) -> (Outcome, Duration) {
    run_capped_with(row, name, n, cap, &[])
}

fn run_capped_with(
    row: &str,
    name: &str,
    n: usize,
    cap: Cap,
    extra: &[(&str, &str)],
) -> (Outcome, Duration) {
    let exe = std::env::current_exe().expect("the test binary");
    let args: Vec<OsString> = ["child_entry", "--exact", "--nocapture", "--test-threads=1"]
        .into_iter()
        .map(OsString::from)
        .collect();
    let mut env = vec![(
        OsString::from(CHILD),
        OsString::from(format!("{row}:{name}:{n}")),
    )];
    env.extend(
        extra
            .iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value))),
    );
    let started = Instant::now();
    let outcome = capped_child::run(exe.as_os_str(), &args, &env, None, cap);
    (outcome, started.elapsed())
}

fn problem(label: &str, outcome: &Outcome) -> Option<String> {
    let clean = !outcome.timed_out
        && !outcome.killed_for_memory
        && outcome.signal.is_none()
        && outcome.exit_code == Some(0);
    (!clean).then(|| {
        format!(
            "{label}: exit {:?} signal {:?} timed out {} memory {}\n{}",
            outcome.exit_code,
            outcome.signal,
            outcome.timed_out,
            outcome.killed_for_memory,
            String::from_utf8_lossy(&outcome.stdout)
        )
    })
}

fn generated_literals_agree(failures: &mut Vec<String>) {
    for &class in ALL {
        let (outcome, _) = run_child("literals", class.name(), 1_000);
        let label = format!("literals {} at 1000", class.name());
        failures.extend(problem(&label, &outcome));
    }
}

fn the_scan_bounds_the_parse_on_its_segment(failures: &mut Vec<String>) {
    for &class in ALL {
        for depth in SEGMENT_DEPTHS {
            let (outcome, _) = run_child("segment", class.name(), depth);
            let label = format!("segment {} at {depth}", class.name());
            failures.extend(problem(&label, &outcome));
        }
    }
    println!("segment: {} classes at {SEGMENT_DEPTHS:?}", ALL.len());
}

fn rewind_sites_stay_linear(failures: &mut Vec<String>) {
    for site in ["jsxextends", "jsxarrow", "conflict"] {
        let (small, small_time) = run_child("probe", site, 1_000);
        let (large, large_time) = run_child("probe", site, 2_000);
        failures.extend(problem(&format!("probe {site} 1000"), &small));
        failures.extend(problem(&format!("probe {site} 2000"), &large));
        let rss = large.peak_rss_kb as f64 / small.peak_rss_kb.max(1) as f64;
        let time = large_time.as_secs_f64() / small_time.as_secs_f64().max(1e-3);
        println!(
            "probe {site}: rss {} -> {} kB ({rss:.2}x), {:?} -> {:?} ({time:.2}x)",
            small.peak_rss_kb, large.peak_rss_kb, small_time, large_time
        );
        if rss > PROBE_RATIO || time > PROBE_RATIO {
            failures.push(format!("probe {site}: rss {rss:.2}x, time {time:.2}x"));
        }
    }
}

fn soup_agrees_with_both_goals(failures: &mut Vec<String>) {
    let seed = SOUP_SEED.to_string();
    let (outcome, elapsed) = run_child("soup", &seed, SOUP_PROGRAMS);
    failures.extend(problem("soup", &outcome));
    println!("soup: {elapsed:?}");
    if elapsed > Duration::from_secs(SOUP_SECONDS) {
        failures.push(format!("soup took {elapsed:?}"));
    }
}

fn oracle_sample(oracle: &Oracle, failures: &mut Vec<String>) {
    let seed = oracle.seed.to_string();
    let started = Instant::now();
    let tally = depth_oracle::drive(
        oracle.source,
        oracle.seed,
        (0, oracle.programs),
        oracle.programs,
        |from, _| {
            let start = usize::try_from(from).expect("a start index");
            let (outcome, _) =
                run_capped_with(oracle.row, &seed, start, Cap::DEFAULT, &[STACK_TUNABLE]);
            ChildRun {
                stdout: String::from_utf8_lossy(&outcome.stdout).into_owned(),
                status: format!(
                    "exit {:?} signal {:?} timed out {} memory {}",
                    outcome.exit_code, outcome.signal, outcome.timed_out, outcome.killed_for_memory
                ),
            }
        },
    );
    let elapsed = started.elapsed();
    println!("{}: {} in {elapsed:?}", oracle.label, tally.summary());
    for failure in &tally.failures {
        println!("{failure}");
    }
    if !tally.clean() || tally.programs < oracle.programs {
        failures.push(format!("{}: {}", oracle.label, tally.summary()));
    }
    if elapsed > Duration::from_secs(oracle.seconds) {
        failures.push(format!("{} took {elapsed:?}", oracle.label));
    }
}

fn the_harness_reports_a_memory_kill(failures: &mut Vec<String>) {
    let tight = Cap {
        memory_max: "300M",
        timeout_s: 120,
    };
    let (outcome, _) = run_capped("probe", "arrowobj", 6_000, tight);
    if !outcome.killed_for_memory {
        failures.push(format!(
            "the 300M canary was not reported as a memory kill: {outcome:?}"
        ));
    }
}

#[test]
fn depth_sweep() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let mut failures = Vec::new();
    the_harness_reports_a_memory_kill(&mut failures);
    generated_literals_agree(&mut failures);
    the_scan_bounds_the_parse_on_its_segment(&mut failures);
    rewind_sites_stay_linear(&mut failures);
    soup_agrees_with_both_goals(&mut failures);
    oracle_sample(&STACK, &mut failures);
    oracle_sample(&MIX, &mut failures);
    oracle_sample(&NAMES, &mut failures);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
