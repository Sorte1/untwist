use std::cell::Cell;
use std::rc::Rc;

use oxc_allocator::GetAllocator;
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::ast_util::matcher::var_function_or_declaration;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{observers, Observers};

use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

use crate::array_rotator::{find_rotators, rotator_unit};
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::mutants::{decoder_name_watch_dropped, unsealed_rotators_removed};
use crate::notes;
use crate::proof::rotator_sealed;
use crate::stamp::{
    find_stamped, find_stamped_named, is_unedited, original_name, string_array_name, Stamped,
    StampedFn,
};
use crate::units::MachineryDepth;
use untwist_core::ast_util::search::statement_at;

#[derive(Clone, Copy, Default)]
pub struct Observation {
    pub observed: Option<Span>,
    pub outside_premise: Option<Span>,
}

pub(crate) fn observed_global_names(
    program: &Program<'_>,
    scoping: &Scoping,
    nested_root: bool,
) -> Observation {
    let stamped = find_stamped(program);
    let units = units_of(program, &stamped);
    observed_names_of(program, scoping, nested_root, (&stamped, &units))
}

fn observed_names_of(
    program: &Program<'_>,
    scoping: &Scoping,
    nested_root: bool,
    (stamped, units): (&Stamped<'_>, &[Machinery]),
) -> Observation {
    if nested_root
        || untwist_core::semantic::has_module_syntax(program)
        || observers() == Observers::None
    {
        return Observation::default();
    }
    let root = scoping.root_scope_id();
    let mut names: Vec<&str> = Vec::new();
    let mut fns: Vec<SymbolId> = Vec::new();
    for entry in stamped.arrays.iter().chain(stamped.decoders.iter()) {
        if let Some(symbol) = entry.symbol {
            if scoping.symbol_scope_id(symbol) == root {
                names.push(entry.name);
                names.extend(
                    original_name(program.source_text, scoping, symbol).filter(|original| {
                        *original != entry.name && !decoder_name_watch_dropped()
                    }),
                );
                fns.push(symbol);
            }
        }
    }
    if names.is_empty() {
        return Observation::default();
    }
    let machinery: Vec<Span> = units.iter().flat_map(Machinery::statements).collect();
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &fns,
        observe: true,
    };
    let reach = global_reach(program, scoping, (None, &machinery), watch);
    #[cfg(feature = "refusal-spans")]
    record_observed(&reach);
    Observation {
        observed: reach.observed,
        outside_premise: reach.outside_premise,
    }
}

#[cfg(feature = "refusal-spans")]
fn record_observed(reach: &crate::calls::agnostic::GlobalReach) {
    let Some(span) = reach.observed else {
        return;
    };
    crate::refusal_spans::record(|| "MACHINERY_NAME_OBSERVED".to_string(), span);
    crate::refusal_spans::extend(reach.source.as_slice());
}

#[derive(Default)]
pub struct Machinery {
    pub unit: usize,
    pub array: Option<Span>,
    pub decoders: Vec<Span>,
    pub rotator: Option<Span>,
    pub rotator_sealed: bool,
}

impl Machinery {
    fn statements(&self) -> Vec<Span> {
        let mut spans = Vec::new();
        spans.extend(self.array);
        spans.extend(self.rotator);
        spans.extend(self.decoders.iter().copied());
        spans
    }
}

pub fn machinery_units(program: &Program<'_>) -> Vec<Machinery> {
    units_of(program, &find_stamped(program))
}

struct Owned<'s, 'p> {
    arrays: FxHashMap<usize, &'s StampedFn<'p>>,
    decoders: FxHashMap<usize, Vec<&'s StampedFn<'p>>>,
}

impl<'s, 'p> Owned<'s, 'p> {
    fn of(stamped: &'s Stamped<'p>) -> Self {
        let mut arrays = FxHashMap::default();
        for array in &stamped.arrays {
            arrays.entry(array.number).or_insert(array);
        }
        let mut decoders: FxHashMap<usize, Vec<&'s StampedFn<'p>>> = FxHashMap::default();
        for (decoder, owner) in stamped.decoders.iter().zip(stamped.owners()) {
            if let Some(unit) = owner {
                decoders.entry(unit).or_default().push(decoder);
            }
        }
        Self { arrays, decoders }
    }

