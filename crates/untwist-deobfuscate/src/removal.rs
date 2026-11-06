use std::cell::Cell;
use std::rc::Rc;

use oxc_allocator::GetAllocator;
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{observers, Observers};

use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

use crate::array_rotator::find_rotator;
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::mutants::decoder_name_watch_dropped;
use crate::notes;
use crate::stamp::{
    find_stamped, find_stamped_named, is_unedited, original_name, string_array_name, Stamped,
};
use crate::units::MachineryDepth;

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

fn units_of(program: &Program<'_>, stamped: &Stamped<'_>) -> Vec<Machinery> {
    let owners = stamped.owners();
    let mut units = Vec::new();
    for unit in stamped.units() {
        let Some(array) = stamped.array(unit) else {
            continue;
        };
        units.push(Machinery {
            unit,
            array: Some(array.statement),
            decoders: stamped
                .decoders
                .iter()
                .zip(&owners)
                .filter(|(_, owner)| **owner == Some(unit))
                .map(|(entry, _)| entry.statement)
                .collect(),
            rotator: find_rotator(program, array.name).map(|(span, _)| span),
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

fn names_of(stamped: &Stamped<'_>, unit: usize) -> FxHashSet<String> {
    let owners = stamped.owners();
    let mut names: FxHashSet<String> = stamped
        .decoders
        .iter()
        .zip(&owners)
        .filter(|(_, owner)| **owner == Some(unit))
        .map(|(entry, _)| entry.name.to_string())
        .collect();
    names.insert(string_array_name(unit));
    names
}

struct References<'w> {
    units: MachineryDepth,
    found: Vec<Span>,
    listing: Option<&'w FxHashSet<Span>>,
    listed: FxHashSet<Span>,
}

impl References<'_> {
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
        let entered = self.units.enter(it);
        walk_js::walk_statement(self, it);
        self.units.leave(entered);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if !self.units.inside() && self.units.covers(it.name.as_str()) {
            self.found.push(it.span);
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn references_listing(
    program: &Program<'_>,
    units: MachineryDepth,
    listing: Option<&FxHashSet<Span>>,
) -> (Vec<Span>, FxHashSet<Span>) {
    let mut references = References {
        units,
        found: Vec::new(),
        listing,
        listed: FxHashSet::default(),
    };
    references.visit_program(program);
    (references.found, references.listed)
}

fn references(program: &Program<'_>, units: MachineryDepth) -> Vec<Span> {
    references_listing(program, units, None).0
}

pub fn outstanding_references(program: &Program<'_>) -> Vec<Span> {
    references(program, MachineryDepth::every_unit())
}

pub fn outstanding_references_for(program: &Program<'_>, unit: usize) -> Vec<Span> {
    let names = names_of(&find_stamped(program), unit);
    references(program, MachineryDepth::one_unit(names))
}

#[derive(Default)]
struct Plan {
    kept: Vec<(String, Span)>,
    spans: Vec<Span>,
}

impl Plan {
    fn judge(&mut self, machinery: &Machinery, outstanding: &[Span], members: &FxHashSet<Span>) {
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

fn plan(program: &Program<'_>, (stamped, units): (&Stamped<'_>, &[Machinery])) -> Plan {
    let mut plan = Plan::default();
    let decoding: Vec<&Machinery> = units
        .iter()
        .filter(|machinery| !machinery.decoders.is_empty())
        .collect();
    let wanted: FxHashSet<Span> = decoding
        .iter()
        .flat_map(|machinery| machinery.statements())
        .collect();
    let mut members: Option<FxHashSet<Span>> = None;
    for machinery in decoding {
        let names = names_of(stamped, machinery.unit);
        let listing = members.is_none().then_some(&wanted);
        let (outstanding, listed) =
            references_listing(program, MachineryDepth::one_unit(names), listing);
        let members = members.get_or_insert(listed);
        plan.judge(machinery, &outstanding, members);
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
        let units = units_of(program, &stamped);
        let Plan { kept, spans } = plan(program, (&stamped, &units));
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
