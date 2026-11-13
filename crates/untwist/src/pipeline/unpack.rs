use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::ast_util::respan::reset_synthetic_spans;
use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{Note, NoteKind, StopReason};
use untwist_core::reflow::{GuardRecord, PendingReaders};
use untwist_core::source_map::{self, Mapped, Origins, OwnedSourceMap, Recording, Through};

use super::boundary::Boundary;
use super::input::{final_trees, guard_admits, FinalTextGuard, FinalTextRefusal};
use super::notes::{
    guard_notes, SOURCE_READER_UNGUARDED_MODULE, UNPACK_PANICKED, UNPACK_TEXT_UNPARSABLE,
};
use super::refusal::UNPACK_TEXT_TOO_DEEP;
use super::stages::round_of;
use super::Pipeline;
use crate::bundle::BundleOutput;
use crate::options::Progress;
use crate::report::StageReport;

#[derive(Clone, Copy)]
pub(super) struct Prior<'p> {
    pub(super) guards: &'p GuardRecord,
    pub(super) maps: &'p [Option<OwnedSourceMap>],
}

impl<'p> Prior<'p> {
    pub(super) fn guards(guards: &'p GuardRecord) -> Self {
        Self { guards, maps: &[] }
    }
}

pub(super) struct PendingMap {
    mapped: Mapped,
    start: usize,
    input: Option<usize>,
}

type PendingMaps = Vec<Option<PendingMap>>;

impl Pipeline {
    pub(super) fn unpack_options(&self) -> untwist_unpack::UnpackOptions {
        untwist_unpack::UnpackOptions {
            split_hoisted: self.options.split_hoisted,
            assume_modules: self.options.split_assume_modules,
            conservative: !self.options.extended(),
            trust_free_loader: self.options.unsafe_transforms,
        }
    }

    pub(super) fn unpack_final_texts<T: AsRef<str>>(
        &self,
        codes: &[T],
        prior: Prior<'_>,
        guard: (FinalTextGuard, bool),
        stages: &mut Vec<StageReport>,
        notes: &mut Vec<Note>,
        panicked: &mut bool,
    ) -> Option<BundleOutput> {
        if !self.options.unpack {
            return None;
        }
        reset_synthetic_spans();
        let _boundary = Boundary::enter();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (bundle, pending, readers) = self.unpack_in_arena(codes, guard, stages, notes)?;
            let (mut bundle, tally, record) = bundle.reflowed(prior.guards);
            if readers.iter().any(|module| module.unguarded(&record)) {
                notes.push(Note {
                    pass: "pipeline",
                    kind: NoteKind::Refusal,
                    message: SOURCE_READER_UNGUARDED_MODULE.to_string(),
                    span: None,
                });
                return None;
            }
            notes.extend(guard_notes(tally));
            attach_module_maps(&mut bundle, pending, prior.maps, codes);
            Some(bundle)
        }));
        match outcome {
            Ok(bundle) => bundle,
            Err(_) => {
                *panicked = true;
                notes.push(Note {
                    pass: "pipeline",
                    kind: NoteKind::Refusal,
                    message: UNPACK_PANICKED.to_string(),
                    span: None,
                });
                None
            }
        }
    }

    fn unpack_in_arena<T: AsRef<str>>(
        &self,
        codes: &[T],
        (guard, unbundled): (FinalTextGuard, bool),
        stages: &mut Vec<StageReport>,
        notes: &mut Vec<Note>,
    ) -> Option<(BundleOutput, PendingMaps, Vec<PendingReaders>)> {
        self.options.progress(Progress::StageBegins("unpack"));
        if unbundled {
            return unpack_nothing(codes, guard, stages, notes);
        }
        let alloc = Allocator::default();
        let mut programs = match final_trees(&alloc, codes, guard) {
            Ok(programs) => programs,
            Err(refusal) => {
                refuse_final_texts(notes, refusal);
                return None;
            }
        };
        let texts: Vec<usize> = programs
            .iter()
            .map(|program| program.source_text.as_ptr() as usize)
            .collect();
        let recording = self.options.source_map.then(Recording::start);
        let mut bundle = self.unpack_stage(&alloc, &mut programs, stages, notes)?;
        self.relex_modules(&alloc, &mut bundle, notes);
        self.after_unpack(&alloc, &mut bundle, stages, notes);
        let unreflowed = BundleOutput::unreflowed(&bundle, crate::bundle::single_text_len(codes));
        let readers = bundle
            .modules
            .values()
            .map(|module| PendingReaders::of(module.program()))
            .collect();
        let pending = match recording {
            Some(recording) => module_maps(&mut bundle, &texts, &recording.finish()),
            None => Vec::new(),
        };
        Some((unreflowed, pending, readers))
    }

    fn unpack_stage<'a>(
        &self,
        alloc: &'a Allocator,
        programs: &mut [Program<'a>],
        stages: &mut Vec<StageReport>,
        notes: &mut Vec<Note>,
    ) -> Option<untwist_unpack::Bundle<'a>> {
        let started = Instant::now();
        let (bundle, unpack_notes) = untwist_unpack::unpack_named_inputs(
            alloc,
            programs,
            &self.options.mappings,
            self.unpack_options(),
            &self.options.input_names,
        );
        notes.extend(unpack_notes);
        let modules = bundle.as_ref().map_or(0, |bundle| bundle.modules.len()) as u32;
        push_unpack_row(stages, started, modules);
        bundle
    }
}

