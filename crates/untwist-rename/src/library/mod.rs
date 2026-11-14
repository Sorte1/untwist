pub mod annotate;
pub mod cache;
mod guard;
pub mod hash;
mod inlined;
pub mod pass;
pub mod rename;
pub mod shape;
pub mod table;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use shape::{exact_shape, shape_program, ExactShape, Unit, UnitKind};
use table::{Role, Table};

pub const STRONG_LENGTH: u32 = 60;
pub const STRONG_SPECIFIC: u32 = 2;

pub type Ranges = Vec<(String, String)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub label: String,
    pub versions: (String, String),
    pub ranges: Ranges,
    pub modules: Vec<String>,
    pub role: Role,
    pub span: Span,
    pub kind: UnitKind,
    pub outermost: bool,
    pub reported: bool,
    pub shared: bool,
    pub also: Vec<SameCode>,
}

impl Hit {
    pub fn range(&self) -> String {
        ranges_text(&self.ranges)
    }

    pub fn module_text(&self) -> String {
        self.modules.join(" | ")
    }

    pub fn labels(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.label.as_str()).chain(self.also.iter().map(|same| same.label.as_str()))
    }
}

#[derive(Debug, Default)]
pub struct LibraryNames {
    pub names: FxHashMap<SymbolId, String>,
    pub conflicts: Vec<(SymbolId, Vec<String>)>,
    pub hits: Vec<Hit>,
}

