use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::baseline::Baseline;
use super::collect::{Callee, DeclKind, Facts};
use super::compat::{Compat, Verdict};
use super::dead::{Dead, Origin};
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
    RunsKept,
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
    pub helpers: FxHashMap<SymbolId, (Vec<String>, (String, String))>,
    pub installers: Vec<Installer>,
    pub removed_helpers: usize,
    pub kept_helpers: usize,
    pub library: Vec<Span>,
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
    pub library: &'i [Span],
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
        .filter(|hit| !hit.span.is_empty())
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
        if found.span.is_empty() {
            continue;
        }
        let nested = hits.iter().any(|outer| {
            outer.span.start <= found.span.start
                && found.span.end <= outer.span.end
                && outer.modules == found.modules
        });
        if !nested && known.insert(found.span) {
            out.push(unit(
                found.span,
                found.modules,
                found.versions,
                Source::Lookalike,
                cx,
            ));
        }
    }
    let named: FxHashSet<SymbolId> = out
        .iter()
        .flat_map(|unit| unit.outer.iter().copied())
        .collect();
    for span in &facts.trivial {
        if !known.insert(*span) {
            continue;
        }
        let candidate = unit(
            *span,
            Vec::new(),
            (String::new(), String::new()),
            Source::Trivial,
            cx,
        );
        if candidate
            .binding
            .is_some_and(|binding| named.contains(&binding))
        {
            out.push(candidate);
        }
    }
    out
}

pub struct Graph<'u> {
    units: &'u [Unit],
    pub by_binding: FxHashMap<SymbolId, usize>,
    refs: FxHashMap<SymbolId, Vec<usize>>,
    state: FxHashMap<SymbolId, bool>,
}

