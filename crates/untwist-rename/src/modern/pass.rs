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
use super::pure::looks_pure;
use super::report;
use super::rewrite::{self, Site};
use super::rewrite_apply;
use crate::library::match_program;
use crate::text_reads::{text_read_bindings_with, SelfReads, TextReads};
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashMap;
use untwist_core::semantic::build_scoping;

pub const NAME: &str = "modern";

const TEXT_READ_UNKNOWN: &str =
    "code may read function text through a computed key, so no core-js module was removed";
const DYNAMIC_SCOPE: &str =
    "the program uses direct eval or with, so no core-js module was removed";
const PURE: &str =
    "the core-js code reads a path object of its own (core-js-pure), so no core-js module was removed or rewritten";
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
    rewrites: FxHashMap<&'static str, usize>,
    kept: Option<Vec<Installer>>,
    rewritten: bool,
    last: Option<Plan>,
    library: Vec<Span>,
}

impl ModernPass {
    fn planned(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        cx: &mut Cx<'_, '_>,
        library: &[Span],
    ) -> Option<(Plan, FxHashMap<Span, Site>)> {
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
        if looks_pure(&units, &facts) {
            refuse(cx, PURE);
            return None;
        }
        let inputs = Inputs {
            facts: &facts,
            scoping,
            compat: compat::builtin(),
            baseline: &self.baseline,
            pinned: &pinned,
            library,
        };
        let plan = plan(&units, &inputs);
        let rewrites = rewrite::Inputs {
            facts: &facts,
            scoping,
            helpers: &plan.helpers,
            compat: compat::builtin(),
            baseline: &self.baseline,
            pinned: &pinned,
        };
        let sites = rewrite::plan(program, &rewrites);
        Some((plan, sites))
    }

    fn round<'a>(
        &self,
        program: &mut Program<'a>,
        scoping: &Scoping,
        rounds: &mut Rounds,
        cx: &mut Cx<'a, '_>,
        rewrite: bool,
    ) -> bool {
        let alloc = cx.allocator();
        let Some((mut plan, sites)) = self.planned(program, scoping, cx, &rounds.library) else {
            return false;
        };
        let removing = !plan.regions.is_empty();
        if removing && reachable(program, &plan.regions) != plan.regions.len() {
            refuse(cx, EDIT_MISMATCH);
            rounds.last = Some(plan);
            return false;
        }
        if removing {
            rounds.applied += apply(program, &plan.regions, alloc);
            rounds.removed_helpers += plan.removed_helpers;
            let (removed, kept) = plan
                .installers
                .drain(..)
                .partition(|installer| installer.kept.is_none());
            rounds.removed.extend::<Vec<Installer>>(removed);
            plan.installers = kept;
        }
        let rewritten = match rewrite {
            true => rewrite_apply::apply(program, &sites, alloc, &*cx),
            false => Vec::new(),
        };
        rounds.applied += rewritten.len();
        for kind in &rewritten {
            *rounds.rewrites.entry(kind).or_default() += 1;
        }
        if rounds.kept.is_none() || !rounds.rewritten {
            rounds.kept = Some(plan.installers.clone());
        }
        rounds.rewritten |= rewrite;
        rounds.library.extend(plan.library.iter().copied());
        rounds.last = Some(plan);
        removing || !rewritten.is_empty()
    }

    fn notes(&self, rounds: &Rounds, cx: &mut Cx<'_, '_>) {
        let Some(last) = &rounds.last else {
            return;
        };
        let kept: Vec<&Installer> = rounds
            .kept
            .iter()
            .flatten()
            .filter(|installer| {
                !rounds
                    .removed
                    .iter()
                    .any(|removed| removed.span == installer.span)
            })
            .collect();
        let removed: Vec<&Installer> = rounds.removed.iter().collect();
        cx.note(
            report::summary(&removed, &kept, rounds.removed_helpers, last.kept_helpers),
            None,
        );
        for installer in removed.iter().chain(&kept) {
            cx.note(report::installer_note(installer), None);
        }
        let mut rewrites: Vec<(&&str, &usize)> = rounds.rewrites.iter().collect();
        rewrites.sort();
        for (kind, count) in rewrites {
            cx.note(
                format!("rewrote {count} core-js helper calls: {kind}"),
                None,
            );
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
        for phase in [false, true, false] {
            for _ in 0..MAX_ROUNDS {
                let sem = build_scoping(program);
                if !self.round(program, &sem.scoping, &mut rounds, cx, phase) || phase {
                    break;
                }
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
