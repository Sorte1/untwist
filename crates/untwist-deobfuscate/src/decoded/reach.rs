use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::{decoded_strings, UnitRun};
use crate::bridge::DecodePlan;
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::mutants::decoder_name_watch_dropped;
use crate::root_reach::root_alias_names;
use crate::stamp::original_name;
use untwist_core::semantic::is_shared_global;

pub(super) type Reached = (FxHashSet<usize>, Span, Option<&'static str>);

pub(super) struct Watched<'p> {
    units: Vec<usize>,
    names: Vec<&'p str>,
    fns: Vec<SymbolId>,
}

fn watched_units<'p>(
    program: &'p Program<'_>,
    scoping: &Scoping,
    runs: &[UnitRun<'p, '_>],
) -> Watched<'p> {
    let root = scoping.root_scope_id();
    let mut watched = Watched {
        units: Vec::new(),
        names: Vec::new(),
        fns: Vec::new(),
    };
    for run in runs {
        if run.request.calls.is_empty() || run.result.is_err() {
            continue;
        }
        let watched_symbols = if decoder_name_watch_dropped() {
            &run.request.guard.symbols[..run.request.guard.symbols.len().min(1)]
        } else {
            &run.request.guard.symbols[..]
        };
        let mut global = watched_symbols
            .iter()
            .filter(|(symbol, _)| scoping.symbol_scope_id(*symbol) == root)
            .peekable();
        if global.peek().is_some() {
            watched.units.push(run.request.unit);
        }
        let mut targets = FxHashSet::default();
        for (symbol, name) in global {
            watched.names.push(name);
            watched.fns.push(*symbol);
            targets.insert(*symbol);
        }
        watched
            .names
            .extend(root_alias_names(program, scoping, &targets));
    }
    watched
}

pub(super) fn names_shared(
    program: &Program<'_>,
    scoping: &Scoping,
    run: &UnitRun<'_, '_>,
) -> bool {
    let root = scoping.root_scope_id();
    let roots: Vec<(SymbolId, &str)> = run
        .request
        .guard
        .symbols
        .iter()
        .filter(|(symbol, _)| scoping.symbol_scope_id(*symbol) == root)
        .copied()
        .collect();
    let targets: FxHashSet<SymbolId> = roots.iter().map(|(symbol, _)| *symbol).collect();
    roots
        .iter()
        .flat_map(|(symbol, name)| {
            [
                Some(*name),
                original_name(program.source_text, scoping, *symbol),
            ]
        })
        .flatten()
        .chain(root_alias_names(program, scoping, &targets))
        .any(is_shared_global)
}

pub(super) fn watched<'p>(
    program: &'p Program<'_>,
    scoping: &Scoping,
    runs: &[UnitRun<'p, '_>],
) -> Option<Watched<'p>> {
    if untwist_core::semantic::top_level_is_local(program) {
        return None;
    }
    let watched = watched_units(program, scoping, runs);
    (!watched.names.is_empty()).then_some(watched)
}

impl<'p> Watched<'p> {
    pub(super) fn watch(&self) -> ReachWatch<'_> {
        ReachWatch {
            names: &self.names,
            props: &self.names,
            fns: &self.fns,
            observe: false,
        }
    }

    pub(super) fn reached_at(self, span: Span, bootstrap: Option<&'static str>) -> Reached {
        (self.units.into_iter().collect(), span, bootstrap)
    }

    pub(super) fn globally_reached(
        self,
        (program, scoping): (&'p Program<'p>, &Scoping),
        plan: &DecodePlan<'p>,
        runs: &[UnitRun<'p, '_>],
    ) -> Option<Reached> {
        let strings = decoded_strings(runs);
        let machinery = plan.machinery_spans();
        let found = global_reach(program, scoping, (Some(&strings), &machinery), self.watch());
        #[cfg(feature = "refusal-spans")]
        crate::refusal_spans::extend(&found.trail);
        let span = found.reach?;
        Some(self.reached_at(span, found.bootstrap))
    }
}

impl<'p> Watched<'p> {
    pub(super) fn syntactically_reached(
        self,
        (program, scoping): (&'p Program<'p>, &Scoping),
        plan: &DecodePlan<'p>,
        runs: &[UnitRun<'p, '_>],
    ) -> Option<Reached> {
        let strings = decoded_strings(runs);
        let machinery = plan.machinery_spans();
        let root = scoping.root_scope_id();
        let mut units = FxHashSet::default();
        let mut first: Option<Span> = None;
        for run in runs {
            if !self.units.contains(&run.request.unit) {
                continue;
            }
            let roots: Vec<(SymbolId, &str)> = run
                .request
                .guard
                .symbols
                .iter()
                .filter(|(symbol, _)| scoping.symbol_scope_id(*symbol) == root)
                .map(|(symbol, name)| (*symbol, *name))
                .collect();
            let targets: FxHashSet<SymbolId> = roots.iter().map(|(symbol, _)| *symbol).collect();
            let mut names: Vec<&str> = roots.iter().map(|(_, name)| *name).collect();
            names.extend(root_alias_names(program, scoping, &targets));
            let reached = super::syntactic_reach::syntactic_global_reach(
                (program, scoping),
                &strings,
                &names,
                &machinery,
            );
            if let Some(span) = reached {
                units.insert(run.request.unit);
                first.get_or_insert(span);
            }
        }
        Some((units, first?, None))
    }
}
