use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::{decoded_strings, UnitRun};
use crate::bridge::DecodePlan;
use crate::calls::agnostic::{global_reach, ReachWatch};

pub(super) type Reached = (FxHashSet<usize>, Span, Option<&'static str>);

pub(super) struct Watched<'p> {
    units: Vec<usize>,
    names: Vec<&'p str>,
    fns: Vec<SymbolId>,
}

fn watched_units<'p>(scoping: &Scoping, runs: &[UnitRun<'p, '_>]) -> Watched<'p> {
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
        let mut global = run
            .request
            .guard
            .symbols
            .iter()
            .filter(|(symbol, _)| scoping.symbol_scope_id(*symbol) == root)
            .peekable();
        if global.peek().is_some() {
            watched.units.push(run.request.unit);
        }
        for (symbol, name) in global {
            watched.names.push(name);
            watched.fns.push(*symbol);
        }
    }
    watched
}

pub(super) fn watched<'p>(
    program: &Program<'_>,
    scoping: &Scoping,
    runs: &[UnitRun<'p, '_>],
) -> Option<Watched<'p>> {
    if untwist_core::semantic::has_module_syntax(program) {
        return None;
    }
    let watched = watched_units(scoping, runs);
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
