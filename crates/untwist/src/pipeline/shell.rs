use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{Note, NoteKind, StopReason};
use untwist_deobfuscate::directives::{strip_anonymous_shell, ShellStrip};

use super::notes::{ANONYMOUS_SHELL_KEPT, ANONYMOUS_SHELL_STRIPPED};
use super::stages::round_of;
use crate::report::StageReport;

const STRIP_SHELL: &str = "strip-shell";

fn hoisted_parameters_detail(names: &[String]) -> String {
    match names.len() {
        0 => "; it had no parameters".to_string(),
        1 => format!("; 1 parameter hoisted as var: {}", names[0]),
        count => format!("; {count} parameters hoisted as var: {}", names.join(", ")),
    }
}

pub(super) fn strip_shell<'a>(
    (alloc, program): (&'a Allocator, &mut Program<'a>),
    stages: &mut Vec<StageReport>,
    notes: &mut Vec<Note>,
) -> bool {
    let started = Instant::now();
    let names = match strip_anonymous_shell(program, alloc) {
        ShellStrip::Absent => return false,
        ShellStrip::Kept { span, reason } => {
            notes.push(Note {
                pass: STRIP_SHELL,
                kind: NoteKind::Refusal,
                message: format!("{ANONYMOUS_SHELL_KEPT} {}", reason.reason()),
                span: Some(span),
            });
            return false;
        }
        ShellStrip::Stripped { hoisted } => hoisted,
    };
    let elapsed = started.elapsed();
    stages.push(StageReport {
        stage: STRIP_SHELL,
        round: round_of(stages, STRIP_SHELL),
        iterations: 1,
        changes: 1,
        per_pass: vec![(STRIP_SHELL, 1)],
        per_pass_elapsed: vec![(STRIP_SHELL, elapsed)],
        fixpoint_per_pass: Vec::new(),
        fixpoint_per_pass_elapsed: Vec::new(),
        iteration_elapsed: Vec::new(),
        elapsed,
        stop: StopReason::Converged,
        #[cfg(feature = "test-hooks")]
        work: Vec::new(),
    });
    notes.push(Note {
        pass: STRIP_SHELL,
        kind: NoteKind::Progress,
        message: format!(
            "{ANONYMOUS_SHELL_STRIPPED}{}",
            hoisted_parameters_detail(&names)
        ),
        span: None,
    });
    true
}
