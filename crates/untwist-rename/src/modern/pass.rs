use oxc_allocator::{Allocator, GetAllocator};
use oxc_ast::ast::{CommentNewlines, Program};
use oxc_ast::{Comment, CommentKind, CommentPosition};
use oxc_span::GetSpan;
use untwist_core::pass::{Cx, Interest, NoteKind, Pass};

use super::baseline::Baseline;
use super::collect::collect;
use super::compat;
use super::edit::{apply, reachable};
use super::lookalike::lookalikes;
use super::plan::{module_hits, plan, units, Inputs, Installer, Plan, LABEL};
use super::report;
use crate::library::match_program;
use crate::text_reads::{text_read_bindings_with, SelfReads, TextReads};
use oxc_semantic::Scoping;
use untwist_core::semantic::build_scoping;

pub const NAME: &str = "modern";

const TEXT_READ_UNKNOWN: &str =
    "code may read function text through a computed key, so no core-js module was removed";
const DYNAMIC_SCOPE: &str =
    "the program uses direct eval or with, so no core-js module was removed";
const EDIT_MISMATCH: &str =
    "some planned removals are not in a statement list, so no core-js module was removed";

pub struct ModernPass {
    baseline: Baseline,
    done: bool,
}

pub fn boxed<'a>(baseline: Baseline) -> Box<dyn Pass<'a> + 'a> {
    Box::new(ModernPass {
        baseline,
        done: false,
    })
}

fn refuse(cx: &mut Cx<'_, '_>, message: &str) {
    cx.note_kind(NoteKind::Refusal, message, None);
}

fn add_banner<'a>(program: &mut Program<'a>, text: &str, alloc: &'a Allocator) {
    let Some(first) = program.body.first() else {
        return;
    };
    let start = first.span().start;
    let mut source = String::from(program.source_text);
    source.push('\n');
    let begin = u32::try_from(source.len()).unwrap_or(u32::MAX);
    source.push_str(text);
    let end = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let mut comment = Comment::new(begin, end, CommentKind::Line);
    comment.attached_to = start;
    comment.position = CommentPosition::Leading;
    comment.newlines = CommentNewlines::Leading | CommentNewlines::Trailing;
    program.source_text = alloc.alloc_str(&source);
    program.comments.push(comment);
}

const MAX_ROUNDS: usize = 8;

#[derive(Default)]
struct Rounds {
    removed: Vec<Installer>,
    removed_helpers: usize,
    applied: usize,
    last: Option<Plan>,
}

impl ModernPass {
    fn planned(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        cx: &mut Cx<'_, '_>,
    ) -> Option<Plan> {
        let found = match_program(program, scoping);
        if !found.hits.iter().any(|hit| hit.label == LABEL) {
            cx.note("no core-js module was recognised", None);
            return None;
        }
        let facts = collect(program, scoping);
        if facts.dynamic {
            refuse(cx, DYNAMIC_SCOPE);
            return None;
        }
        let self_reads = match cx.opts.unsafe_transforms {
            true => SelfReads::InstanceMethodsReadInstances,
            false => SelfReads::Unknown,
        };
        let TextReads::Bindings(pinned) = text_read_bindings_with(program, scoping, self_reads)
        else {
            refuse(cx, TEXT_READ_UNKNOWN);
            return None;
        };
        let hits = module_hits(&found.hits);
        let known = hits.iter().map(|hit| hit.span).collect();
        let lookalikes = lookalikes(program, scoping, &known);
        let units = units(&hits, lookalikes, &facts, scoping);
        let inputs = Inputs {
            facts: &facts,
            scoping,
            compat: compat::builtin(),
            baseline: &self.baseline,
            pinned: &pinned,
        };
        Some(plan(&units, &inputs))
    }

    fn round<'a>(
        &self,
        program: &mut Program<'a>,
        scoping: &Scoping,
        rounds: &mut Rounds,
        cx: &mut Cx<'a, '_>,
    ) -> bool {
        let alloc = cx.allocator();
        let Some(mut plan) = self.planned(program, scoping, cx) else {
            return false;
        };
        if plan.regions.is_empty() {
            rounds.last = Some(plan);
            return false;
        }
        if reachable(program, &plan.regions) != plan.regions.len() {
            refuse(cx, EDIT_MISMATCH);
            rounds.last = Some(plan);
            return false;
        }
        rounds.applied += apply(program, &plan.regions, alloc);
        rounds.removed_helpers += plan.removed_helpers;
        let (removed, kept) = plan
            .installers
            .drain(..)
            .partition(|installer| installer.kept.is_none());
        rounds.removed.extend::<Vec<Installer>>(removed);
        plan.installers = kept;
        rounds.last = Some(plan);
        true
    }

    fn notes(&self, rounds: &Rounds, cx: &mut Cx<'_, '_>) {
        let Some(last) = &rounds.last else {
            return;
        };
        let kept: Vec<&Installer> = last.installers.iter().collect();
        let removed: Vec<&Installer> = rounds.removed.iter().collect();
        cx.note(
            report::summary(&removed, &kept, rounds.removed_helpers, last.kept_helpers),
            None,
        );
        for installer in removed.iter().chain(&kept) {
            cx.note(report::installer_note(installer), None);
        }
    }
}

impl<'a> Pass<'a> for ModernPass {
    fn name(&self) -> &'static str {
        NAME
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        cx.note(report::assumption(&self.baseline), None);
        let mut rounds = Rounds::default();
        for _ in 0..MAX_ROUNDS {
            let sem = build_scoping(program);
            if !self.round(program, &sem.scoping, &mut rounds, cx) {
                break;
            }
        }
        self.notes(&rounds, cx);
        if rounds.applied > 0 {
            let text = report::banner(rounds.removed.len(), &self.baseline);
            add_banner(program, &text, cx.allocator());
            cx.changed_by(rounds.applied);
        }
    }
}
