use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{Note, NoteKind};

use super::{Output, Pipeline};
use crate::options::{Progress, TypesMode};
use crate::types::TypesReport;

pub const TYPES_PASS: &str = "types";

pub const TYPES_WITH_SOURCE_MAPS: &str =
    "not added, because annotations would move the code under the source map";

pub const TYPES_NOT_BUILT: &str = "this build of untwist has no types stage";

pub const TYPES_KEPT_JS_NAMES: &str =
    "a module loads another by a computed name, so the TypeScript files keep their .js names";

#[cfg(feature = "types")]
type Realm = Option<untwist_types::Realm>;

#[cfg(not(feature = "types"))]
type Realm = Option<()>;

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
    report.near_eval += stats.near_eval;
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
        kind: NoteKind::Progress,
        message: message.to_string(),
        span: None,
    });
}

struct Typed {
    jsx: bool,
    typescript: bool,
}

#[cfg(feature = "types")]
fn realm_of(output: &Output) -> Realm {
    let mut texts: Vec<untwist_types::RunText<'_>> = output
        .codes
        .iter()
        .map(|code| untwist_types::RunText {
            code,
            dir: "",
            commonjs: false,
        })
        .collect();
    let mut known = Vec::new();
    if let Some(bundle) = &output.bundle {
        for module in &bundle.modules {
            let dir = module.path.rfind('/').map_or("", |at| &module.path[..at]);
            texts.push(untwist_types::RunText {
                code: &module.code,
                dir,
                commonjs: !module.path.ends_with(".mjs"),
            });
            known.push(untwist_types::normalized("", &module.path).unwrap_or_default());
        }
    }
    (texts.len() > 1).then(|| untwist_types::realm_in(&texts, &known))
}

#[cfg(not(feature = "types"))]
fn realm_of(_output: &Output) -> Realm {
    None
}

#[cfg(feature = "types")]
fn typed(
    code: &mut String,
    (mode, assert, realm): (TypesMode, bool, Realm),
    report: &mut TypesReport,
    notes: &mut Vec<Note>,
) -> Typed {
    let typed_mode = match mode {
        TypesMode::Ts => untwist_types::Mode::Ts,
        TypesMode::Jsdoc => untwist_types::Mode::Jsdoc,
    };
    let held = std::mem::take(code);
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if assert {
            let checked = untwist_types::instrumented(&held);
            return (checked, None, untwist_types::TypeStats::default(), false);
        }
        let out = untwist_types::annotate_in(&held, typed_mode, realm);
        (Some(out.code), out.refused, out.stats, out.jsx)
    }));
    match ran {
        Ok((typed_code, refused, stats, jsx)) => {
            absorb(report, stats);
            if let Some(refused) = refused {
                note(notes, refused.message());
            }
            *code = typed_code.unwrap_or(held);
            Typed {
                jsx,
                typescript: mode == TypesMode::Ts && !assert && refused.is_none(),
            }
        }
        Err(_) => {
            note(
                notes,
                "the types stage panicked on a file, which was left untyped",
            );
            *code = held;
            Typed {
                jsx: false,
                typescript: false,
            }
        }
    }
}

#[cfg(not(feature = "types"))]
fn typed(
    _code: &mut String,
    _: (TypesMode, bool, Realm),
    _report: &mut TypesReport,
    notes: &mut Vec<Note>,
) -> Typed {
    note(notes, TYPES_NOT_BUILT);
    Typed {
        jsx: false,
        typescript: false,
    }
}

#[cfg(feature = "types")]
fn declarations(code: &str, realm: Realm) -> Option<String> {
    std::panic::catch_unwind(|| untwist_types::declarations_in(code, realm))
        .ok()
        .flatten()
}

#[cfg(not(feature = "types"))]
fn declarations(_code: &str, _realm: Realm) -> Option<String> {
    None
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
        let realm = realm_of(output);
        let how = (mode, assert, realm);
        let untyped = crate::types::Untyped {
            codes: output.codes.clone(),
            modules: output
                .bundle
                .as_ref()
                .map(|bundle| {
                    bundle
                        .modules
                        .iter()
                        .map(|module| module.code.clone())
                        .collect()
                })
                .unwrap_or_default(),
        };
        let notes = &mut output.notes;
        for code in &mut output.codes {
            let done = typed(code, how, &mut report, notes);
            report.jsx_outputs.push(done.jsx);
            report.ts_outputs.push(done.typescript);
        }
        if let Some(bundle) = output.bundle.as_mut() {
            for module in bundle.modules.iter_mut() {
                let done = typed(&mut module.code, how, &mut report, notes);
                report.ts_modules.push(done.typescript);
                let declares = mode == TypesMode::Jsdoc && !assert;
                report.declarations.push(
                    declares
                        .then(|| declarations(&module.code, realm))
                        .flatten(),
                );
                report.jsx_modules.push(done.jsx);
            }
        }
        let mut held = std::mem::take(&mut output.notes);
        crate::types::settle_names(output, &mut report, &untyped, &mut held);
        output.notes = held;
        if report.near_eval > 0 {
            let message = format!("{} positions skipped near a direct eval", report.near_eval);
            let mut held = std::mem::take(&mut output.notes);
            note(&mut held, &message);
            output.notes = held;
        }
        report.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        output.types = Some(report);
    }
}

#[cfg(feature = "types")]
pub(crate) fn specifier_list(
    code: &str,
    typescript: bool,
) -> Option<Vec<(u32, u32, Option<String>)>> {
    untwist_types::specifiers(code, typescript).map(|found| {
        found
            .into_iter()
            .map(|specifier| (specifier.start, specifier.end, specifier.value))
            .collect()
    })
}

#[cfg(not(feature = "types"))]
pub(crate) fn specifier_list(
    _code: &str,
    _typescript: bool,
) -> Option<Vec<(u32, u32, Option<String>)>> {
    None
}

pub(crate) fn kept_names_note(notes: &mut Vec<Note>) {
    note(notes, TYPES_KEPT_JS_NAMES);
}
