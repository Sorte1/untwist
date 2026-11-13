use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::baseline::Baseline;
use super::collect::{Callee, DeclKind, Facts};
use super::compat::{Compat, Verdict};
use super::lookalike::Lookalike;
use super::regions::Regions;
use crate::library::shape::UnitKind;
use crate::library::table::Role;
use crate::library::Hit;

pub const LABEL: &str = "core-js";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Keep {
    Proposal,
    Needed {
        engine: String,
        needs: Option<String>,
        baseline: String,
    },
    Unlisted,
    Foreign,
    ExportsRead,
    NotRun,
    SharedState,
    TextRead,
}

#[derive(Clone, Debug)]
pub struct Installer {
    pub modules: Vec<String>,
    pub range: String,
    pub span: Span,
    pub kept: Option<Keep>,
    pub with_others: bool,
}

#[derive(Default)]
pub struct Plan {
    pub regions: Regions,
    pub installers: Vec<Installer>,
    pub removed_helpers: usize,
    pub kept_helpers: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Matched,
    Lookalike,
    Trivial,
}

pub struct Unit {
    pub span: Span,
    pub modules: Vec<String>,
    pub versions: (String, String),
    pub range: String,
    pub role: Role,
    pub source: Source,
    pub binding: Option<SymbolId>,
    pub outer: Vec<SymbolId>,
}

pub struct Inputs<'i> {
    pub facts: &'i Facts,
    pub scoping: &'i Scoping,
    pub compat: &'i Compat,
    pub baseline: &'i Baseline,
    pub pinned: &'i FxHashSet<SymbolId>,
}

fn outer_symbols(facts: &Facts, scoping: &Scoping, span: Span) -> Vec<SymbolId> {
    let mut seen = FxHashSet::default();
    facts
        .refs_in(span)
        .iter()
        .map(|site| site.symbol)
        .filter(|symbol| {
            let declared = scoping.symbol_span(*symbol);
            !(span.start <= declared.start && declared.end <= span.end)
        })
        .filter(|symbol| seen.insert(*symbol))
        .collect()
}

fn unit(
    span: Span,
    modules: Vec<String>,
    versions: (String, String),
    source: Source,
    cx: (&Facts, &Scoping),
) -> Unit {
    let (facts, scoping) = cx;
    Unit {
        span,
        range: crate::library::version_range(&versions.0, &versions.1),
        modules,
        versions,
        role: Role::Wrapper,
        source,
        binding: facts.function_binding.get(&span).copied(),
        outer: outer_symbols(facts, scoping, span),
    }
}

pub fn module_hits(hits: &[Hit]) -> Vec<&Hit> {
    let mut seen = FxHashSet::default();
    hits.iter()
        .filter(|hit| hit.label == LABEL && hit.kind == UnitKind::Function)
        .filter(|hit| matches!(hit.role, Role::Wrapper | Role::Module))
        .filter(|hit| seen.insert(hit.span))
        .collect()
}

pub fn units(
    hits: &[&Hit],
    lookalikes: Vec<Lookalike>,
    facts: &Facts,
    scoping: &Scoping,
) -> Vec<Unit> {
    let cx = (facts, scoping);
    let mut out: Vec<Unit> = hits
        .iter()
        .map(|hit| {
            unit(
                hit.span,
                hit.modules.clone(),
                hit.versions.clone(),
                Source::Matched,
                cx,
            )
        })
        .collect();
    out.retain(|inner| {
        !hits.iter().any(|outer| {
            outer.span != inner.span
                && outer.span.start <= inner.span.start
                && inner.span.end <= outer.span.end
                && outer.modules == inner.modules
        })
    });
    let mut known: FxHashSet<Span> = hits.iter().map(|hit| hit.span).collect();
    for found in lookalikes {
        if known.insert(found.span) {
            out.push(unit(
                found.span,
                found.modules,
                found.versions,
                Source::Lookalike,
                cx,
            ));
        }
    }
    for span in &facts.trivial {
        if known.insert(*span) {
            out.push(unit(
                *span,
                Vec::new(),
                (String::new(), String::new()),
                Source::Trivial,
                cx,
            ));
        }
    }
    out
}

pub struct Graph<'u> {
    units: &'u [Unit],
    by_binding: FxHashMap<SymbolId, usize>,
    refs: FxHashMap<SymbolId, Vec<usize>>,
    coverage: Vec<Span>,
}

fn coverage(units: &[Unit]) -> Vec<Span> {
    let mut spans: Vec<Span> = units.iter().map(|unit| unit.span).collect();
    spans.sort_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        match out.last() {
            Some(last) if span.end <= last.end => {}
            _ => out.push(span),
        }
    }
    out
}

fn covered(coverage: &[Span], at: u32) -> bool {
    let index = coverage.partition_point(|span| span.start <= at);
    index > 0 && at < coverage[index - 1].end
}

