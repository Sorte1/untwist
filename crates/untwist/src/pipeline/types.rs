use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{Note, NoteKind};

use super::{Output, Pipeline};
use crate::options::{Progress, TypesMode};
use crate::types::TypesReport;

pub const TYPES_PASS: &str = "types";

pub const TYPES_WITH_SOURCE_MAPS: &str =
    "types: not added, because annotations would move the code under the source map";

pub const TYPES_NOT_BUILT: &str = "types: this build of untwist has no types stage";

#[cfg(feature = "types")]
fn absorb(report: &mut TypesReport, stats: untwist_types::TypeStats) {
    let counts = |counts: untwist_types::Counts| crate::types::TypeCounts {
        eligible: counts.eligible,
        proven: counts.proven,
        annotated: counts.annotated,
        redundant: counts.redundant,
        refused: counts.refused,
    };
    report.bindings.absorb(counts(stats.bindings));
    report.params.absorb(counts(stats.params));
    report.returns.absorb(counts(stats.returns));
    report.files += stats.files;
    report.files_refused += stats.files_refused;
}

fn note(notes: &mut Vec<Note>, message: &str) {
    if notes
        .iter()
        .any(|held| held.pass == TYPES_PASS && held.message == message)
    {
        return;
    }
    notes.push(Note {
        pass: TYPES_PASS,
        kind: NoteKind::Refusal,
        message: message.to_string(),
        span: None,
    });
}

#[cfg(feature = "types")]
fn typed(
    code: &mut String,
    mode: TypesMode,
    assert: bool,
    report: &mut TypesReport,
    notes: &mut Vec<Note>,
) -> bool {
    let mode = match mode {
        TypesMode::Ts => untwist_types::Mode::Ts,
        TypesMode::Jsdoc => untwist_types::Mode::Jsdoc,
    };
    let held = std::mem::take(code);
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if assert {
            return (
                untwist_types::instrumented(&held),
                None,
                untwist_types::TypeStats::default(),
                false,
            );
        }
        let out = untwist_types::annotate(&held, mode);
        (Some(out.code), out.refused, out.stats, out.jsx)
    }));
    match ran {
        Ok((typed_code, refused, stats, jsx)) => {
            absorb(report, stats);
            if let Some(refused) = refused {
                note(notes, refused.message());
            }
            *code = typed_code.unwrap_or(held);
            jsx
        }
        Err(_) => {
            note(
                notes,
                "types: the types stage panicked on a file, which was left untyped",
            );
            *code = held;
            false
        }
    }
}

#[cfg(not(feature = "types"))]
fn typed(
    _code: &mut String,
    _mode: TypesMode,
    _assert: bool,
    _report: &mut TypesReport,
    notes: &mut Vec<Note>,
) -> bool {
    note(notes, TYPES_NOT_BUILT);
    false
}

impl Pipeline {
    pub(super) fn types_stage(&self, output: &mut Output) {
        let Some(mode) = self.options.types else {
            return;
        };
        if self.options.source_map {
            note(&mut output.notes, TYPES_WITH_SOURCE_MAPS);
            return;
        }
        self.options.progress(Progress::StageBegins(TYPES_PASS));
        let started = Instant::now();
        let assert = self.options.hooks().types_assert;
        let mut report = TypesReport::new(mode);
        let notes = &mut output.notes;
        for code in &mut output.codes {
            let jsx = typed(code, mode, assert, &mut report, notes);
            report.jsx_outputs.push(jsx);
        }
        if let Some(bundle) = output.bundle.as_mut() {
            for module in &mut bundle.modules {
                let jsx = typed(&mut module.code, mode, assert, &mut report, notes);
                if mode == TypesMode::Ts && !assert {
                    module.path = crate::types::typed_path(&module.path, jsx);
                }
                report.jsx_modules.push(jsx);
            }
        }
        report.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        output.types = Some(report);
    }
}
