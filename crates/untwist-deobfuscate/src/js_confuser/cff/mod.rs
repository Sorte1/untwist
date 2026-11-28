mod build;
mod entry;
mod install;
mod rewrite;
mod shape;
mod structure;
mod value;
mod walk;

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{build_scoping, has_module_syntax, may_call_eval};

use self::build::{Context, Lifted, Plan};
use self::entry::Finder;
use self::install::Installer;
use self::shape::{helpers, Helpers};
use super::names::names;
use super::refs::{inside, names_observed, scan, shared_elsewhere, unpatched_outside, Refs};
use super::resolution::{holds, Expected, Spans};
use crate::calls::agnostic::{global_reach, ReachWatch};

pub const PASS: &str = "undo-js-confuser-cff";

const HINTS: [&str; 3] = ["2654435769", "0x9e3779b9", "0x9E3779B9"];

static RELIED_ON: Reliance = Reliance {
    roots: &["Array", "String", "Symbol", "Object"],
    members: &[],
};

#[derive(Default)]
pub struct UndoCff {
    settled: bool,
    inert: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UndoCff::default())
}

struct Analysis {
    spans: Spans,
    plans: Vec<Plan>,
    helpers: Option<Helpers>,
    remove_helpers: bool,
    notes: Vec<String>,
    skip: Vec<SymbolId>,
    used: FxHashSet<String>,
}

fn used_names(scoping: &Scoping, refs: &Refs<'_>) -> FxHashSet<String> {
    scoping
        .symbol_names()
        .map(str::to_string)
        .chain(
            refs.all
                .iter()
                .filter(|(r, _)| r.symbol.is_none())
                .map(|(_, n)| n.to_string()),
        )
        .collect()
}

fn sum_shaped(statement: &Statement<'_>) -> bool {
    let Statement::FunctionDeclaration(function) = statement else {
        return false;
    };
    function.params.items.len() == 1
        && function.body.as_ref().is_some_and(|body| {
            matches!(
                body.statements.as_slice(),
                [Statement::ForStatement(_), Statement::ReturnStatement(_)]
            )
        })
}

fn analyse(program: &Program<'_>, scoping: &Scoping, nested_root: bool) -> Analysis {
    let mut analysis = Analysis {
        spans: Spans::default(),
        plans: Vec::new(),
        helpers: None,
        remove_helpers: false,
        notes: Vec::new(),
        skip: Vec::new(),
        used: FxHashSet::default(),
    };
    if has_module_syntax(program)
        || !HINTS.iter().any(|h| program.source_text.contains(h))
        || !program.body.iter().any(sum_shaped)
    {
        return analysis;
    }
    let refs = scan(program, scoping);
    let names = names(program, scoping);
    let Some(found) = helpers(program, (&refs, scoping, &names)) else {
        return analysis;
    };
    let note = |reason: &str| format!("js-confuser control flow flattening kept: {reason}");
    if refs.has_with || may_call_eval(scoping) {
        analysis
            .notes
            .push(note("the program has a with statement or may call eval"));
        return analysis;
    }
    let spans: Spans = refs
        .all
        .iter()
        .filter_map(|(r, _)| Some((r.span, r.symbol?)))
        .collect();
    let mut finder = Finder {
        scoping,
        spans: &spans,
        refs: &refs,
        helpers: &found,
        plans: Vec::new(),
        mains: Vec::new(),
        notes: Vec::new(),
        extras: Vec::new(),
    };
    finder.visit_program(program);
    let mains = finder.mains.clone();
    let plans: Vec<Plan> = finder
        .plans
        .into_iter()
        .filter(|plan| {
            !mains
                .iter()
                .any(|m| *m != plan.main_statement && inside(plan.main_statement, *m))
        })
        .collect();
    analysis.notes.extend(finder.notes.iter().map(|r| note(r)));
    let regions: Vec<Span> = plans
        .iter()
        .flat_map(|p| std::iter::once(p.main_statement).chain(p.entry.iter().copied()))
        .collect();
    let all_done = plans.len() == mains.len();
    let helper_refs_inside = found.symbols.iter().all(|symbol| {
        refs.of(*symbol).iter().all(|r| {
            regions.iter().any(|region| inside(*region, r.span))
                || found.statements.iter().any(|s| inside(*s, r.span))
        })
    });
    let top_mains: Vec<SymbolId> = plans
        .iter()
        .filter(|p| matches!(p.context, Context::Program))
        .map(|p| p.syms.main)
        .collect();
    analysis.remove_helpers = all_done
        && helper_refs_inside
        && !shared_elsewhere(program, scoping, nested_root, &found.symbols);
    if shared_elsewhere(program, scoping, nested_root, &top_mains) {
        analysis
            .notes
            .push(note("another input names its main function"));
        return analysis;
    }
    analysis.skip = plans
        .iter()
        .flat_map(|p| {
            [p.syms.main, p.syms.states, p.syms.scope, p.syms.runtime]
                .into_iter()
                .chain(p.syms.arg)
        })
        .chain(found.symbols.iter().copied())
        .chain(finder.extras)
        .collect();
    analysis.used = used_names(scoping, &refs);
    analysis.plans = plans;
    analysis.spans = spans;
    analysis.helpers = Some(found);
    analysis
}