impl LibraryNames {
    pub fn name(&self, symbol: SymbolId) -> Option<&str> {
        self.names.get(&symbol).map(String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameCode {
    pub label: String,
    pub versions: (String, String),
    pub ranges: Ranges,
    pub modules: Vec<String>,
}

impl SameCode {
    pub fn range(&self) -> String {
        ranges_text(&self.ranges)
    }
}

#[derive(Clone, Debug)]
struct Group {
    label: String,
    ranges: Ranges,
    modules: Vec<String>,
}

impl Group {
    fn same_code(&self) -> SameCode {
        SameCode {
            label: self.label.clone(),
            versions: hull(&self.ranges),
            ranges: self.ranges.clone(),
            modules: self.modules.clone(),
        }
    }
}

struct Merged {
    groups: Vec<Group>,
    role: Role,
    own: Slot,
    inner: Vec<Slot>,
    outer: Vec<Slot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Slot {
    Unknown,
    Name(String),
    Conflict,
}

fn name_key(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_' && *c != '$')
        .flat_map(char::to_lowercase)
        .collect()
}

fn same_name(left: &str, right: &str) -> bool {
    left == right || name_key(left) == name_key(right)
}

impl Slot {
    fn merge(&mut self, name: Option<&str>) {
        let Some(name) = name else {
            return;
        };
        match self {
            Slot::Unknown => *self = Slot::Name(name.to_string()),
            Slot::Name(existing) if !same_name(existing, name) => *self = Slot::Conflict,
            _ => {}
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Slot::Name(name) => Some(name),
            _ => None,
        }
    }
}

pub(crate) fn version_key(version: &str) -> Vec<u64> {
    version
        .split(['.', '-', '+'])
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

fn short_version(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    match parts.as_slice() {
        [major, minor, ..] => format!("{major}.{minor}"),
        _ => version.to_string(),
    }
}

pub fn version_range(low: &str, high: &str) -> String {
    let (low, high) = (short_version(low), short_version(high));
    if low == high {
        return low;
    }
    format!("{low}\u{2013}{high}")
}

pub const VERSIONS_DISAGREE: &str = "versions disagree";

pub fn ranges_text(ranges: &[(String, String)]) -> String {
    let mut short: Vec<(String, String)> = Vec::new();
    for (low, high) in ranges {
        let (low, high) = (short_version(low), short_version(high));
        match short.last_mut() {
            Some(last) if version_key(&last.1) >= version_key(&low) => {
                if version_key(&high) > version_key(&last.1) {
                    last.1 = high;
                }
            }
            _ => short.push((low, high)),
        }
    }
    if short.is_empty() {
        return VERSIONS_DISAGREE.to_string();
    }
    short
        .iter()
        .map(|(low, high)| version_range(low, high))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_ranges(versions: &str) -> Ranges {
    let mut ranges: Ranges = versions
        .split(',')
        .map(str::trim)
        .filter(|range| !range.is_empty())
        .map(|range| {
            let (low, high) = range
                .split_once('~')
                .or_else(|| range.split_once('\u{2013}'))
                .unwrap_or((range, range));
            (low.trim().to_string(), high.trim().to_string())
        })
        .filter(|(low, high)| version_key(low) <= version_key(high))
        .collect();
    normalise(&mut ranges);
    ranges
}

fn normalise(ranges: &mut Ranges) {
    ranges.sort_by_key(|range| version_key(&range.0));
    let mut merged: Ranges = Vec::with_capacity(ranges.len());
    for (low, high) in ranges.drain(..) {
        match merged.last_mut() {
            Some(last) if version_key(&low) <= version_key(&last.1) => {
                if version_key(&high) > version_key(&last.1) {
                    last.1 = high;
                }
            }
            _ => merged.push((low, high)),
        }
    }
    *ranges = merged;
}

pub fn union_ranges(left: &[(String, String)], right: &[(String, String)]) -> Ranges {
    let mut all: Ranges = left.iter().chain(right).cloned().collect();
    normalise(&mut all);
    all
}

pub fn intersect_ranges(left: &[(String, String)], right: &[(String, String)]) -> Ranges {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        let (a, b) = (&left[i], &right[j]);
        let low = if version_key(&a.0) >= version_key(&b.0) {
            &a.0
        } else {
            &b.0
        };
        let a_ends_first = version_key(&a.1) <= version_key(&b.1);
        let high = if a_ends_first { &a.1 } else { &b.1 };
        if version_key(low) <= version_key(high) {
            out.push((low.clone(), high.clone()));
        }
        if a_ends_first {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

fn hull(ranges: &[(String, String)]) -> (String, String) {
    match (ranges.first(), ranges.last()) {
        (Some(first), Some(last)) => (first.0.clone(), last.1.clone()),
        _ => (String::new(), String::new()),
    }
}

fn merge_slots(into: &mut Vec<Slot>, from: &[Option<&str>]) {
    if into.len() < from.len() {
        into.resize(from.len(), Slot::Unknown);
    }
    for (slot, name) in into.iter_mut().zip(from) {
        slot.merge(*name);
    }
}

type Windows = FxHashMap<String, Ranges>;

fn add_to_group(groups: &mut Vec<Group>, label: &str, ranges: Ranges, module: String) {
    let Some(group) = groups.iter_mut().find(|group| group.label == label) else {
        groups.push(Group {
            label: label.to_string(),
            ranges,
            modules: vec![module],
        });
        return;
    };
    if !group.modules.contains(&module) {
        group.modules.push(module);
    }
    group.ranges = union_ranges(&group.ranges, &ranges);
}

fn merge_rows(table: &Table, shape: &ExactShape, windows: Option<&Windows>) -> Option<Merged> {
    let loose = table::loose_key(shape.loose);
    let mut merged: Option<Merged> = None;
    for row in table.rows_for(table::exact_key(shape.exact)) {
        if row.loose != loose {
            continue;
        }
        let label = table.label(row);
        let mut ranges = parse_ranges(table.versions(row));
        if let Some(window) = windows.and_then(|windows| windows.get(label)) {
            ranges = intersect_ranges(&ranges, window);
        }
        if ranges.is_empty() {
            continue;
        }
        let module = table.module(row).to_string();
        let current = merged.get_or_insert_with(|| Merged {
            groups: Vec::new(),
            role: row.role,
            own: Slot::Unknown,
            inner: Vec::new(),
            outer: Vec::new(),
        });
        add_to_group(&mut current.groups, label, ranges, module);
        current.role = current.role.min(row.role);
        current.own.merge(table.own(row));
        merge_slots(&mut current.inner, &table.inner(row));
        merge_slots(&mut current.outer, &table.outer(row));
    }
    let mut merged = merged?;
    if merged.own == Slot::Conflict && merged.groups.len() == 1 {
        return None;
    }
    for group in &mut merged.groups {
        group.modules.sort();
    }
    merged.groups.sort_by(|a, b| a.label.cmp(&b.label));
    Some(merged)
}

struct Candidate {
    unit_index: usize,
    shape: ExactShape,
    merged: Merged,
    strong: bool,
}

#[derive(Default)]
struct Offers {
    names: FxHashMap<SymbolId, Option<String>>,
    seen: FxHashMap<SymbolId, Vec<String>>,
    votes: FxHashMap<SymbolId, Option<String>>,
}

fn agree(entry: &mut Option<String>, name: &str) {
    if entry.as_deref() != Some(name) {
        *entry = None;
    }
}

impl Offers {
    fn offer(&mut self, symbol: SymbolId, name: &str) {
        let seen = self.seen.entry(symbol).or_default();
        if !seen.iter().any(|known| known == name) {
            seen.push(name.to_string());
        }
        agree(
            self.names
                .entry(symbol)
                .or_insert_with(|| Some(name.to_string())),
            name,
        );
    }

    fn vote(&mut self, symbol: SymbolId, name: &str) {
        agree(
            self.votes
                .entry(symbol)
                .or_insert_with(|| Some(name.to_string())),
            name,
        );
    }

    fn vouches(&self, symbol: SymbolId, name: &str) -> bool {
        let agreed = |map: &FxHashMap<SymbolId, Option<String>>| {
            map.get(&symbol)
                .is_some_and(|offered| offered.as_deref() == Some(name))
        };
        agreed(&self.names) || agreed(&self.votes)
    }
}

struct Bound {
    spans: FxHashMap<SymbolId, Vec<Span>>,
}

impl Bound {
    fn new(units: &[Unit<'_, '_>]) -> Self {
        let mut spans: FxHashMap<SymbolId, Vec<Span>> = FxHashMap::default();
        for unit in units.iter().filter(|unit| unit.kind == UnitKind::Function) {
            if let Some(binding) = unit.binding {
                spans.entry(binding).or_default().push(unit.span);
            }
        }
        Self { spans }
    }

    fn defined_outside(&self, symbol: SymbolId, within: Span) -> bool {
        self.spans.get(&symbol).is_some_and(|spans| {
            spans
                .iter()
                .any(|span| !(within.start <= span.start && span.end <= within.end))
        })
    }
}

fn probes(
    table: &Table,
    units: &[Unit<'_, '_>],
    shaped: &shape::Shaped<'_, '_>,
    scoping: &Scoping,
) -> Vec<(usize, ExactShape)> {
    units
        .iter()
        .enumerate()
        .filter(|(_, unit)| table.has_loose(table::loose_key(unit.loose_key(&shaped.prefix))))
        .map(|(unit_index, unit)| (unit_index, exact_shape(unit, scoping)))
        .chain(inlined::expanded_probes(table, units, shaped, scoping))
        .collect()
}

fn candidates(
    table: &Table,
    probes: &[(usize, ExactShape)],
    windows: Option<&Windows>,
) -> Vec<Candidate> {
    probes
        .iter()
        .filter_map(|(unit_index, shape)| {
            let merged = merge_rows(table, shape, windows)?;
            let strong = shape.loose.0 >= STRONG_LENGTH && shape.specific >= STRONG_SPECIFIC;
            Some(Candidate {
                unit_index: *unit_index,
                shape: shape.clone(),
                merged,
                strong,
            })
        })
        .collect()
}

fn single_group(candidate: &Candidate) -> Option<&Group> {
    match candidate.merged.groups.as_slice() {
        [group] => Some(group),
        _ => None,
    }
}

fn windows(candidates: &[Candidate]) -> Windows {
    let mut windows: Windows = FxHashMap::default();
    let mut empty: rustc_hash::FxHashSet<String> = rustc_hash::FxHashSet::default();
    for group in candidates
        .iter()
        .filter(|candidate| candidate.strong)
        .filter_map(single_group)
        .filter(|group| group.modules.len() == 1)
    {
        if empty.contains(&group.label) {
            continue;
        }
        let window = match windows.get(&group.label) {
            Some(window) => intersect_ranges(window, &group.ranges),
            None => group.ranges.clone(),
        };
        if window.is_empty() {
            windows.remove(&group.label);
            empty.insert(group.label.clone());
            continue;
        }
        windows.insert(group.label.clone(), window);
    }
    windows
}

struct Acceptance<'u, 'n, 'a> {
    units: &'u [Unit<'n, 'a>],
    candidates: &'u [Candidate],
    bound: Bound,
    offers: Offers,
    accepted: Vec<Option<bool>>,
    waiting: FxHashMap<SymbolId, Vec<usize>>,
    queue: Vec<usize>,
}

fn binding_and_own<'c>(
    units: &[Unit<'_, '_>],
    candidates: &'c [Candidate],
    index: usize,
) -> Option<(SymbolId, &'c str)> {
    let candidate = &candidates[index];
    units[candidate.unit_index]
        .binding
        .zip(candidate.merged.own.name())
}

impl Acceptance<'_, '_, '_> {
    fn give(&mut self, index: usize) {
        let (units, candidates) = (self.units, self.candidates);
        let candidate = &candidates[index];
        let unit = &units[candidate.unit_index];
        let mut touched: Vec<SymbolId> = Vec::new();
        if let Some((binding, own)) = binding_and_own(units, candidates, index) {
            self.offers.offer(binding, own);
            touched.push(binding);
        }
        for (symbol, name) in candidate.shape.inner.iter().zip(&candidate.merged.inner) {
            if let Some(name) = name.name() {
                self.offers.offer(*symbol, name);
                touched.push(*symbol);
            }
        }
        for (symbol, name) in candidate.shape.outer.iter().zip(&candidate.merged.outer) {
            let Some(name) = name.name() else {
                continue;
            };
            match self.bound.defined_outside(*symbol, unit.span) {
                true => self.offers.vote(*symbol, name),
                false => self.offers.offer(*symbol, name),
            }
            touched.push(*symbol);
        }
        for symbol in touched {
            self.wake(symbol);
        }
    }

    fn wake(&mut self, symbol: SymbolId) {
        let Some(waiting) = self.waiting.get(&symbol) else {
            return;
        };
        for &index in waiting {
            if self.accepted[index].is_some() {
                continue;
            }
            let vouched = binding_and_own(self.units, self.candidates, index)
                .is_some_and(|(binding, own)| self.offers.vouches(binding, own));
            if vouched {
                self.accepted[index] = Some(false);
                self.queue.push(index);
            }
        }
    }

    fn run(mut self) -> (Vec<(usize, bool)>, Offers) {
        for index in 0..self.candidates.len() {
            if self.candidates[index].strong {
                self.accepted[index] = Some(true);
                continue;
            }
            if let Some((binding, _)) = binding_and_own(self.units, self.candidates, index) {
                self.waiting.entry(binding).or_default().push(index);
            }
        }
        for index in 0..self.candidates.len() {
            if self.accepted[index] == Some(true) {
                self.give(index);
            }
        }
        while let Some(index) = self.queue.pop() {
            self.give(index);
        }
        let accepted = self
            .accepted
            .iter()
            .enumerate()
            .filter_map(|(index, strong)| Some((index, (*strong)?)))
            .collect();
        (accepted, self.offers)
    }
}

fn accept(units: &[Unit<'_, '_>], candidates: &[Candidate]) -> (Vec<(usize, bool)>, Offers) {
    Acceptance {
        units,
        candidates,
        bound: Bound::new(units),
        offers: Offers::default(),
        accepted: vec![None; candidates.len()],
        waiting: FxHashMap::default(),
        queue: Vec::new(),
    }
    .run()
}

fn context(candidates: &[Candidate], accepted: &[(usize, bool)]) -> FxHashMap<String, usize> {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    for group in accepted
        .iter()
        .filter(|(_, strong)| *strong)
        .filter_map(|(index, _)| single_group(&candidates[*index]))
    {
        *counts.entry(group.label.clone()).or_default() += 1;
    }
    counts
}

fn primary(groups: &[Group], counts: &FxHashMap<String, usize>) -> (usize, bool) {
    let score = |group: &Group| counts.get(&group.label).copied().unwrap_or(0);
    let best = groups.iter().map(score).max().unwrap_or(0);
    let leaders: Vec<usize> = (0..groups.len())
        .filter(|&index| score(&groups[index]) == best)
        .collect();
    (leaders[0], leaders.len() > 1)
}

fn mark_outermost(hits: &mut [Hit]) {
    let mut open: Vec<usize> = Vec::new();
    let mut labels: FxHashMap<String, usize> = FxHashMap::default();
    for index in 0..hits.len() {
        let span = hits[index].span;
        while let Some(&top) = open.last() {
            let outer = hits[top].span;
            if outer != span && outer.start <= span.start && span.end <= outer.end {
                break;
            }
            open.pop();
            for label in hits[top].labels() {
                if let Some(count) = labels.get_mut(label) {
                    *count -= 1;
                }
            }
        }
        let nested = hits[index]
            .labels()
            .any(|label| labels.get(label).is_some_and(|count| *count > 0));
        hits[index].outermost = !nested;
        for label in hits[index].labels() {
            *labels.entry(label.to_string()).or_default() += 1;
        }
        open.push(index);
    }
}

fn hits(units: &[Unit<'_, '_>], candidates: &[Candidate], accepted: &[(usize, bool)]) -> Vec<Hit> {
    let counts = context(candidates, accepted);
    let mut hits: Vec<Hit> = accepted
        .iter()
        .map(|&(index, strong)| {
            let candidate = &candidates[index];
            let unit = &units[candidate.unit_index];
            let groups = &candidate.merged.groups;
            let (chosen, shared) = primary(groups, &counts);
            let main = &groups[chosen];
            Hit {
                label: main.label.clone(),
                versions: hull(&main.ranges),
                ranges: main.ranges.clone(),
                modules: main.modules.clone(),
                role: candidate.merged.role,
                span: unit.span,
                kind: unit.kind,
                outermost: true,
                reported: strong || candidate.merged.role != Role::Function,
                shared,
                also: groups
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != chosen)
                    .map(|(_, group)| group.same_code())
                    .collect(),
            }
        })
        .collect();
    hits.sort_by_key(|hit| (hit.span.start, std::cmp::Reverse(hit.span.end)));
    mark_outermost(&mut hits);
    hits
}

pub fn match_with(table: &Table, program: &Program<'_>, scoping: &Scoping) -> LibraryNames {
    if table.is_empty() {
        return LibraryNames::default();
    }
    let shaped = shape_program(program, scoping);
    match_shaped(table, &shaped, scoping)
}

fn match_shaped(table: &Table, shaped: &shape::Shaped<'_, '_>, scoping: &Scoping) -> LibraryNames {
    let probes = probes(table, &shaped.units, shaped, scoping);
    let first = candidates(table, &probes, None);
    if first.is_empty() {
        return LibraryNames::default();
    }
    let windows = windows(&first);
    let found = match windows.is_empty() {
        true => first,
        false => candidates(table, &probes, Some(&windows)),
    };
    let found = inlined::settle(found, &shaped.units);
    let (accepted, offers) = accept(&shaped.units, &found);
    let mut conflicts: Vec<(SymbolId, Vec<String>)> = offers
        .names
        .iter()
        .filter(|(_, name)| name.is_none())
        .map(|(symbol, _)| {
            (
                *symbol,
                offers.seen.get(symbol).cloned().unwrap_or_default(),
            )
        })
        .collect();
    conflicts.sort_by_key(|(symbol, _)| *symbol);
    let names = offers
        .names
        .into_iter()
        .filter_map(|(symbol, name)| Some((symbol, name?)))
        .collect();
    LibraryNames {
        names,
        conflicts,
        hits: hits(&shaped.units, &found, &accepted),
    }
}

pub fn match_program(program: &Program<'_>, scoping: &Scoping) -> Arc<LibraryNames> {
    match_cached(&table::current(), program, scoping)
}

pub fn match_cached(
    table: &Arc<Table>,
    program: &Program<'_>,
    scoping: &Scoping,
) -> Arc<LibraryNames> {
    cache::matched(table, program, scoping, match_shaped)
}

pub struct UnitRecord {
    pub loose: (u32, u64),
    pub exact: u128,
    pub specific: u32,
    pub kind: UnitKind,
    pub span: Span,
    pub binding: Option<String>,
    pub inner: Vec<String>,
    pub outer: Vec<String>,
}

pub fn unit_records(program: &Program<'_>, scoping: &Scoping) -> Vec<UnitRecord> {
    let shaped = shape_program(program, scoping);
    let name = |symbol: &SymbolId| scoping.symbol_name(*symbol).to_string();
    shaped
        .units
        .iter()
        .map(|unit| {
            let shape = exact_shape(unit, scoping);
            UnitRecord {
                loose: unit.loose_key(&shaped.prefix),
                exact: shape.exact,
                specific: shape.specific,
                kind: unit.kind,
                span: unit.span,
                binding: unit.binding.as_ref().map(name),
                inner: shape.inner.iter().map(name).collect(),
                outer: shape.outer.iter().map(name).collect(),
            }
        })
        .chain(inlined::expanded_records(&shaped, scoping))
        .collect()
}