    fn decoders_of(&self, unit: usize) -> &[&'s StampedFn<'p>] {
        self.decoders.get(&unit).map_or(&[], Vec::as_slice)
    }

    fn names_of(&self, unit: usize) -> FxHashSet<String> {
        let mut names: FxHashSet<String> = self
            .decoders_of(unit)
            .iter()
            .map(|entry| entry.name.to_string())
            .collect();
        names.insert(string_array_name(unit));
        names
    }
}

fn units_of(program: &Program<'_>, stamped: &Stamped<'_>) -> Vec<Machinery> {
    units_owned(program, stamped, &Owned::of(stamped))
}

fn units_owned(
    program: &Program<'_>,
    stamped: &Stamped<'_>,
    owned: &Owned<'_, '_>,
) -> Vec<Machinery> {
    let arrays: Vec<(usize, &StampedFn<'_>)> = stamped
        .units()
        .into_iter()
        .filter_map(|unit| Some((unit, *owned.arrays.get(&unit)?)))
        .collect();
    let names: Vec<&str> = arrays.iter().map(|(_, array)| array.name).collect();
    let rotators = find_rotators(program, &names);
    let mut units = Vec::new();
    for ((unit, array), rotator) in arrays.into_iter().zip(rotators) {
        units.push(Machinery {
            unit,
            array: Some(array.statement),
            decoders: owned
                .decoders_of(unit)
                .iter()
                .map(|entry| entry.statement)
                .collect(),
            rotator: rotator.map(|(span, _)| span),
            rotator_sealed: true,
        });
    }
    units
}

pub fn machinery(program: &Program<'_>) -> Machinery {
    machinery_units(program)
        .into_iter()
        .next()
        .unwrap_or_default()
}

struct References<'w> {
    units: Vec<MachineryDepth>,
    by_name: Option<FxHashMap<String, Vec<usize>>>,
    found: Vec<Vec<Span>>,
    listing: Option<&'w FxHashSet<Span>>,
    listed: FxHashSet<Span>,
}

fn units_by_name(units: &[MachineryDepth]) -> Option<FxHashMap<String, Vec<usize>>> {
    let mut by_name: FxHashMap<String, Vec<usize>> = FxHashMap::default();
    for (position, units) in units.iter().enumerate() {
        for name in units.names()? {
            by_name.entry(name.clone()).or_default().push(position);
        }
    }
    Some(by_name)
}

impl References<'_> {
    fn named(&self, name: &str) -> &[usize] {
        self.by_name
            .as_ref()
            .and_then(|by_name| by_name.get(name))
            .map_or(&[], Vec::as_slice)
    }

    fn candidates(&self, it: &Statement<'_>) -> Vec<usize> {
        let declared = var_function_or_declaration(it).map(|declaration| declaration.name);
        let rotated = rotator_unit(it).map(string_array_name);
        let mut candidates: Vec<usize> = declared
            .into_iter()
            .chain(rotated.as_deref())
            .flat_map(|name| self.named(name).iter().copied())
            .collect();
        candidates.sort_unstable();
        candidates.dedup();
        candidates
    }

    fn visit_indexed<'a>(&mut self, it: &Statement<'a>) {
        let candidates = self.candidates(it);
        let entered: Vec<(usize, bool)> = candidates
            .into_iter()
            .map(|position| (position, self.units[position].enter(it)))
            .collect();
        walk_js::walk_statement(self, it);
        for (position, entered) in entered {
            self.units[position].leave(entered);
        }
    }

    fn note_listed(&mut self, statements: &[Statement<'_>]) {
        let Some(listing) = self.listing else {
            return;
        };
        for statement in statements {
            let span = statement.span();
            if listing.contains(&span) {
                self.listed.insert(span);
            }
        }
    }
}

