#![allow(dead_code)]

use oxc_allocator::Allocator;
use oxc_ast::ast::{ObjectProperty, Program};
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use untwist_core::parse::parse_program;
use untwist_core::pass::{
    run_once, run_to_fixpoint, Note, PassCtor, PassGroup, PassOptions, RunReport,
};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::{build_scoping, Semantics};

#[derive(Default)]
pub struct ZeroSpans(pub Vec<String>);

impl<'a> Visit<'a> for ZeroSpans {
    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.span == Span::new(0, 0) {
            self.0.push(format!("property at {:?}", it.span));
        }
        if it.key.span() == Span::new(0, 0) {
            self.0.push(format!("key at {:?}", it.key.span()));
        }
        walk::walk_object_property(self, it);
    }
}

pub fn pass_options() -> PassOptions {
    PassOptions {
        unsafe_transforms: true,
        extended: false,
        max_iterations: Some(10),
        fuel: None,
        ..PassOptions::default()
    }
}

pub fn run_report(source: &str) -> (String, RunReport) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let (_sem, report) = deobfuscate_and_sweep(&alloc, &mut parsed.program, sem, &pass_options());
    (print(&parsed.program, PrintStyle::Pretty), report)
}

fn deobfuscate_and_sweep<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    let (sem, mut report) = untwist_deobfuscate::run_deobfuscate(alloc, program, sem, opts);
    let (sem, last) = untwist_deobfuscate::run_final_machinery(alloc, program, sem, opts);
    report.notes.extend(last.notes);
    (sem, report)
}

pub fn run_report_extended(source: &str) -> (String, RunReport) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        extended: true,
        ..pass_options()
    };
    let (_sem, report) = deobfuscate_and_sweep(&alloc, &mut parsed.program, sem, &opts);
    (print(&parsed.program, PrintStyle::Pretty), report)
}

pub fn assert_converged(report: &RunReport) {
    assert!(
        report.converged(),
        "passes did not converge: stop={:?} per_pass={:?}",
        report.stop,
        report.per_pass
    );
}

pub fn run_converged(source: &str) -> (String, RunReport) {
    let (code, report) = run_report(source);
    assert_converged(&report);
    (code, report)
}

pub fn run_passes(source: &str) -> (String, Vec<String>) {
    let (code, report) = run_converged(source);
    (
        code,
        report.notes.into_iter().map(|note| note.message).collect(),
    )
}

pub fn run_ctors_report<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctors: &[PassCtor<'a>],
) -> (String, RunReport) {
    run_ctors_with(alloc, source, ctors, pass_options())
}

pub fn run_ctors_extended<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctors: &[PassCtor<'a>],
) -> String {
    let opts = PassOptions {
        extended: true,
        ..pass_options()
    };
    run_ctors_with(alloc, source, ctors, opts).0
}

fn run_ctors_with<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctors: &[PassCtor<'a>],
    opts: PassOptions,
) -> (String, RunReport) {
    let mut parsed = parse_program(alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(ctors, opts);
    let (_sem, report) = run_to_fixpoint(alloc, &mut parsed.program, sem, &mut group);
    assert_converged(&report);
    (print(&parsed.program, PrintStyle::Pretty), report)
}

pub fn run_ctors<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctors: &[PassCtor<'a>],
) -> (String, Vec<String>) {
    let (code, report) = run_ctors_report(alloc, source, ctors);
    (
        code,
        report.notes.into_iter().map(|note| note.message).collect(),
    )
}

pub fn directive_count(source: &str) -> usize {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    parsed.program.directives.len()
}

fn drain(stage: &str, report: RunReport, notes: &mut Vec<Note>) {
    assert!(
        report.converged(),
        "{stage} did not converge: stop={:?} per_pass={:?}",
        report.stop,
        report.per_pass
    );
    notes.extend(report.notes);
}

pub fn default_run<'a>(alloc: &'a Allocator, source: &'a str, ctor: PassCtor<'a>) -> String {
    run_single(alloc, source, ctor, true)
}

pub fn conservative_run<'a>(alloc: &'a Allocator, source: &'a str, ctor: PassCtor<'a>) -> String {
    run_single(alloc, source, ctor, false)
}

fn run_single<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctor: PassCtor<'a>,
    extended: bool,
) -> String {
    let mut parsed = parse_program(alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        extended,
        ..pass_options()
    };
    let mut group = PassGroup::from_ctors(&[ctor], opts);
    let (_sem, _report) = run_once(alloc, &mut parsed.program, sem, &mut group);
    print(&parsed.program, PrintStyle::Pretty)
}

pub fn run_all_stages(source: &str) -> (String, Vec<String>) {
    messages_of(run_all_stage_notes(source))
}

pub fn run_all_stages_extended(source: &str) -> (String, Vec<String>) {
    messages_of(run_all_stage_notes_extended(source))
}

fn messages_of((code, notes): (String, Vec<Note>)) -> (String, Vec<String>) {
    (code, notes.into_iter().map(|note| note.message).collect())
}

pub fn run_all_stage_notes(source: &str) -> (String, Vec<Note>) {
    run_stages_with(source, pass_options())
}

pub fn run_all_stage_notes_extended(source: &str) -> (String, Vec<Note>) {
    run_stages_with(
        source,
        PassOptions {
            extended: true,
            ..pass_options()
        },
    )
}

pub fn run_all_stage_notes_default_lane(source: &str) -> (String, Vec<Note>) {
    run_stages_with(
        source,
        PassOptions {
            unsafe_transforms: false,
            ..pass_options()
        },
    )
}

fn run_stages_with(source: &str, opts: PassOptions) -> (String, Vec<Note>) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let mut notes = Vec::new();
    let (sem, report) =
        untwist_deobfuscate::run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    drain("deobfuscate", report, &mut notes);
    let (sem, report) =
        untwist_deobfuscate::run_protections(&alloc, &mut parsed.program, sem, &opts);
    drain("protections", report, &mut notes);
    let (sem, report) = untwist_deobfuscate::run_cleanup(&alloc, &mut parsed.program, sem, &opts);
    drain("cleanup", report, &mut notes);
    let (_sem, report) =
        untwist_deobfuscate::run_final_machinery(&alloc, &mut parsed.program, sem, &opts);
    drain("final machinery", report, &mut notes);
    (print(&parsed.program, PrintStyle::Pretty), notes)
}

pub fn run_ctor_groups<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    groups: &[Vec<PassCtor<'a>>],
) -> String {
    let mut parsed = parse_program(alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let mut sem = build_scoping(&parsed.program);
    for ctors in groups {
        let mut group = PassGroup::from_ctors(ctors, pass_options());
        let (next, report) = run_to_fixpoint(alloc, &mut parsed.program, sem, &mut group);
        assert!(report.converged(), "{:?}", report.stop);
        sem = next;
    }
    print(&parsed.program, PrintStyle::Pretty)
}
