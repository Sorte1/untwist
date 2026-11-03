use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::ast_util::respan::reset_synthetic_spans;
use untwist_core::clock::StageInstant as Instant;
use untwist_core::pass::{Note, NoteKind, StopReason};
use untwist_core::reflow::GuardRecord;

use super::boundary::Boundary;
use super::input::{final_trees, guard_admits, FinalTextGuard, FinalTextRefusal};
use super::notes::{guard_notes, UNPACK_PANICKED, UNPACK_TEXT_UNPARSABLE};
use super::refusal::UNPACK_TEXT_TOO_DEEP;
use super::stages::round_of;
use super::Pipeline;
use crate::bundle::BundleOutput;
use crate::report::StageReport;

impl Pipeline {
    pub(super) fn unpack_options(&self) -> untwist_unpack::UnpackOptions {
        untwist_unpack::UnpackOptions {
            split_hoisted: self.options.split_hoisted,
            conservative: !self.options.extended(),
        }
    }

    pub(super) fn unpack_final_texts<T: AsRef<str>>(
        &self,
        codes: &[T],
        earlier: &GuardRecord,
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
            let (bundle, tally) = self
                .unpack_in_arena(codes, guard, stages, notes)?
                .reflowed(earlier);
            notes.extend(guard_notes(tally));
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
    ) -> Option<BundleOutput> {
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
        let mut bundle = self.unpack_stage(&alloc, &mut programs, stages, notes)?;
        self.after_unpack(&alloc, &mut bundle, stages, notes);
        #[cfg(feature = "module-spans")]
        let unreflowed = BundleOutput::unreflowed(&bundle, crate::bundle::single_text_len(codes));
        #[cfg(not(feature = "module-spans"))]
        let unreflowed = BundleOutput::unreflowed(&bundle);
        Some(unreflowed)
    }

    fn unpack_stage<'a>(
        &self,
        alloc: &'a Allocator,
        programs: &mut [Program<'a>],
        stages: &mut Vec<StageReport>,
        notes: &mut Vec<Note>,
    ) -> Option<untwist_unpack::Bundle<'a>> {
        let started = self.begin("unpack");
        let (bundle, unpack_notes) = untwist_unpack::unpack_inputs(
            alloc,
            programs,
            &self.options.mappings,
            self.unpack_options(),
        );
        notes.extend(unpack_notes);
        let modules = bundle.as_ref().map_or(0, |bundle| bundle.modules.len()) as u32;
        push_unpack_row(stages, started, modules);
        bundle
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
) -> Option<BundleOutput> {
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
    });
}

#[cfg(test)]
mod tests {
    use super::super::notes::UNPACK_TEXT_UNPARSABLE;
    use super::super::{Options, Pipeline};
    use super::FinalTextGuard;
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
            &GuardRecord::default(),
            guard,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut false,
        );
        let (Some(unreflowed), Some(finished)) = (unreflowed, finished) else {
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
            &GuardRecord::default(),
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
