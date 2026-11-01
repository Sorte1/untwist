#![allow(dead_code)]

use untwist_rename::fingerprint::alpha_fingerprint;

pub fn assert_scope_safe(before: &str, after: &str) {
    let expected = alpha_fingerprint(before).expect("the input must parse cleanly");
    let actual = alpha_fingerprint(after).expect("the output must parse cleanly");
    assert_eq!(
        expected, actual,
        "renaming changed the program's binding structure\nbefore: {before}\nafter:  {after}"
    );
}

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, Note, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_rename::{rename_passes, RenameConfig, RenameMode};

pub fn unique_config() -> RenameConfig {
    RenameConfig {
        mode: RenameMode::Unique,
        rename_all: true,
        extended: true,
    }
}

pub fn render(
    source: &str,
    config: RenameConfig,
    style: PrintStyle,
    with_shorthand: bool,
) -> (String, Vec<Note>) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let mut notes = Vec::new();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::new(rename_passes(config), PassOptions::default());
    let (sem, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    assert!(
        report.converged(),
        "the mode stage did not converge: stop={:?}",
        report.stop
    );
    notes.extend(report.notes);
    if with_shorthand {
        let mut group = PassGroup::new(
            vec![untwist_rename::shorthand::boxed()],
            PassOptions::default(),
        );
        let (_sem, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
        assert!(
            report.converged(),
            "the shorthand stage did not converge: stop={:?}",
            report.stop
        );
        notes.extend(report.notes);
    }
    (print(&parsed.program, style), notes)
}

pub fn rename(source: &str, config: RenameConfig) -> String {
    render(source, config, PrintStyle::Pretty, false).0
}

pub fn rename_compact(source: &str, config: RenameConfig) -> String {
    render(source, config, PrintStyle::Compact, false).0
}

pub fn rename_full(source: &str, config: RenameConfig) -> String {
    render(source, config, PrintStyle::Compact, true).0
}