impl<'a> VisitJs<'a> for References<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        self.note_listed(it);
        walk_js::walk_statements(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let [units] = self.units.as_mut_slice() {
            let entered = units.enter(it);
            walk_js::walk_statement(self, it);
            if let Some(units) = self.units.first_mut() {
                units.leave(entered);
            }
            return;
        }
        if self.by_name.is_some() {
            self.visit_indexed(it);
            return;
        }
        let entered: Vec<bool> = self.units.iter_mut().map(|units| units.enter(it)).collect();
        walk_js::walk_statement(self, it);
        for (units, entered) in self.units.iter_mut().zip(entered) {
            units.leave(entered);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if self.by_name.is_some() {
            for position in self.named(name).to_vec() {
                if !self.units[position].inside() {
                    self.found[position].push(it.span);
                }
            }
            walk_js::walk_identifier_reference(self, it);
            return;
        }
        for (units, found) in self.units.iter().zip(self.found.iter_mut()) {
            if !units.inside() && units.covers(name) {
                found.push(it.span);
            }
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn references_listing(
    program: &Program<'_>,
    units: Vec<MachineryDepth>,
    listing: Option<&FxHashSet<Span>>,
) -> (Vec<Vec<Span>>, FxHashSet<Span>) {
    let by_name = (units.len() > 1).then(|| units_by_name(&units)).flatten();
    let mut references = References {
        found: vec![Vec::new(); units.len()],
        by_name,
        units,
        listing,
        listed: FxHashSet::default(),
    };
    references.visit_program(program);
    (references.found, references.listed)
}

fn references(program: &Program<'_>, units: MachineryDepth) -> Vec<Span> {
    references_listing(program, vec![units], None)
        .0
        .into_iter()
        .next()
        .unwrap_or_default()
}

pub fn outstanding_references(program: &Program<'_>) -> Vec<Span> {
    references(program, MachineryDepth::every_unit())
}

pub fn outstanding_references_for(program: &Program<'_>, unit: usize) -> Vec<Span> {
    let stamped = find_stamped(program);
    let names = Owned::of(&stamped).names_of(unit);
    references(program, MachineryDepth::one_unit(names))
}

#[derive(Default)]
struct Plan {
    kept: Vec<(String, Span)>,
    spans: Vec<Span>,
}

impl Plan {
    fn judge(&mut self, machinery: &Machinery, outstanding: &[Span], members: &FxHashSet<Span>) {
        if let Some(rotator) = machinery.rotator.filter(|_| !machinery.rotator_sealed) {
            self.kept
                .push((notes::ROTATOR_NOT_SEALED.to_string(), rotator));
            return;
        }
        if let Some(first) = outstanding.first() {
            self.kept
                .push((notes::machinery_references_kept(outstanding.len()), *first));
            return;
        }
        let spans = machinery.statements();
        match spans.iter().find(|span| !members.contains(span)) {
            Some(stray) => self
                .kept
                .push((notes::MACHINERY_NOT_A_LIST_MEMBER.to_string(), *stray)),
            None => self.spans.extend(spans),
        }
    }
}

fn seal_rotators(
    program: &Program<'_>,
    scoping: &Scoping,
    owned: &Owned<'_, '_>,
    units: &mut [Machinery],
) {
    if unsealed_rotators_removed() {
        return;
    }
    for machinery in units.iter_mut() {
        let Some(span) = machinery.rotator else {
            continue;
        };
        let allowed: FxHashSet<SymbolId> = owned
            .decoders_of(machinery.unit)
            .iter()
            .filter_map(|entry| entry.symbol)
            .chain(
                owned
                    .arrays
                    .get(&machinery.unit)
                    .and_then(|array| array.symbol),
            )
            .collect();
        machinery.rotator_sealed = statement_at(program, span)
            .is_some_and(|statement| rotator_sealed(statement, scoping, &allowed));
    }
}

fn plan(program: &Program<'_>, (owned, units): (&Owned<'_, '_>, &[Machinery])) -> Plan {
    let mut plan = Plan::default();
    let decoding: Vec<&Machinery> = units
        .iter()
        .filter(|machinery| !machinery.decoders.is_empty())
        .collect();
    let wanted: FxHashSet<Span> = decoding
        .iter()
        .flat_map(|machinery| machinery.statements())
        .collect();
    if decoding.is_empty() {
        return plan;
    }
    let depths: Vec<MachineryDepth> = decoding
        .iter()
        .map(|machinery| MachineryDepth::one_unit(owned.names_of(machinery.unit)))
        .collect();
    let (outstanding, members) = references_listing(program, depths, Some(&wanted));
    for (machinery, outstanding) in decoding.into_iter().zip(outstanding) {
        plan.judge(machinery, &outstanding, &members);
    }
    plan
}

pub const NAME: &str = "remove-decoder-machinery";

#[derive(Default)]
pub struct ObservedNames {
    generation: Cell<u32>,
    judged: Cell<Option<(u32, Observation)>>,
}

impl ObservedNames {
    pub fn at(&self, generation: u32) {
        self.generation.set(generation);
    }

    pub fn judge(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        nested_root: bool,
    ) -> Option<Span> {
        self.judge_with(|| observed_global_names(program, scoping, nested_root))
            .observed
    }

    fn judge_with(&self, judge: impl FnOnce() -> Observation) -> Observation {
        let generation = self.generation.get();
        if let Some((judged_at, found)) = self.judged.get() {
            if judged_at == generation {
                return found;
            }
        }
        let found = judge();
        self.judged.set(Some((generation, found)));
        found
    }
}

#[derive(Default)]
pub struct RemoveDecoderMachinery {
    observed: Option<Rc<ObservedNames>>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(RemoveDecoderMachinery::default())
}

pub fn boxed_sharing<'a>(observed: Rc<ObservedNames>) -> Box<dyn Pass<'a> + 'a> {
    Box::new(RemoveDecoderMachinery {
        observed: Some(observed),
    })
}

impl RemoveDecoderMachinery {
    fn observed_span(
        &self,
        program: &Program<'_>,
        cx: &Cx<'_, '_>,
        found: (&Stamped<'_>, &[Machinery]),
    ) -> Observation {
        let judge = || observed_names_of(program, cx.scoping(), cx.opts.nested_root, found);
        match &self.observed {
            Some(observed) => observed.judge_with(judge),
            None => judge(),
        }
    }

    fn removable(&self, program: &Program<'_>, cx: &mut Cx<'_, '_>) -> Vec<Span> {
        let stamped = find_stamped_named(program, cx.scoping(), is_unedited(cx));
        let owned = Owned::of(&stamped);
        let mut units = units_owned(program, &stamped, &owned);
        seal_rotators(program, cx.scoping(), &owned, &mut units);
        let Plan { kept, spans } = plan(program, (&owned, &units));
        for (message, span) in kept {
            cx.note(message, Some(span));
        }
        if spans.is_empty() {
            return spans;
        }
        let found = self.observed_span(program, cx, (&stamped, &units));
        if let Some(span) = found.observed {
            cx.note(notes::MACHINERY_NAME_OBSERVED.to_string(), Some(span));
            return Vec::new();
        }
        if let Some(span) = found.outside_premise {
            cx.progress(notes::MACHINERY_REMOVED_UNDER_OUTSIDE_PREMISE, Some(span));
        }
        spans
    }
}

impl<'a> Pass<'a> for RemoveDecoderMachinery {
    fn name(&self) -> &'static str {
        NAME
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let removed = crate::write_only::sweep(program, &*cx);
        if removed > 0 {
            cx.changed_by(removed);
            return;
        }
        let spans = self.removable(program, cx);
        if spans.is_empty() {
            return;
        }
        let mut removals = StmtEdits::new();
        for span in &spans {
            if !removals.remove(*span) {
                cx.note(notes::MACHINERY_NO_USABLE_SPAN, Some(*span));
                return;
            }
        }
        let wanted = removals.len();
        let applied = removals.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
        if applied != wanted {
            cx.note(notes::machinery_removed(applied, wanted), None);
        }
    }
}
