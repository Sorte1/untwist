use std::collections::HashSet;

use oxc_ast::ast::{Expression, Program, Statement, SwitchStatement};
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, Visit, VisitJs};
use oxc_span::{GetSpan, Span};
use untwist_core::parse::{has_error, parse_with};
use untwist_core::pass::{Note, NoteKind, RoundCapUndo};
use untwist_core::print::pretty_codegen;
use untwist_core::semantic::{build_scoping, forget_failures, Semantics};
use untwist_core::source_map::origins::{self, Origins};
use untwist_deobfuscate::notes::ROUND_CAP;

use super::input::Staged;
use super::layer::Layering;
use super::stages::{Shaping, StageCx};
use super::trail::Taps;
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

const UNBLOCKED_PASSES: [&str; 2] = ["control-flow-object", "dead-code"];

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

struct Probe<'a> {
    held: Program<'a>,
    opened: Round,
    taps: Taps,
    earlier: Option<Origins>,
}

enum Next {
    Settled(Semantics),
    Advanced(Semantics, Shaping, Round, Vec<Note>),
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
            return Staged::Done;
        };
        let mut carried: Vec<Note> = Vec::new();
        let mut taps = Taps::now();
        for _ in 1..LAYER_ROUNDS {
            let due = worth_another(site, round) || cleanup_due(site, round);
            if !due || untwist_core::budget::check().is_err() {
                break;
            }
            taps.settle(site.trail);
            taps = Taps::now();
            match self.next_round(site, (sem, reservation), (layering, round)) {
                Next::Exit(staged) => {
                    taps.rewind();
                    return staged;
                }
                Next::Settled(settled) => {
                    sem = settled;
                    break;
                }
                Next::Advanced(advanced, shaped, opened, kept) => {
                    carried.extend(kept);
                    round = opened;
                    sem = advanced;
                    shaping = shaped;
                }
            }
        }
        self.closing_stages(site, sem, reservation, shaping);
        taps.settle(site.trail);
        let at = base.min(site.notes.len());
        site.notes.splice(at..at, carried);
        Staged::Done
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
        (layering, round): (&mut Layering, Round),
    ) -> Next {
        let printed = pretty_codegen(site.program);
        let text: &'a str = site.alloc.alloc_str(&printed);
        let parsed = parse_with(site.alloc, text, site.program.source_type);
        if has_error(&parsed.diagnostics) {
            return Next::Settled(sem);
        }
        let taps = Taps::now();
        let earlier = origins::restart();
        let held = std::mem::replace(site.program, parsed.program);
        let probe = Probe {
            held,
            opened: Round::opened(site),
            taps,
            earlier,
        };
        let (front, decoded) = self.probe_stages(site, (fresh_round(site.program), &printed));
        if site.pass_opts.round_cap_undo.get() == RoundCapUndo::Requested {
            site.pass_opts.round_cap_undo.set(RoundCapUndo::Replay);
            return Next::Settled(settled_back(site, probe, sem, true));
        }
        if let Some(peeled) = self.peel(site, &front, layering) {
            return Next::Exit(Staged::Peeled(peeled));
        }
        if !decoded {
            return Next::Settled(settled_back(site, probe, sem, false));
        }
        drop(sem);
        self.advance(site, (front, reservation), (probe, round, text))
    }

    fn advance<'a>(
        &self,
        site: &mut StageCx<'a, '_>,
        (front, reservation): (Semantics, Reservation<'_>),
        (probe, round, text): (Probe<'a>, Round, &'a str),
    ) -> Next {
        let mut shaping = Shaping::default();
        let Some(shaped) = self.machinery_stages(site, front, reservation, &mut shaping) else {
            return Next::Exit(Staged::Done);
        };
        let Probe {
            mut held,
            opened,
            earlier,
            ..
        } = probe;
        let kept = superseded(site, (round, opened), &held);
        if self.traces_spans() {
            site.trail.reprinted(&mut held, earlier, text);
        }
        let next = Round {
            notes: round.notes,
            stages: opened.stages,
        };
        let drained = opened.notes - round.notes;
        Next::Advanced(shaped, shaping.shifted(drained), next, kept)
    }

    fn traces_spans(&self) -> bool {
        #[cfg(feature = "refusal-spans")]
        let refusals = self.records_refusal_spans();
        #[cfg(not(feature = "refusal-spans"))]
        let refusals = false;
        refusals
            || self.options.source_map
            || self.options.decoded_strings
            || self.options.statement_spans
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
        let stages = &site.stages[opened..];
        if removals(stages, &REMOVING_PASSES) == 0 && settled_rewrites(stages) == 0 {
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
}

fn settled_back<'a>(
    site: &mut StageCx<'a, '_>,
    probe: Probe<'a>,
    sem: Semantics,
    capped: bool,
) -> Semantics {
    *site.program = probe.held;
    let cap_notes: Vec<Note> = site.notes[probe.opened.notes..]
        .iter()
        .filter(|note| capped && note.message.starts_with(ROUND_CAP))
        .cloned()
        .collect();
    site.notes.truncate(probe.opened.notes);
    site.notes.extend(cap_notes);
    probe.taps.rewind();
    origins::resume(probe.earlier);
    untwist_deobfuscate::reset_round_state();
    untwist_deobfuscate::legacy_reach::round_discarded();
    forget_failures();
    sem
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

fn settled_rewrites(stages: &[StageReport]) -> u32 {
    stages
        .iter()
        .filter(|stage| MACHINERY_STAGES.contains(&stage.stage))
        .flat_map(|stage| stage.per_pass.iter().chain(&stage.fixpoint_per_pass))
        .filter(|(pass, _)| UNBLOCKED_PASSES.contains(pass))
        .map(|(_, count)| *count)
        .fold(0, u32::saturating_add)
}

struct EmptiedDispatch {
    found: bool,
}

impl<'a> VisitJs<'a> for EmptiedDispatch {
    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        let dispatched = matches!(
            &it.discriminant,
            Expression::ComputedMemberExpression(member)
                if matches!(member.expression, Expression::UpdateExpression(_))
        );
        let emptied = it.cases.iter().any(|case| {
            matches!(
                case.consequent.as_slice(),
                [Statement::ContinueStatement(next)] if next.label.is_none()
            )
        });
        if dispatched && emptied {
            self.found = true;
            return;
        }
        walk_js::walk_switch_statement(self, it);
    }
}