impl<'u> Graph<'u> {
    pub fn new(units: &'u [Unit], facts: &Facts) -> Self {
        let by_binding = units
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| unit.binding.map(|symbol| (symbol, index)))
            .collect();
        let mut refs: FxHashMap<SymbolId, Vec<usize>> = FxHashMap::default();
        for (index, site) in facts.refs.iter().enumerate() {
            refs.entry(site.symbol).or_default().push(index);
        }
        Graph {
            units,
            by_binding,
            refs,
            coverage: coverage(units),
        }
    }

    fn refs_of(&self, symbol: SymbolId) -> &[usize] {
        self.refs.get(&symbol).map_or(&[], Vec::as_slice)
    }

    fn state_var(&self, symbol: SymbolId, facts: &Facts) -> bool {
        let is_var = facts
            .decls
            .get(&symbol)
            .is_some_and(|decl| matches!(decl.kind, DeclKind::Var { .. }));
        is_var
            && self
                .refs_of(symbol)
                .iter()
                .all(|&at| covered(&self.coverage, facts.refs[at].start))
    }

    pub fn closed(&self, facts: &Facts) -> Vec<bool> {
        let mut closed = vec![true; self.units.len()];
        loop {
            let mut changed = false;
            for index in 0..self.units.len() {
                if closed[index] && !self.outer_ok(index, &closed, facts) {
                    closed[index] = false;
                    changed = true;
                }
            }
            if !changed {
                return closed;
            }
        }
    }

    fn outer_ok(&self, index: usize, closed: &[bool], facts: &Facts) -> bool {
        self.units[index]
            .outer
            .iter()
            .all(|symbol| match self.by_binding.get(symbol) {
                Some(&unit) => closed[unit],
                None => self.state_var(*symbol, facts),
            })
    }

    fn uses_outside(&self, symbol: SymbolId, span: Span, facts: &Facts) -> Vec<u32> {
        self.refs_of(symbol)
            .iter()
            .map(|&at| facts.refs[at].start)
            .filter(|start| !(span.start <= *start && *start < span.end))
            .collect()
    }
}

fn module_name(module: &str) -> Option<&str> {
    module.strip_prefix("modules/")
}

pub fn is_installer(unit: &Unit) -> bool {
    unit.source != Source::Trivial
        && !unit.modules.is_empty()
        && unit
            .modules
            .iter()
            .all(|module| module_name(module).is_some())
}

fn verdict(unit: &Unit, inputs: &Inputs<'_>) -> Option<Keep> {
    for module in &unit.modules {
        let name = module_name(module)?;
        if !(name.starts_with("es.") || name.starts_with("web.")) {
            return Some(Keep::Proposal);
        }
        let versions = (unit.versions.0.as_str(), unit.versions.1.as_str());
        match inputs.compat.verdict(name, versions, inputs.baseline) {
            Verdict::Native => {}
            Verdict::Needed {
                engine,
                needs,
                baseline,
            } => {
                return Some(Keep::Needed {
                    engine,
                    needs,
                    baseline,
                })
            }
            Verdict::Unlisted => return Some(Keep::Unlisted),
        }
    }
    None
}

fn run_sites(unit: &Unit, graph: &Graph<'_>, facts: &Facts) -> Result<Vec<Span>, Keep> {
    let Some(binding) = unit.binding else {
        let sites: Vec<Span> = facts
            .discarded
            .iter()
            .filter(|site| site.callee == Callee::Function(unit.span))
            .map(|site| site.span)
            .collect();
        return if sites.is_empty() {
            Err(Keep::NotRun)
        } else {
            Ok(sites)
        };
    };
    let uses = graph.uses_outside(binding, unit.span, facts);
    let sites: Vec<Span> = facts
        .discarded
        .iter()
        .filter(|site| site.callee == Callee::Symbol(binding))
        .map(|site| site.span)
        .collect();
    let all_run = uses.iter().all(|start| {
        sites
            .iter()
            .any(|site| site.start <= *start && *start < site.end)
    });
    match (all_run && uses.len() == sites.len(), sites.is_empty()) {
        (true, false) => Ok(sites),
        (true, true) => Err(Keep::NotRun),
        (false, _) => Err(Keep::ExportsRead),
    }
}

fn shares_state(unit: &Unit, graph: &Graph<'_>, facts: &Facts) -> bool {
    unit.outer
        .iter()
        .filter(|symbol| !graph.by_binding.contains_key(symbol))
        .filter(|symbol| {
            facts
                .refs_in(unit.span)
                .iter()
                .any(|site| site.symbol == **symbol && site.write)
        })
        .any(|symbol| !graph.uses_outside(*symbol, unit.span, facts).is_empty())
}

