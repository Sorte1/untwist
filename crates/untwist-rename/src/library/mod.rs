pub mod annotate;
mod guard;
mod inlined;
pub mod hash;
pub mod pass;
pub mod rename;
pub mod shape;
pub mod table;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use shape::{exact_shape, shape_program, ExactShape, Unit, UnitKind};
use table::{Role, Table};

pub const STRONG_LENGTH: u32 = 60;
pub const STRONG_SPECIFIC: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub label: String,
    pub versions: (String, String),
    pub modules: Vec<String>,
    pub role: Role,
    pub span: Span,
    pub kind: UnitKind,
    pub outermost: bool,
    pub reported: bool,
    pub also: Vec<SameCode>,
}

impl Hit {
    pub fn range(&self) -> String {
        version_range(&self.versions.0, &self.versions.1)
    }

    pub fn module_text(&self) -> String {
        self.modules.join(" | ")
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
    pub modules: Vec<String>,
}

impl SameCode {
    pub fn range(&self) -> String {
        version_range(&self.versions.0, &self.versions.1)
    }
}

struct Merged {
    also: Vec<SameCode>,
    label: String,
    versions: (String, String),
    modules: Vec<String>,
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
    fn merge(&mut self, name: Option<&String>) {
        let Some(name) = name else {
            return;
        };
        match self {
            Slot::Unknown => *self = Slot::Name(name.clone()),
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

fn bounds(versions: &str) -> Option<(String, String)> {
    let mut all: Vec<&str> = versions
        .split(',')
        .flat_map(|range| range.split('~'))
        .filter(|version| !version.is_empty())
        .collect();
    all.sort_by_key(|version| version_key(version));
    Some((all.first()?.to_string(), all.last()?.to_string()))
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

fn slot_of(name: Option<&str>) -> Slot {
    name.map_or(Slot::Unknown, |name| Slot::Name(name.to_string()))
}

fn merge_slots(into: &mut Vec<Slot>, from: &[Option<&str>]) {
    if into.len() < from.len() {
        into.resize(from.len(), Slot::Unknown);
    }
    for (slot, name) in into.iter_mut().zip(from) {
        slot.merge(name.map(str::to_string).as_ref());
    }
}

type Windows = FxHashMap<String, (String, String)>;

fn ranges(versions: &str) -> impl Iterator<Item = (&str, &str)> {
    versions
        .split(',')
        .filter(|range| !range.is_empty())
        .map(|range| range.split_once('~').unwrap_or((range, range)))
}

fn overlaps(versions: &str, window: &(String, String)) -> bool {
    let (low, high) = (version_key(&window.0), version_key(&window.1));
    ranges(versions).any(|(from, to)| version_key(from) <= high && version_key(to) >= low)
}

fn note_same_code(also: &mut Vec<SameCode>, label: &str, bounds: (String, String), module: String) {
    let Some(entry) = also.iter_mut().find(|entry| entry.label == label) else {
        also.push(SameCode {
            label: label.to_string(),
            versions: bounds,
            modules: vec![module],
        });
        return;
    };
    if !entry.modules.contains(&module) {
        entry.modules.push(module);
        entry.modules.sort();
    }
    if version_key(&bounds.0) < version_key(&entry.versions.0) {
        entry.versions.0 = bounds.0;
    }
    if version_key(&bounds.1) > version_key(&entry.versions.1) {
        entry.versions.1 = bounds.1;
    }
}

fn merge_rows(table: &Table, shape: &ExactShape, windows: Option<&Windows>) -> Option<Merged> {
    let loose = table::loose_key(shape.loose);
    let mut merged: Option<Merged> = None;
    for row in table.rows_for(table::exact_key(shape.exact)) {
        if row.loose != loose {
            continue;
        }
        let window = windows.and_then(|windows| windows.get(table.label(row)));
        if window.is_some_and(|window| !overlaps(table.versions(row), window)) {
            continue;
        }
        let row_bounds = bounds(table.versions(row))?;
        let label = table.label(row);
        let module = table.module(row).to_string();
        let Some(current) = merged.as_mut() else {
            merged = Some(Merged {
                also: Vec::new(),
                label: label.to_string(),
                versions: row_bounds,
                modules: vec![module],
                role: row.role,
                own: slot_of(table.own(row)),
                inner: table.inner(row).into_iter().map(slot_of).collect(),
                outer: table.outer(row).into_iter().map(slot_of).collect(),
            });
            continue;
        };
        if current.label != label {
            current
                .own
                .merge(table.own(row).map(str::to_string).as_ref());
            merge_slots(&mut current.inner, &table.inner(row));
            merge_slots(&mut current.outer, &table.outer(row));
            note_same_code(&mut current.also, label, row_bounds, module);
            continue;
        }
        current
            .own
            .merge(table.own(row).map(str::to_string).as_ref());
        if current.own == Slot::Conflict && current.also.is_empty() {
            return None;
        }
        if !current.modules.contains(&module) {
            current.modules.push(module);
        }
        if version_key(&row_bounds.0) < version_key(&current.versions.0) {
            current.versions.0 = row_bounds.0;
        }
        if version_key(&row_bounds.1) > version_key(&current.versions.1) {
            current.versions.1 = row_bounds.1;
        }
        current.role = current.role.min(row.role);
        merge_slots(&mut current.inner, &table.inner(row));
        merge_slots(&mut current.outer, &table.outer(row));
    }
    let mut merged = merged?;
    merged.modules.sort();
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
}

impl Offers {
    fn offer(&mut self, symbol: SymbolId, name: &str) {
        let seen = self.seen.entry(symbol).or_default();
        if !seen.iter().any(|known| known == name) {
            seen.push(name.to_string());
        }
        let entry = self
            .names
            .entry(symbol)
            .or_insert_with(|| Some(name.to_string()));
        if entry.as_deref() != Some(name) {
            *entry = None;
        }
    }

    fn has(&self, symbol: SymbolId, name: &str) -> bool {
        self.names
            .get(&symbol)
            .is_some_and(|offered| offered.as_deref() == Some(name))
    }

    fn offer_candidate(&mut self, unit: &Unit<'_, '_>, candidate: &Candidate) {
        if let (Some(binding), Some(own)) = (unit.binding, candidate.merged.own.name()) {
            self.offer(binding, own);
        }
        let slots = [
            (&candidate.shape.inner, &candidate.merged.inner),
            (&candidate.shape.outer, &candidate.merged.outer),
        ];
        for (symbols, names) in slots {
            for (symbol, name) in symbols.iter().zip(names) {
                if let Some(name) = name.name() {
                    self.offer(*symbol, name);
                }
            }
        }
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

fn windows(candidates: &[Candidate]) -> Windows {
    let mut windows: FxHashMap<String, Option<(String, String)>> = FxHashMap::default();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.strong && candidate.merged.modules.len() == 1)
    {
        let (low, high) = &candidate.merged.versions;
        let entry = windows
            .entry(candidate.merged.label.clone())
            .or_insert_with(|| Some((low.clone(), high.clone())));
        let Some(window) = entry else {
            continue;
        };
        if version_key(low) > version_key(&window.0) {
            window.0 = low.clone();
        }
        if version_key(high) < version_key(&window.1) {
            window.1 = high.clone();
        }
        if version_key(&window.0) > version_key(&window.1) {
            *entry = None;
        }
    }
    windows
        .into_iter()
        .filter_map(|(label, window)| Some((label, window?)))
        .collect()
}

fn accept(units: &[Unit<'_, '_>], candidates: &[Candidate]) -> (Vec<(usize, bool)>, Offers) {
    let mut offers = Offers::default();
    let mut accepted: Vec<(usize, bool)> = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.strong {
            offers.offer_candidate(&units[candidate.unit_index], candidate);
            accepted.push((index, true));
        }
    }
    loop {
        let before = accepted.len();
        for (index, candidate) in candidates.iter().enumerate() {
            if accepted.iter().any(|(known, _)| *known == index) {
                continue;
            }
            let unit = &units[candidate.unit_index];
            let corroborated = unit
                .binding
                .zip(candidate.merged.own.name())
                .is_some_and(|(binding, own)| offers.has(binding, own));
            if corroborated {
                offers.offer_candidate(unit, candidate);
                accepted.push((index, false));
            }
        }
        if accepted.len() == before {
            break;
        }
    }
    accepted.sort_unstable();
    (accepted, offers)
}

fn contains(outer: Span, inner: Span) -> bool {
    outer != inner && outer.start <= inner.start && inner.end <= outer.end
}

fn hits(units: &[Unit<'_, '_>], candidates: &[Candidate], accepted: &[(usize, bool)]) -> Vec<Hit> {
    let mut hits: Vec<Hit> = accepted
        .iter()
        .map(|&(index, strong)| {
            let candidate = &candidates[index];
            let unit = &units[candidate.unit_index];
            Hit {
                label: candidate.merged.label.clone(),
                versions: candidate.merged.versions.clone(),
                modules: candidate.merged.modules.clone(),
                role: candidate.merged.role,
                span: unit.span,
                kind: unit.kind,
                outermost: true,
                reported: strong || candidate.merged.role != Role::Function,
                also: candidate.merged.also.clone(),
            }
        })
        .collect();
    let snapshot: Vec<(Span, String)> = hits
        .iter()
        .map(|hit| (hit.span, hit.label.clone()))
        .collect();
    for hit in &mut hits {
        hit.outermost = !snapshot
            .iter()
            .any(|(span, label)| contains(*span, hit.span) && *label == hit.label);
    }
    hits.sort_by_key(|hit| (hit.span.start, std::cmp::Reverse(hit.span.end)));
    hits
}

pub fn match_with(table: &Table, program: &Program<'_>, scoping: &Scoping) -> LibraryNames {
    if table.is_empty() {
        return LibraryNames::default();
    }
    let shaped = shape_program(program, scoping);
    let probes = probes(table, &shaped.units, &shaped, scoping);
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

pub fn match_program(program: &Program<'_>, scoping: &Scoping) -> LibraryNames {
    match_with(&table::current(), program, scoping)
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
        .collect()
}