fn cleanup_due(site: &StageCx<'_, '_>, round: Round) -> bool {
    if removals(&site.stages[round.stages..], &LAYER_PASSES) == 0 {
        return false;
    }
    let mut scan = EmptiedDispatch { found: false };
    scan.visit_program(site.program);
    scan.found
}

fn worth_another(site: &StageCx<'_, '_>, round: Round) -> bool {
    removals(&site.stages[round.stages..], &LAYER_PASSES) > 0
        && site.notes[round.notes..]
            .iter()
            .any(|note| note.kind == NoteKind::Refusal && MACHINERY_PASSES.contains(&note.pass))
}

fn superseded(
    site: &mut StageCx<'_, '_>,
    (round, opened): (Round, Round),
    held: &Program<'_>,
) -> Vec<Note> {
    let judged: HashSet<&'static str> = site.notes[opened.notes..]
        .iter()
        .map(|note| note.pass)
        .chain(
            site.stages[opened.stages..]
                .iter()
                .flat_map(|stage| stage.per_pass.iter().map(|(pass, _)| *pass)),
        )
        .collect();
    let wanted: HashSet<Span> = site.notes[round.notes..opened.notes]
        .iter()
        .filter(|note| note.kind == NoteKind::Refusal)
        .filter_map(|note| note.span)
        .collect();
    let present = surviving(held, wanted);
    site.notes
        .drain(round.notes..opened.notes)
        .filter(|note| match (note.kind, note.span) {
            (NoteKind::Progress, _) => true,
            (NoteKind::Refusal, Some(span)) => !present.contains(&span),
            (NoteKind::Refusal, None) => !judged.contains(note.pass),
        })
        .collect()
}

struct Present {
    wanted: HashSet<Span>,
    found: HashSet<Span>,
}

impl<'a> Visit<'a> for Present {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let span = kind.span();
        if self.wanted.contains(&span) {
            self.found.insert(span);
        }
    }
}

fn surviving(program: &Program<'_>, wanted: HashSet<Span>) -> HashSet<Span> {
    if wanted.is_empty() {
        return wanted;
    }
    let mut present = Present {
        wanted,
        found: HashSet::new(),
    };
    present.visit_program(program);
    present.found
}

fn fresh_round(program: &Program<'_>) -> Semantics {
    untwist_deobfuscate::reset_round_state();
    untwist_deobfuscate::legacy_reach::round_opened();
    forget_failures();
    build_scoping(program)
}
