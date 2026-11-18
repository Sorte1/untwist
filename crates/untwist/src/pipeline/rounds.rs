use oxc_ast::ast::Program;
use untwist_core::parse::{has_error, parse_with};
use untwist_core::pass::{Note, NoteKind, RoundCapUndo};
use untwist_core::print::pretty_codegen;
use untwist_core::semantic::{build_scoping, forget_failures, Semantics};

use super::input::Staged;
use super::layer::Layering;
use super::stages::{Shaping, StageCx};
use super::{Pipeline, Reservation};
use crate::plugin::Stage;
use crate::report::StageReport;

const LAYER_ROUNDS: u32 = 6;

const MACHINERY_STAGES: [&str; 2] = ["deobfuscate", "protections"];

const MACHINERY_PASSES: [&str; 8] = [
    "find-string-array",
    "find-array-rotator",
    "find-decoders",
    "inline-decoder-wrappers",
    "inline-decoded-strings",
    "remove-decoder-machinery",
    "self-defending",
    "debug-protection",
];

const REMOVING_PASSES: [&str; 5] = [
    "inline-decoder-wrappers",
    "inline-decoded-strings",
    "remove-decoder-machinery",
    "self-defending",
    "debug-protection",
];

const LAYER_PASSES: [&str; 3] = [
    "remove-decoder-machinery",
    "self-defending",
    "debug-protection",
];

#[derive(Clone, Copy)]
struct Round {
    notes: usize,
    stages: usize,
}

impl Round {
    fn opened(site: &StageCx<'_, '_>) -> Self {
        Self {
            notes: site.notes.len(),
            stages: site.stages.len(),
        }
    }
}

struct Quiet {
    _decoded: Option<untwist_deobfuscate::decoded_strings::Recorder>,
    #[cfg(feature = "refusal-spans")]
    _refusals: Option<untwist_deobfuscate::refusal_spans::Recorder>,
}

enum Next {
    Settled(Semantics),
    Advanced(Semantics, Shaping, Round, Option<Quiet>),
    Exit(Staged),
}

impl Pipeline {
    pub(super) fn layer_rounds<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        (sem, reservation): (Semantics, Reservation<'_>),
        layering: &mut Layering,
    ) -> Staged {
        let base = site.notes.len();
        let mut round = Round::opened(site);
        let front = self.front_stages(site, sem);
        if let Some(exit) = self.exits(site, &front, layering) {
            return exit;
        }
        let mut shaping = Shaping::default();
        let Some(mut sem) = self.machinery_stages(site, front, reservation, &mut shaping) else {
            return Staged::Done(false);
        };
        let mut carried: Vec<Note> = Vec::new();
        let mut quiet = None;
        for _ in 1..LAYER_ROUNDS {
            if !worth_another(site, round) || untwist_core::budget::check().is_err() {
                break;
            }
            let quieted = quiet.is_some();
            match self.next_round(site, (sem, reservation), (layering, quieted)) {
                Next::Exit(staged) => return staged,
                Next::Settled(settled) => {
                    sem = settled;
                    break;
                }
                Next::Advanced(advanced, shaped, opened, held) => {
                    carried.extend(superseded(site.notes, round.notes, opened.notes));
                    round = Round {
                        notes: round.notes,
                        ..opened
                    };
                    sem = advanced;
                    shaping = shaped;
                    quiet = quiet.or(held);
                }
            }
        }
        self.closing_stages(site, sem, reservation, shaping);
        let at = base.min(site.notes.len());
        site.notes.splice(at..at, carried);
        Staged::Done(quiet.is_some())
    }

    fn exits(
        &self,
        site: &mut StageCx<'_, '_>,
        sem: &Semantics,
        layering: &mut Layering,
    ) -> Option<Staged> {
        if site.pass_opts.round_cap_undo.get() == RoundCapUndo::Requested {
            return Some(Staged::Replay);
        }
        self.peel(site, sem, layering).map(Staged::Peeled)
    }