fn module_maps(
    bundle: &mut untwist_unpack::Bundle<'_>,
    texts: &[usize],
    origins: &Origins,
) -> PendingMaps {
    bundle
        .modules
        .values_mut()
        .map(|module| {
            let text = module.program().source_text.as_ptr() as usize;
            let input = texts.iter().position(|probe| *probe == text);
            let (mapped, start) = source_map::mapped_window(module.program_mut(), origins)?;
            Some(PendingMap {
                mapped,
                start,
                input,
            })
        })
        .collect()
}

fn attach_module_maps<T: AsRef<str>>(
    bundle: &mut BundleOutput,
    pending: PendingMaps,
    maps: &[Option<OwnedSourceMap>],
    codes: &[T],
) {
    if pending.iter().all(Option::is_none) {
        return;
    }
    let through: Vec<Option<Through<'_>>> = maps
        .iter()
        .zip(codes)
        .map(|(map, code)| Some(Through::new(map.as_ref()?, code.as_ref())))
        .collect();
    for (module, pending) in bundle.modules.iter_mut().zip(pending) {
        let Some(pending) = pending else {
            continue;
        };
        let input = pending
            .input
            .and_then(|index| through.get(index))
            .and_then(Option::as_ref);
        module.map = input.map(|input| {
            let mapped = pending.mapped;
            let map = source_map::retarget(mapped.map, &mapped.printed, &module.code);
            input.rebased(map, pending.start)
        });
    }
}

fn refuse_final_texts(notes: &mut Vec<Note>, refusal: FinalTextRefusal) {
    let message = match refusal {
        FinalTextRefusal::Unparsable => UNPACK_TEXT_UNPARSABLE,
        FinalTextRefusal::TooDeep => UNPACK_TEXT_TOO_DEEP,
    };
    notes.push(Note {
        pass: "pipeline",
        kind: NoteKind::Refusal,
        message: message.to_string(),
        span: None,
    });
}

fn unpack_nothing<T: AsRef<str>>(
    codes: &[T],
    guard: FinalTextGuard,
    stages: &mut Vec<StageReport>,
    notes: &mut Vec<Note>,
) -> Option<(BundleOutput, PendingMaps, Vec<PendingReaders>)> {
    if !codes.iter().all(|code| guard_admits(guard, code.as_ref())) {
        refuse_final_texts(notes, FinalTextRefusal::TooDeep);
        return None;
    }
    push_unpack_row(stages, Instant::now(), 0);
    None
}

fn push_unpack_row(stages: &mut Vec<StageReport>, started: Instant, modules: u32) {
    let elapsed = started.elapsed();
    stages.push(StageReport {
        stage: "unpack",
        round: round_of(stages, "unpack"),
        iterations: 1,
        changes: modules,
        per_pass: vec![("unpack", modules)],
        per_pass_elapsed: vec![("unpack", elapsed)],
        fixpoint_per_pass: Vec::new(),
        fixpoint_per_pass_elapsed: Vec::new(),
        iteration_elapsed: Vec::new(),
        elapsed,
        stop: StopReason::Converged,
        #[cfg(feature = "test-hooks")]
        work: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::super::notes::UNPACK_TEXT_UNPARSABLE;
    use super::super::{Options, Pipeline};
    use super::{FinalTextGuard, Prior};
    use untwist_core::ast_util::respan::reset_synthetic_spans;
    use untwist_core::reflow::{reflow, GuardRecord};

    const WEBPACK_4_ANONYMOUS: &str = "!function(e){var a={};function n(t){if(a[t])return a[t].exports;var r=a[t]={exports:{}};return e[t].call(r.exports,r,r.exports,n),r.exports}return n(n.s=0)}([function(e,t,i){const lib=i(1);console.log(lib)},function(e,t,i){e.exports=function(){return [1,,2]}}]);";

    #[test]
    fn the_unpack_arena_scope_hands_back_modules_their_reflow_has_not_touched() {
        let pipeline = Pipeline::new(Options::default());
        let codes = Pipeline::new(Options {
            unpack: false,
            ..Options::default()
        })
        .run(WEBPACK_4_ANONYMOUS)
        .codes;
        reset_synthetic_spans();
        let guard = (FinalTextGuard::PremiseP1, false);
        let unreflowed = pipeline.unpack_in_arena(&codes, guard, &mut Vec::new(), &mut Vec::new());
        let finished = pipeline.unpack_final_texts(
            &codes,
            Prior::guards(&GuardRecord::default()),
            guard,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut false,
        );
        let (Some((unreflowed, _, _)), Some(finished)) = (unreflowed, finished) else {
            panic!("the bundle did not unpack");
        };
        assert_eq!(unreflowed.modules.len(), finished.modules.len());
        assert!(unreflowed
            .modules
            .iter()
            .zip(&finished.modules)
            .any(|(before, after)| before.code != after.code));
        for (before, after) in unreflowed.modules.into_iter().zip(&finished.modules) {
            assert_eq!(reflow(before.code), after.code);
        }
    }

    #[test]
    fn a_final_text_that_fails_to_reparse_refuses_the_bundle() {
        let pipeline = Pipeline::new(Options::default());
        let mut stages = Vec::new();
        let mut notes = Vec::new();
        let mut panicked = false;
        let bundle = pipeline.unpack_final_texts(
            &["(".to_string()],
            Prior::guards(&GuardRecord::default()),
            (FinalTextGuard::PremiseP1, false),
            &mut stages,
            &mut notes,
            &mut panicked,
        );
        assert!(bundle.is_none());
        assert!(
            !panicked,
            "an unparsable final text is a refusal, not a panic"
        );
        assert!(notes
            .iter()
            .any(|note| note.pass == "pipeline" && note.message == UNPACK_TEXT_UNPARSABLE));
    }
}