fn coverage(spans: impl Iterator<Item = Span>) -> Vec<Span> {
    let mut spans: Vec<Span> = spans.collect();
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

fn inside(coverage: &[Span], span: Span) -> bool {
    let index = coverage.partition_point(|outer| outer.start <= span.start);
    index > 0 && span.end <= coverage[index - 1].end
}

impl<'u> Graph<'u> {
    pub fn new(units: &'u [Unit], facts: &Facts) -> Self {
        let by_binding: FxHashMap<SymbolId, usize> = units
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| unit.binding.map(|symbol| (symbol, index)))
            .collect();
        let mut refs: FxHashMap<SymbolId, Vec<usize>> = FxHashMap::default();
        for (index, site) in facts.refs.iter().enumerate() {
            refs.entry(site.symbol).or_default().push(index);
        }
        let spans = coverage(units.iter().map(|unit| unit.span));
        let mut state = FxHashMap::default();
        for symbol in units.iter().flat_map(|unit| unit.outer.iter()) {
            if by_binding.contains_key(symbol) || state.contains_key(symbol) {
                continue;
            }
            let is_var = facts
                .decls
                .get(symbol)
                .is_some_and(|decl| matches!(decl.kind, DeclKind::Var { .. }));
            let inside = refs
                .get(symbol)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .all(|&at| covered(&spans, facts.refs[at].start));
            state.insert(*symbol, is_var && inside);
        }
        Graph {
            units,
            by_binding,
            refs,
            state,
        }
    }

    fn refs_of(&self, symbol: SymbolId) -> &[usize] {
        self.refs.get(&symbol).map_or(&[], Vec::as_slice)
    }

    fn state_var(&self, symbol: SymbolId) -> bool {
        self.state.get(&symbol).copied().unwrap_or(false)
    }

    pub fn closed(&self) -> Vec<bool> {
        let mut closed = vec![true; self.units.len()];
        let mut users: Vec<Vec<usize>> = vec![Vec::new(); self.units.len()];
        let mut queue = Vec::new();
        for (index, unit) in self.units.iter().enumerate() {
            for symbol in &unit.outer {
                match self.by_binding.get(symbol) {
                    Some(&dep) => users[dep].push(index),
                    None if !self.state_var(*symbol) && closed[index] => {
                        closed[index] = false;
                        queue.push(index);
                    }
                    None => {}
                }
            }
        }
        while let Some(dep) = queue.pop() {
            for &user in &users[dep] {
                if closed[user] {
                    closed[user] = false;
                    queue.push(user);
                }
            }
        }
        closed
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
    let written: FxHashSet<SymbolId> = facts
        .refs_in(unit.span)
        .iter()
        .filter(|site| site.write)
        .map(|site| site.symbol)
        .collect();
    unit.outer
        .iter()
        .filter(|symbol| !graph.by_binding.contains_key(symbol))
        .filter(|symbol| written.contains(symbol))
        .any(|symbol| !graph.uses_outside(*symbol, unit.span, facts).is_empty())
}

fn text_read(spans: &[Span], unit: &Unit, inputs: &Inputs<'_>) -> bool {
    std::iter::once(unit.span)
        .chain(spans.iter().copied())
        .flat_map(|span| inputs.facts.refs_in(span))
        .any(|site| inputs.pinned.contains(&site.symbol))
}

fn checks(
    index: usize,
    closed: &[bool],
    graph: &Graph<'_>,
    inputs: &Inputs<'_>,
) -> Result<Vec<Span>, Keep> {
    let unit = &graph.units[index];
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

#[derive(Clone, Debug)]
enum Judged {
    Remove(Vec<Span>),
    Cascade,
    Spare(Keep),
    Keep(Keep),
}

type FirstLook = (Option<Keep>, Result<Vec<Span>, Keep>);

fn judge_all(
    installers: &[usize],
    closed: &[bool],
    graph: &Graph<'_>,
    inputs: &Inputs<'_>,
) -> Vec<Judged> {
    let first: Vec<FirstLook> = installers
        .iter()
        .map(|&index| {
            (
                verdict(&graph.units[index], inputs),
                checks(index, closed, graph, inputs),
            )
        })
        .collect();
    let removable = coverage(
        installers
            .iter()
            .zip(&first)
            .filter(|(_, (own, rest))| own.is_none() && rest.is_ok())
            .map(|(&index, _)| graph.units[index].span),
    );
    first
        .into_iter()
        .zip(installers)
        .map(|(judged, &index)| match judged {
            (None, Ok(sites)) => Judged::Remove(sites),
            (None, Err(keep @ (Keep::ExportsRead | Keep::NotRun)))
                if !text_read(&[], &graph.units[index], inputs) =>
            {
                Judged::Spare(keep)
            }
            (None, Err(keep)) => Judged::Keep(keep),
            (Some(Keep::Unlisted), Ok(sites))
                if sites.iter().all(|site| inside(&removable, *site)) =>
            {
                Judged::Cascade
            }
            (Some(keep), _) => Judged::Keep(keep),
        })
        .collect()
}

const MAX_SWEEPS: usize = 32;

struct Sets {
    eligible: FxHashSet<SymbolId>,
    helpers: FxHashSet<SymbolId>,
}

fn sets(
    units: &[Unit],
    closed: &[bool],
    graph: &Graph<'_>,
    inputs: &Inputs<'_>,
    judged: &FxHashMap<usize, Judged>,
) -> Sets {
    let mut eligible: FxHashSet<SymbolId> = FxHashSet::default();
    let mut helpers: FxHashSet<SymbolId> = FxHashSet::default();
    for (index, unit) in units.iter().enumerate() {
        if let (Some(binding), true) = (unit.binding, closed[index]) {
            match judged.get(&index) {
                None => {
                    helpers.insert(binding);
                    eligible.insert(binding);
                }
                Some(Judged::Remove(_) | Judged::Cascade | Judged::Spare(_)) => {
                    eligible.insert(binding);
                }
                Some(Judged::Keep(_)) => {}
            }
        }
        for symbol in &unit.outer {
            if !graph.by_binding.contains_key(symbol) && graph.state_var(*symbol) {
                eligible.insert(*symbol);
            }
        }
    }
    let library = coverage(
        units
            .iter()
            .filter(|unit| unit.source != Source::Trivial)
            .map(|unit| unit.span)
            .chain(inputs.library.iter().copied()),
    );
    for (symbol, decl) in &inputs.facts.decls {
        if let DeclKind::Var {
            call_of: Some(callee),
            ..
        } = decl.kind
        {
            if helpers.contains(&callee) && inside(&library, decl.region) {
                eligible.insert(*symbol);
            }
        }
    }
    Sets { eligible, helpers }
}

struct Sweep {
    regions: Regions,
    blame: Vec<Origin>,
}

fn sweep(
    units: &[Unit],
    graph: &Graph<'_>,
    inputs: &Inputs<'_>,
    judged: &FxHashMap<usize, Judged>,
    sets: &Sets,
) -> Sweep {
    let mut dead = Dead::new(inputs.facts, inputs.pinned, &sets.eligible, &sets.helpers);
    let mut order: Vec<&usize> = judged.keys().collect();
    order.sort_unstable();
    for &index in order {
        if let Some(Judged::Remove(sites)) = judged.get(&index) {
            for site in sites {
                dead.remove(*site, Origin::Installer(index));
            }
        }
    }
    dead.drain();
    dead.seed_unused();
    dead.drain();
    let mut blame = Vec::new();
    for (&index, judged) in judged {
        let unit = &units[index];
        let removed = dead.regions.covers(unit.span);
        match judged {
            Judged::Remove(_) | Judged::Spare(_) => continue,
            Judged::Cascade if removed => continue,
            Judged::Keep(_) if removed => {
                blame.extend(dead.origin_over(unit.span));
                continue;
            }
            _ => {}
        }
        let Some(binding) = unit.binding else {
            continue;
        };
        for &at in graph.refs_of(binding) {
            blame.extend(dead.origin_at(inputs.facts.refs[at].start));
        }
    }
    Sweep {
        regions: dead.regions,
        blame,
    }
}

fn settle(
    units: &[Unit],
    closed: &[bool],
    graph: &Graph<'_>,
    inputs: &Inputs<'_>,
    judged: &mut FxHashMap<usize, Judged>,
) -> Regions {
    let mut blocked: FxHashSet<SymbolId> = FxHashSet::default();
    for _ in 0..MAX_SWEEPS {
        let mut sets = sets(units, closed, graph, inputs, judged);
        sets.eligible.retain(|symbol| !blocked.contains(symbol));
        let found = sweep(units, graph, inputs, judged, &sets);
        if found.blame.is_empty() {
            return found.regions;
        }
        for origin in found.blame {
            match origin {
                Origin::Installer(index) => {
                    judged.insert(index, Judged::Keep(Keep::RunsKept));
                }
                Origin::Unused(symbol) => {
                    blocked.insert(symbol);
                }
            }
        }
    }
    for judged in judged.values_mut() {
        if matches!(judged, Judged::Remove(_) | Judged::Cascade) {
            *judged = Judged::Keep(Keep::RunsKept);
        }
    }
    Regions::default()
}

pub fn plan(units: &[Unit], inputs: &Inputs<'_>) -> Plan {
    let graph = Graph::new(units, inputs.facts);
    let closed = graph.closed();
    let mut plan = Plan::default();
    for (index, unit) in units.iter().enumerate() {
        let library_only = unit.outer.iter().all(|symbol| {
            graph
                .by_binding
                .get(symbol)
                .is_none_or(|&dep| units[dep].source == Source::Matched)
        });
        if let (Some(binding), true, Source::Matched, true) =
            (unit.binding, closed[index], unit.source, library_only)
        {
            plan.helpers
                .insert(binding, (unit.modules.clone(), unit.versions.clone()));
        }
    }
    let installers: Vec<usize> = (0..units.len())
        .filter(|&index| is_installer(&units[index]))
        .collect();
    let mut judged: FxHashMap<usize, Judged> = installers
        .iter()
        .copied()
        .zip(judge_all(&installers, &closed, &graph, inputs))
        .collect();
    plan.regions = settle(units, &closed, &graph, inputs, &mut judged);
    for &index in &installers {
        let unit = &units[index];
        let removed = plan.regions.covers(unit.span);
        let kept = match &judged[&index] {
            Judged::Remove(_) => None,
            Judged::Cascade | Judged::Spare(_) if removed => None,
            Judged::Cascade => Some(Keep::Unlisted),
            Judged::Spare(keep) => Some(keep.clone()),
            Judged::Keep(keep) => Some(keep.clone()),
        };
        plan.installers.push(Installer {
            modules: unit.modules.clone(),
            range: unit.range.clone(),
            span: unit.span,
            with_others: kept.is_none()
                && matches!(judged[&index], Judged::Cascade | Judged::Spare(_)),
            kept,
        });
    }
    let (removed, kept) = helper_counts(units, &plan.regions);
    plan.removed_helpers = removed;
    plan.kept_helpers = kept;
    plan.library = units
        .iter()
        .filter(|unit| unit.source != Source::Trivial)
        .map(|unit| unit.span)
        .collect();
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