    fn next_round<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        (sem, reservation): (Semantics, Reservation<'_>),
        (layering, quieted): (&mut Layering, bool),
    ) -> Next {
        let printed = pretty_codegen(site.program);
        let text: &'a str = site.alloc.alloc_str(&printed);
        let parsed = parse_with(site.alloc, text, site.program.source_type);
        if has_error(&parsed.diagnostics) {
            return Next::Settled(sem);
        }
        let quiet = (!quieted).then(|| self.quiet());
        let held = std::mem::replace(site.program, parsed.program);
        let opened = Round::opened(site);
        let (front, decoded) = self.probe_stages(site, (fresh_round(site.program), &printed));
        if let Some(exit) = self.exits(site, &front, layering) {
            return Next::Exit(exit);
        }
        if !decoded {
            *site.program = held;
            site.notes.truncate(opened.notes);
            untwist_deobfuscate::reset_round_state();
            forget_failures();
            return Next::Settled(sem);
        }
        drop(sem);
        let mut shaping = Shaping::default();
        match self.machinery_stages(site, front, reservation, &mut shaping) {
            Some(shaped) => Next::Advanced(shaped, shaping, opened, quiet),
            None => Next::Exit(Staged::Done(true)),
        }
    }

    fn probe_stages<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        (sem, printed): (Semantics, &str),
    ) -> (Semantics, bool) {
        let opened = site.stages.len();
        let (sem, prepared) = self.prepare_stage(site, sem);
        let sem = self.plugin_stage(Stage::AfterPrepare, site, sem);
        let before = match changes(&site.stages[opened..]) {
            0 => None,
            _ => Some(pretty_codegen(site.program)),
        };
        let opened = site.stages.len();
        let sem = self.deobfuscate_stage(site, (sem, prepared));
        if site.pass_opts.round_cap_undo.get() == RoundCapUndo::Requested {
            return (sem, true);
        }
        if removals(&site.stages[opened..], &REMOVING_PASSES) == 0 {
            return (self.plugin_stage(Stage::AfterDeobfuscate, site, sem), false);
        }
        let (sem, _) = self.final_machinery_stage(site, sem);
        let after = pretty_codegen(site.program);
        let decoded = after != before.as_deref().unwrap_or(printed);
        (
            self.plugin_stage(Stage::AfterDeobfuscate, site, sem),
            decoded,
        )
    }

    fn quiet(&self) -> Quiet {
        Quiet {
            _decoded: self
                .options
                .decoded_strings
                .then(untwist_deobfuscate::decoded_strings::Recorder::start),
            #[cfg(feature = "refusal-spans")]
            _refusals: self
                .records_refusal_spans()
                .then(untwist_deobfuscate::refusal_spans::Recorder::start),
        }
    }
}

fn changes(stages: &[StageReport]) -> u32 {
    stages
        .iter()
        .map(|stage| stage.changes)
        .fold(0, u32::saturating_add)
}

fn removals(stages: &[StageReport], passes: &[&str]) -> u32 {
    stages
        .iter()
        .filter(|stage| MACHINERY_STAGES.contains(&stage.stage))
        .flat_map(|stage| stage.per_pass.iter())
        .filter(|(pass, _)| passes.contains(pass))
        .map(|(_, count)| *count)
        .fold(0, u32::saturating_add)
}

fn worth_another(site: &StageCx<'_, '_>, round: Round) -> bool {
    removals(&site.stages[round.stages..], &LAYER_PASSES) > 0
        && site.notes[round.notes..]
            .iter()
            .any(|note| note.kind == NoteKind::Refusal && MACHINERY_PASSES.contains(&note.pass))
}

fn superseded(notes: &mut Vec<Note>, from: usize, to: usize) -> Vec<Note> {
    notes
        .drain(from..to)
        .filter(|note| note.kind == NoteKind::Progress)
        .collect()
}

fn fresh_round(program: &Program<'_>) -> Semantics {
    untwist_deobfuscate::reset_round_state();
    forget_failures();
    build_scoping(program)
}