fn text_read(spans: &[Span], unit: &Unit, inputs: &Inputs<'_>) -> bool {
    std::iter::once(unit.span)
        .chain(spans.iter().copied())
        .flat_map(|span| inputs.facts.refs_in(span))
        .any(|site| inputs.pinned.contains(&site.symbol))
}

fn judge(
    index: usize,
    closed: &[bool],
    graph: &Graph<'_>,
    inputs: &Inputs<'_>,
) -> Result<Vec<Span>, Keep> {
    let unit = &graph.units[index];
    if let Some(keep) = verdict(unit, inputs) {
        return Err(keep);
    }
    if !closed[index] {
        return Err(Keep::Foreign);
    }
    if shares_state(unit, graph, inputs.facts) {
        return Err(Keep::SharedState);
    }
    let sites = run_sites(unit, graph, inputs.facts)?;
    if text_read(&sites, unit, inputs) {
        return Err(Keep::TextRead);
    }
    Ok(sites)
}

pub fn plan(units: &[Unit], inputs: &Inputs<'_>) -> Plan {
    let graph = Graph::new(units, inputs.facts);
    let closed = graph.closed(inputs.facts);
    let mut plan = Plan::default();
    let mut eligible: FxHashSet<SymbolId> = FxHashSet::default();
    for (index, unit) in units.iter().enumerate() {
        if let Some(binding) = unit.binding {
            if closed[index] {
                eligible.insert(binding);
            }
        }
        for symbol in &unit.outer {
            if !graph.by_binding.contains_key(symbol) && graph.state_var(*symbol, inputs.facts) {
                eligible.insert(*symbol);
            }
        }
    }
    let mut seeds: Vec<Span> = Vec::new();
    for (index, unit) in units
        .iter()
        .enumerate()
        .filter(|(_, unit)| is_installer(unit))
    {
        let judged = judge(index, &closed, &graph, inputs);
        if let Ok(sites) = &judged {
            seeds.extend(sites.iter().copied());
        }
        plan.installers.push(Installer {
            modules: unit.modules.clone(),
            range: unit.range.clone(),
            span: unit.span,
            kept: judged.err(),
            with_others: false,
        });
    }
    let mut dead = Dead::new(&graph, inputs, eligible);
    for seed in seeds {
        dead.remove(seed);
    }
    dead.drain();
    plan.regions = dead.regions;
    for installer in &mut plan.installers {
        if installer.kept.is_some() && plan.regions.covers(installer.span) {
            installer.kept = None;
            installer.with_others = true;
        }
    }
    let (removed, kept) = helper_counts(units, &plan.regions);
    plan.removed_helpers = removed;
    plan.kept_helpers = kept;
    plan
}

fn helper_counts(units: &[Unit], regions: &Regions) -> (usize, usize) {
    let helpers = || {
        units
            .iter()
            .filter(|unit| unit.source != Source::Trivial && !is_installer(unit))
    };
    let removed = helpers().filter(|unit| regions.covers(unit.span)).count();
    (removed, helpers().count() - removed)
}

struct Dead<'g, 'i> {
    inputs: &'g Inputs<'i>,
    eligible: FxHashSet<SymbolId>,
    counts: FxHashMap<SymbolId, usize>,
    queue: Vec<Span>,
    regions: Regions,
}

impl<'g, 'i> Dead<'g, 'i> {
    fn new(graph: &'g Graph<'g>, inputs: &'g Inputs<'i>, eligible: FxHashSet<SymbolId>) -> Self {
        let counts = graph
            .refs
            .iter()
            .map(|(symbol, refs)| (*symbol, refs.len()))
            .collect();
        Dead {
            inputs,
            eligible,
            counts,
            queue: Vec::new(),
            regions: Regions::default(),
        }
    }

    fn remove(&mut self, span: Span) {
        self.queue.push(span);
    }

    fn drain(&mut self) {
        while let Some(span) = self.queue.pop() {
            let Some(subsumed) = self.regions.insert(span) else {
                continue;
            };
            let facts = self.inputs.facts;
            for site in facts.refs_in(span) {
                if subsumed
                    .iter()
                    .any(|inner| inner.start <= site.start && site.start < inner.end)
                {
                    continue;
                }
                self.release(site.symbol);
            }
        }
    }

    fn release(&mut self, symbol: SymbolId) {
        let Some(count) = self.counts.get_mut(&symbol) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count > 0 || !self.eligible.contains(&symbol) {
            return;
        }
        let Some(decl) = self.inputs.facts.decls.get(&symbol) else {
            return;
        };
        let removable = !decl.global
            && match decl.kind {
                DeclKind::Function => true,
                DeclKind::Var { pure } => pure,
            };
        if removable && !self.inputs.pinned.contains(&symbol) {
            self.queue.push(decl.region);
        }
    }
}