fn removed_names(scoping: &Scoping, analysis: &Analysis) -> Vec<String> {
    let mut names: Vec<String> = analysis
        .plans
        .iter()
        .filter(|p| matches!(p.context, Context::Program))
        .map(|p| scoping.symbol_name(p.syms.main).to_string())
        .collect();
    if analysis.remove_helpers {
        if let Some(helpers) = &analysis.helpers {
            names.extend(
                helpers
                    .symbols
                    .iter()
                    .map(|s| scoping.symbol_name(*s).to_string()),
            );
        }
    }
    names
}

fn reached(trial: &Program<'_>, scoping: &Scoping, names: &[String]) -> bool {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &[],
        observe: true,
    };
    let reach = global_reach(trial, scoping, (None, &[]), watch);
    reach.reach.is_some() || reach.reach_by_code.is_some() || names_observed(&reach)
}

fn unresolved_leftovers(scoping: &Scoping, names: &[String]) -> bool {
    let unresolved = scoping.root_unresolved_references();
    names
        .iter()
        .any(|name| unresolved.contains_key(name.as_str()))
}

impl<'a> Pass<'a> for UndoCff {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn reset(&mut self) {
        self.settled = self.inert;
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.settled {
            return;
        }
        self.settled = true;
        let analysis = analyse(program, cx.scoping(), cx.opts.nested_root);
        for note in &analysis.notes {
            cx.note(note.clone(), None);
        }
        let Some(helpers) = &analysis.helpers else {
            self.inert = true;
            return;
        };
        if analysis.plans.is_empty() {
            return;
        }
        let names_observed = !cx.opts.unsafe_transforms;
        let mut trial = program.clone_in(cx.allocator());
        let (expected, lifted) = {
            let mut installer = Installer {
                builder: cx.ast(),
                alloc: cx.allocator(),
                scoping: cx.scoping(),
                spans: &analysis.spans,
                plans: &analysis.plans,
                strings: &helpers.strings,
                helper_statements: &helpers.statements,
                remove_helpers: analysis.remove_helpers,
                lifted: Lifted::with_used(analysis.used.clone()),
                expected: Expected::default(),
                skip: &analysis.skip,
                names_observed,
                failed: false,
                applied: 0,
                plain: None,
            };
            installer.visit_program(&mut trial);
            if installer.failed || installer.applied != analysis.plans.len() {
                cx.note("js-confuser control flow flattening kept: the recovered code could not be put in place", None);
                return;
            }
            (installer.expected, installer.lifted)
        };
        let mut expected = expected;
        for (reference, declaration) in &lifted.created {
            expected.insert(*reference, Some(*declaration));
        }
        let restored = build_scoping(&trial).scoping;
        if !holds(&trial, &restored, &expected) {
            cx.note("js-confuser control flow flattening kept: a name in the recovered code would bind differently", None);
            return;
        }
        let removed = removed_names(cx.scoping(), &analysis);
        if unresolved_leftovers(&restored, &removed) {
            cx.note("js-confuser control flow flattening kept: code outside the machine still names its helpers", None);
            return;
        }
        let wrappers: FxHashSet<Span> = analysis
            .plans
            .iter()
            .flat_map(|p| p.wrappers.keys().copied())
            .collect();
        let mut variant = trial.clone_in(cx.allocator());
        let mut plain = Installer {
            builder: cx.ast(),
            alloc: cx.allocator(),
            scoping: cx.scoping(),
            spans: &analysis.spans,
            plans: &analysis.plans,
            strings: &helpers.strings,
            helper_statements: &helpers.statements,
            remove_helpers: false,
            lifted: Lifted::default(),
            expected: Expected::default(),
            skip: &analysis.skip,
            names_observed,
            failed: false,
            applied: 0,
            plain: Some(&wrappers),
        };
        plain.visit_program(&mut variant);
        let variant_scoping = build_scoping(&variant).scoping;
        let intact = |p: &Program<'_>, sc: &Scoping| unpatched_outside(p, sc, &RELIED_ON, &[]);
        if !cx.opts.unsafe_transforms
            && !intact(&trial, &restored)
            && !intact(&variant, &variant_scoping)
        {
            cx.note("js-confuser control flow flattening kept: the program may replace a standard built-in the machine uses (Array, String, Symbol, Object)", None);
            return;
        }
        if !removed.is_empty()
            && reached(&trial, &restored, &removed)
            && reached(&variant, &variant_scoping, &removed)
        {
            cx.note("js-confuser control flow flattening kept: code may reach or observe its helpers through the global object or code text", None);
            return;
        }
        let applied = analysis.plans.len();
        *program = trial;
        *cx.scoping_mut() = restored;
        cx.binding_moved();
        cx.changed_by(applied);
        cx.note(
            format!("{applied} js-confuser control flow flattening machine(s) undone"),
            None,
        );
    }
}
