use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::decode::{decoded_to_expression, Call, Decoded, DecoderProgram, FnRef};

use crate::array_rotator::find_rotators;
use crate::branch_reach::in_branches;
use crate::calls::{collect_base, Collected};
use crate::dead_guards::{enclosing_guards, BranchGuard, EscapeGuards};
use crate::notes;
use crate::proof::{
    decoder_refusal, object_operand_is_intrinsic, patch_refusal, rotator_refusal_at, used_names,
    with_statement, ForeignUnits, PatchKey, PatchSite, PendingPrototype, ProtoWrite, Prototype,
    Refusal, Reliance, RotatorSite, UsedNames,
};
use crate::stamp::{
    context_is_strict_in, find_stamped_named, original_name, referenced_units, Stamped, StampedFn,
};

#[derive(Default)]
pub struct UnitGuard<'p> {
    pub names: Vec<&'p str>,
    pub pending: Vec<Span>,
    pub pending_prototypes: Vec<PendingPrototype<'p>>,
    pub proto_writes: Vec<ProtoWrite<'p>>,
    pub used: UsedNames<'p>,
    pub symbols: Vec<(SymbolId, &'p str)>,
    pub statements: Vec<Span>,
    pub escaped_decoders: FxHashSet<usize>,
}

pub struct DecodeRequest<'p> {
    pub program: DecoderProgram<'p>,
    pub calls: Vec<Call<'p>>,
    pub guard: UnitGuard<'p>,
    pub unit: usize,
    pub reliance: Option<Reliance>,
    pub escape_guards: EscapeGuards,
}

struct UnitInputs<'r, 'p> {
    program: &'p Program<'p>,
    scoping: &'r Scoping,
    array: &'r StampedFn<'p>,
    decoders: Vec<&'r StampedFn<'p>>,
    rotator: Option<&'p Expression<'p>>,
}

#[derive(Default)]
pub struct DecodePlan<'p> {
    pub requests: Vec<DecodeRequest<'p>>,
    pub unattributed: Vec<(&'p str, Span)>,
    pub refusals: Vec<Refusal>,
    pub refused: bool,
    pub intrinsic_object: bool,
}

impl DecodePlan<'_> {
    pub fn machinery_spans(&self) -> Vec<Span> {
        self.requests
            .iter()
            .flat_map(|request| {
                let rotator = request.program.rotator.map(GetSpan::span);
                request.guard.statements.iter().copied().chain(rotator)
            })
            .collect()
    }
}

fn unit_members(stamped: &Stamped<'_>) -> (FxHashMap<usize, Vec<usize>>, Vec<usize>) {
    let mut members: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    let mut unattributed = Vec::new();
    for (position, owner) in stamped.owners().into_iter().enumerate() {
        match owner {
            Some(unit) => members.entry(unit).or_default().push(position),
            None => unattributed.push(position),
        }
    }
    (members, unattributed)
}

fn local_indices(stamped: &Stamped<'_>, members: &FxHashMap<usize, Vec<usize>>) -> Sites {
    let mut sites = Sites::default();
    for (unit, positions) in members {
        for (local, position) in positions.iter().enumerate() {
            let Some(decoder) = stamped.decoders.get(*position) else {
                continue;
            };
            sites.insert(decoder.number, (*unit, local));
        }
    }
    sites
}

type Sites = FxHashMap<usize, (usize, usize)>;

fn escaped_decoders_by_unit(
    collected: &Collected<'_>,
    sites: &Sites,
) -> FxHashMap<usize, FxHashSet<usize>> {
    let mut by_unit: FxHashMap<usize, FxHashSet<usize>> = FxHashMap::default();
    let references = collected
        .decoder_values
        .iter()
        .chain(collected.dynamic_calls.iter());
    for (global_index, _) in references {
        if let Some((unit, local)) = sites.get(global_index).copied() {
            by_unit.entry(unit).or_default().insert(local);
        }
    }
    by_unit
}

fn speculated_array_escapes(
    (program, scoping): (&Program<'_>, &Scoping),
    collected: &mut Collected<'_>,
    array_span: &impl Fn(usize) -> Option<Span>,
) -> FxHashMap<usize, Span> {
    if collected.escapes.is_empty() || !crate::legacy_reach::speculates_escapes() {
        return FxHashMap::default();
    }
    let spans: FxHashSet<Span> = collected.escapes.iter().map(|(_, span)| *span).collect();
    let branched = in_branches(program, scoping, &spans);
    let unguarded: FxHashSet<usize> = collected
        .escapes
        .iter()
        .filter(|(_, span)| !branched.contains(span))
        .map(|(unit, _)| *unit)
        .collect();
    let speculated: FxHashMap<usize, Span> = collected
        .escapes
        .iter()
        .map(|(unit, _)| *unit)
        .filter(|unit| !unguarded.contains(unit))
        .filter_map(|unit| Some((unit, array_span(unit)?)))
        .collect();
    collected
        .escapes
        .retain(|(unit, _)| !speculated.contains_key(unit));
    speculated
}

fn guarded_escapes(
    (program, scoping): (&Program<'_>, &Scoping),
    collected: &mut Collected<'_>,
    (sites, rotators): (&Sites, &FxHashMap<usize, Span>),
    (decoder_spans, array_span): (impl Fn(usize) -> Vec<Span>, impl Fn(usize) -> Option<Span>),
) -> FxHashMap<usize, EscapeGuards> {
    let arrays = speculated_array_escapes((program, scoping), collected, &array_span);
    let mut by_unit = guarded_decoder_escapes(
        (program, scoping),
        collected,
        (sites, rotators),
        decoder_spans,
    );
    for (unit, span) in arrays {
        by_unit.entry(unit).or_default().speculative.push(span);
    }
    by_unit
}

fn guarded_decoder_escapes(
    (program, scoping): (&Program<'_>, &Scoping),
    collected: &mut Collected<'_>,
    (sites, rotators): (&Sites, &FxHashMap<usize, Span>),
    decoder_spans: impl Fn(usize) -> Vec<Span>,
) -> FxHashMap<usize, EscapeGuards> {
    let in_own_rotator = |(index, span): &(usize, Span)| {
        sites
            .get(index)
            .and_then(|(unit, _)| rotators.get(unit))
            .is_some_and(|rotator| rotator.start <= span.start && span.end <= rotator.end)
    };
    collected.dynamic_calls.retain(|call| !in_own_rotator(call));
    let mut by_unit: FxHashMap<usize, EscapeGuards> = FxHashMap::default();
    let escapes: FxHashSet<Span> = collected
        .decoder_values
        .iter()
        .chain(&collected.dynamic_calls)
        .map(|(_, span)| *span)
        .collect();
    if escapes.is_empty() {
        return by_unit;
    }
    let enclosing = enclosing_guards(program, &escapes);
    let branched = match crate::legacy_reach::speculates_escapes() {
        true => in_branches(program, scoping, &escapes),
        false => FxHashSet::default(),
    };
    let mut unguarded: FxHashSet<usize> = FxHashSet::default();
    let mut speculated: FxHashSet<usize> = FxHashSet::default();
    let mut unbranched: FxHashSet<usize> = FxHashSet::default();
    let mut found: Vec<(usize, bool, Vec<BranchGuard>)> = Vec::new();
    let values = collected.decoder_values.iter().map(|escape| (escape, true));
    let dynamic = collected.dynamic_calls.iter().map(|escape| (escape, false));
    for ((index, span), is_value) in values.chain(dynamic) {
        let Some((unit, _)) = sites.get(index).copied() else {
            continue;
        };
        match enclosing.get(span).filter(|guards| !guards.is_empty()) {
            Some(guards) => {
                if !branched.contains(span) {
                    unbranched.insert(unit);
                }
                found.push((unit, is_value, guards.clone()));
            }
            None if branched.contains(span) => {
                speculated.insert(unit);
            }
            None => {
                unguarded.insert(unit);
            }
        }
    }
    let is_guarded = |index: &usize| {
        sites
            .get(index)
            .is_some_and(|(unit, _)| !unguarded.contains(unit))
    };
    collected
        .decoder_values
        .retain(|(index, _)| !is_guarded(index));
    collected
        .dynamic_calls
        .retain(|(index, _)| !is_guarded(index));
    for unit in speculated {
        if !unguarded.contains(&unit) {
            by_unit.entry(unit).or_default().speculative = decoder_spans(unit);
        }
    }
    for (unit, is_value, guards) in found {
        if unguarded.contains(&unit) {
            continue;
        }
        let entry = by_unit.entry(unit).or_default();
        if !unbranched.contains(&unit) && entry.fallback.is_empty() {
            entry.fallback = decoder_spans(unit);
        }
        if is_value {
            entry.values.push(guards);
        } else {
            entry.dynamic.push(guards);
        }
    }
    by_unit
}

fn group_calls<'p>(calls: Vec<Call<'p>>, sites: &Sites) -> FxHashMap<usize, Vec<Call<'p>>> {
    let mut by_unit: FxHashMap<usize, Vec<Call<'p>>> = FxHashMap::default();
    for call in calls {
        let Some((unit, local)) = sites.get(&call.decoder).copied() else {
            continue;
        };
        by_unit.entry(unit).or_default().push(Call {
            decoder: local,
            span: call.span,
            arguments: call.arguments,
        });
    }
    by_unit
}

struct Evidence<'r, 'p> {
    escapes: FxHashMap<usize, Span>,
    decoder_values: FxHashMap<usize, Span>,
    with_statement: Option<Span>,
    rotator_sites: Vec<(usize, RotatorSite)>,
    foreign: ForeignUnits<'r, 'p>,
    protections: FxHashSet<Span>,
}

fn first_per_unit(found: impl Iterator<Item = (usize, Span)>) -> FxHashMap<usize, Span> {
    let mut first: FxHashMap<usize, Span> = FxHashMap::default();
    for (unit, span) in found {
        first.entry(unit).or_insert(span);
    }
    first
}

fn evidence<'r, 'p>(
    program: &'p Program<'p>,
    stamped: &'r Stamped<'p>,
    orphans: &[usize],
    collected: &Collected<'p>,
    sites: &Sites,
    scoping: &Scoping,
) -> Evidence<'r, 'p> {
    let orphan_escapes = orphans
        .iter()
        .filter_map(|position| stamped.decoders.get(*position))
        .flat_map(|decoder| {
            referenced_units(decoder.function)
                .into_iter()
                .map(|unit| (unit, decoder.statement))
        });
    let decoder_values = collected
        .decoder_values
        .iter()
        .filter_map(|(index, span)| sites.get(index).map(|(unit, _)| (*unit, *span)));
    Evidence {
        escapes: first_per_unit(collected.escapes.iter().copied().chain(orphan_escapes)),
        decoder_values: first_per_unit(decoder_values),
        with_statement: with_statement(program, scoping),
        rotator_sites: collected.rotator_sites.clone(),
        foreign: ForeignUnits::new(stamped, sites, &collected.rotator_sites),
        protections: FxHashSet::default(),
    }
}

fn unit_refusal(unit: usize, evidence: &Evidence<'_, '_>) -> Option<Refusal> {
    if let Some(span) = evidence.with_statement {
        return Some(Refusal::at(notes::WITH_STATEMENT, span));
    }
    if let Some(span) = evidence.escapes.get(&unit) {
        return Some(Refusal::at(notes::STRING_ARRAY_ESCAPES, *span));
    }
    evidence
        .decoder_values
        .get(&unit)
        .map(|span| Refusal::at(notes::DECODER_USED_AS_VALUE, *span))
}

fn rotator_check<'p>(
    inputs: &UnitInputs<'_, 'p>,
    unit: usize,
    evidence: &Evidence<'_, 'p>,
    located: Option<Span>,
) -> Option<Refusal> {
    let sites: Vec<RotatorSite> = evidence
        .rotator_sites
        .iter()
        .filter(|(owner, _)| *owner == unit)
        .map(|(_, site)| *site)
        .collect();
    let allowed: FxHashSet<SymbolId> = inputs
        .decoders
        .iter()
        .filter_map(|decoder| decoder.symbol)
        .chain(inputs.array.symbol)
        .collect();
    let foreign = (Some((&evidence.foreign, unit)), &evidence.protections);
    if let Some(refusal) =
        rotator_refusal_at(inputs.program, inputs.scoping, &sites, &allowed, foreign)
    {
        return Some(refusal);
    }
    let (Some(rotator), Some(span)) = (inputs.rotator, located) else {
        return None;
    };
    let proven = sites
        .first()
        .is_some_and(|site| site.holds(inputs.program, rotator));
    (!proven).then(|| Refusal::at(notes::ROTATOR_NOT_TOP_LEVEL, span))
}

fn unit_program<'p>(
    program: &'p Program<'p>,
    stamped: &Stamped<'p>,
    array: &StampedFn<'p>,
    positions: &[usize],
    rotator: Option<(Span, &'p Expression<'p>)>,
) -> DecoderProgram<'p> {
    let sorted = stamped.top_level;
    let array_strict = context_is_strict_in(program, array.statement, sorted);
    DecoderProgram {
        source_text: program.source_text,
        string_array: FnRef {
            name: array.name,
            function: array.function,
            strict: array_strict,
        },
        decoders: positions
            .iter()
            .filter_map(|position| stamped.decoders.get(*position))
            .map(|decoder| FnRef {
                name: decoder.name,
                function: decoder.function,
                strict: context_is_strict_in(program, decoder.statement, sorted),
            })
            .collect(),
        rotator: rotator.map(|(_, expression)| expression),
        strict: rotator.map_or(array_strict, |(statement, _)| {
            context_is_strict_in(program, statement, sorted)
        }),
    }
}

fn machinery_names<'p>(inputs: &UnitInputs<'_, 'p>) -> Option<Vec<(SymbolId, &'p str)>> {
    std::iter::once(inputs.array)
        .chain(inputs.decoders.iter().copied())
        .map(|entry| {
            let symbol = entry.symbol?;
            let name = original_name(inputs.program.source_text, inputs.scoping, symbol)?;
            Some((symbol, name))
        })
        .collect()
}

fn machinery_statements(inputs: &UnitInputs<'_, '_>) -> Vec<Span> {
    std::iter::once(inputs.array.statement)
        .chain(inputs.decoders.iter().map(|decoder| decoder.statement))
        .collect()
}

fn unstamp_only_guard<'p>(inputs: &UnitInputs<'_, 'p>) -> UnitGuard<'p> {
    let Some(symbols) = machinery_names(inputs) else {
        return UnitGuard::default();
    };
    UnitGuard {
        symbols,
        statements: machinery_statements(inputs),
        ..UnitGuard::default()
    }
}

fn needed_pending_spans(patches: &[PatchSite<'_>]) -> FxHashSet<Span> {
    let mut spans = FxHashSet::default();
    for site in patches {
        if let Prototype::Pending(span) = site.prototype {
            spans.insert(span);
        }
        if let PatchKey::Pending(span) = site.key {
            spans.insert(span);
        }
    }
    spans
}

fn own_pending_spans(patches: &[PatchSite<'_>], calls: &[Call<'_>]) -> FxHashSet<Span> {
    let needed = needed_pending_spans(patches);
    if needed.is_empty() {
        return FxHashSet::default();
    }
    calls
        .iter()
        .map(|call| call.span)
        .filter(|span| needed.contains(span))
        .collect()
}

fn nested_rotator<'p>(inputs: &UnitInputs<'_, 'p>) -> Option<&'p Expression<'p>> {
    let rotator = inputs.rotator?;
    let top_level = inputs.program.body.iter().any(|statement| {
        matches!(statement, Statement::ExpressionStatement(statement)
            if std::ptr::eq(&statement.expression, rotator))
    });
    (!top_level).then_some(rotator)
}

fn guard_unit<'p>(
    inputs: &UnitInputs<'_, 'p>,
    patches: &[PatchSite<'p>],
    calls: &[Call<'_>],
    escaped_decoders: FxHashSet<usize>,
) -> Result<UnitGuard<'p>, Refusal> {
    let array_symbol = inputs.array.symbol;
    for decoder in &inputs.decoders {
        if let Some(refusal) = decoder_refusal(inputs.scoping, decoder, array_symbol) {
            return Err(refusal);
        }
    }
    let used = used_names(inputs.scoping, &inputs.decoders, nested_rotator(inputs));
    let own = own_pending_spans(patches, calls);
    let verdict = patch_refusal(patches, inputs.scoping, &used, &own);
    if let Some(refusal) = verdict.refusal {
        #[cfg(feature = "refusal-spans")]
        crate::refusal_spans::record(
            || {
                format!(
                    "BUILTIN_PATCHED: {}",
                    refusal.detail.as_deref().unwrap_or("")
                )
            },
            refusal.span,
        );
        return Err(refusal);
    }
    let symbols = machinery_names(inputs)
        .ok_or_else(|| Refusal::at(notes::MACHINERY_NAME_UNREADABLE, inputs.array.statement))?;
    let statements = machinery_statements(inputs);
    Ok(UnitGuard {
        names: symbols.iter().map(|(_, name)| *name).collect(),
        pending: verdict.pending_keys,
        pending_prototypes: verdict.pending_prototypes,
        proto_writes: verdict.proto_writes,
        used,
        symbols,
        statements,
        escaped_decoders,
    })
}

fn unit_inputs<'r, 'p>(
    program: &'p Program<'p>,
    scoping: &'r Scoping,
    stamped: &'r Stamped<'p>,
    array: &'r StampedFn<'p>,
    positions: &[usize],
    rotator: Option<&'p Expression<'p>>,
) -> UnitInputs<'r, 'p> {
    let decoders: Vec<&StampedFn<'p>> = positions
        .iter()
        .filter_map(|position| stamped.decoders.get(*position))
        .collect();
    UnitInputs {
        program,
        scoping,
        array,
        decoders,
        rotator,
    }
}

#[allow(clippy::too_many_arguments)]
fn plan_unit<'p>(
    (program, located): (&'p Program<'p>, Option<(Span, &'p Expression<'p>)>),
    scoping: &Scoping,
    stamped: &Stamped<'p>,
    unit: usize,
    array: &StampedFn<'p>,
    positions: &[usize],
    unit_calls: Vec<Call<'p>>,
    evidence: &Evidence<'_, 'p>,
    patches: &[PatchSite<'p>],
    escaped_decoders: FxHashSet<usize>,
) -> PlanOutcome<'p> {
    let rotator = located.map(|(_, expr)| expr);
    let program_data = unit_program(program, stamped, array, positions, located);
    let inputs = unit_inputs(program, scoping, stamped, array, positions, rotator);
    if unit_calls.is_empty() {
        return Ok(DecodeRequest {
            program: program_data,
            calls: unit_calls,
            guard: unstamp_only_guard(&inputs),
            unit,
            reliance: None,
            escape_guards: EscapeGuards::default(),
        });
    }
    if let Some(refusal) = unit_refusal(unit, evidence) {
        return Err(refusal);
    }
    if let Some(refusal) = rotator_check(&inputs, unit, evidence, located.map(|(span, _)| span)) {
        return Err(refusal);
    }
    let guard = guard_unit(&inputs, patches, &unit_calls, escaped_decoders)?;
    Ok(DecodeRequest {
        program: program_data,
        calls: unit_calls,
        guard,
        unit,
        reliance: evidence.foreign.reliance(unit),
        escape_guards: EscapeGuards::default(),
    })
}

fn orphan_entries<'p>(stamped: &Stamped<'p>, orphans: &[usize]) -> Vec<(&'p str, Span)> {
    orphans
        .iter()
        .filter_map(|position| stamped.decoders.get(*position))
        .map(|decoder| (decoder.name, decoder.statement))
        .collect()
}

type PlanOutcome<'p> = Result<DecodeRequest<'p>, Refusal>;

fn refuse_unsupported(plan: &mut DecodePlan<'_>, unsafe_lane: bool) {
    loop {
        let decoding: Vec<usize> = plan
            .requests
            .iter()
            .filter(|request| unsafe_lane || !request.calls.is_empty())
            .map(|request| request.unit)
            .collect();
        let before = plan.requests.len();
        let mut refusals = Vec::new();
        plan.requests.retain(|request| {
            match request
                .reliance
                .as_ref()
                .and_then(|reliance| reliance.unheld(&decoding))
            {
                Some(span) => {
                    refusals.push(Refusal::at(notes::CODE_BEFORE_ROTATOR, span));
                    false
                }
                None => true,
            }
        });
        plan.refusals.extend(refusals);
        if plan.requests.len() == before {
            return;
        }
    }
}

fn push_outcome<'p>(plan: &mut DecodePlan<'p>, outcome: PlanOutcome<'p>) {
    match outcome {
        Ok(request) => plan.requests.push(request),
        Err(refusal) => plan.refusals.push(refusal),
    }
}

fn decoder_units_of(sites: &Sites) -> FxHashMap<usize, usize> {
    sites
        .iter()
        .map(|(number, (unit, _))| (*number, *unit))
        .collect()
}

pub(crate) fn decoder_units(
    program: &Program<'_>,
    scoping: &Scoping,
    unedited: bool,
) -> FxHashMap<usize, usize> {
    let stamped = find_stamped_named(program, scoping, unedited);
    let (members, _) = unit_members(&stamped);
    decoder_units_of(&local_indices(&stamped, &members))
}

pub fn build_requests<'p>(program: &'p Program<'p>, scoping: &Scoping) -> DecodePlan<'p> {
    build_requests_in_lane(program, scoping, false, false)
}

pub fn build_requests_in_lane<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    unedited: bool,
    unsafe_lane: bool,
) -> DecodePlan<'p> {
    let stamped = find_stamped_named(program, scoping, unedited);
    if stamped.incomplete {
        return DecodePlan {
            refused: true,
            ..DecodePlan::default()
        };
    }
    let (members, orphans) = unit_members(&stamped);
    if stamped.is_empty() {
        return finish_plan(DecodePlan::default(), &stamped, (&orphans, unsafe_lane));
    }
    let sites = local_indices(&stamped, &members);
    let mut collected = collect_base(program, decoder_units_of(&sites), unsafe_lane);
    let planned: Vec<(usize, &StampedFn<'p>, &Vec<usize>)> = stamped
        .units()
        .into_iter()
        .filter_map(|unit| Some((unit, stamped.array(unit)?, members.get(&unit)?)))
        .collect();
    let names: Vec<&str> = planned.iter().map(|(_, array, _)| array.name).collect();
    let rotators = find_rotators(program, &names);
    let mut guarded = if unsafe_lane {
        let rotator_spans: FxHashMap<usize, Span> = planned
            .iter()
            .zip(&rotators)
            .filter_map(|((unit, _, _), located)| Some((*unit, located.as_ref()?.0)))
            .collect();
        let decoder_spans = |unit: usize| -> Vec<Span> {
            members
                .get(&unit)
                .into_iter()
                .flatten()
                .filter_map(|position| stamped.decoders.get(*position)?.symbol)
                .map(|symbol| scoping.symbol_span(symbol))
                .collect()
        };
        let array_span = |unit: usize| -> Option<Span> {
            let symbol = stamped.array(unit)?.symbol?;
            Some(scoping.symbol_span(symbol))
        };
        guarded_escapes(
            (program, scoping),
            &mut collected,
            (&sites, &rotator_spans),
            (decoder_spans, array_span),
        )
    } else {
        FxHashMap::default()
    };
    if !collected.decoder_values.is_empty() {
        let dead = crate::removal::dead_alias_values(program, Some(scoping));
        collected
            .decoder_values
            .retain(|(_, span)| !dead.contains(span));
    }
    let mut evidence = evidence(program, &stamped, &orphans, &collected, &sites, scoping);
    if unsafe_lane {
        evidence.protections = crate::debug_protection::interval_statements(program, scoping);
    }
    let mut escaped_by_unit = escaped_decoders_by_unit(&collected, &sites);
    let mut plan = DecodePlan {
        intrinsic_object: object_operand_is_intrinsic(&collected.patches),
        requests: Vec::with_capacity(members.len()),
        ..DecodePlan::default()
    };
    let mut calls = group_calls(std::mem::take(&mut collected.calls), &sites);
    if unsafe_lane {
        evidence.foreign.decoded = planned
            .iter()
            .map(|(unit, _, _)| *unit)
            .filter(|unit| !calls.contains_key(unit))
            .collect();
    }
    for ((unit, array, positions), located) in planned.into_iter().zip(rotators) {
        let outcome = plan_unit(
            (program, located),
            scoping,
            &stamped,
            unit,
            array,
            positions,
            calls.remove(&unit).unwrap_or_default(),
            &evidence,
            &collected.patches,
            escaped_by_unit.remove(&unit).unwrap_or_default(),
        )
        .map(|request| DecodeRequest {
            escape_guards: guarded.remove(&unit).unwrap_or_default(),
            ..request
        });
        push_outcome(&mut plan, outcome);
    }
    finish_plan(plan, &stamped, (&orphans, unsafe_lane))
}

fn finish_plan<'p>(
    mut plan: DecodePlan<'p>,
    stamped: &Stamped<'p>,
    (orphans, unsafe_lane): (&[usize], bool),
) -> DecodePlan<'p> {
    refuse_unsupported(&mut plan, unsafe_lane);
    plan.unattributed = orphan_entries(stamped, orphans);
    plan.refused = !stamped.is_empty()
        && plan.requests.is_empty()
        && plan.unattributed.is_empty()
        && plan.refusals.is_empty();
    plan
}

pub fn build_request<'p>(program: &'p Program<'p>, scoping: &Scoping) -> Option<DecodeRequest<'p>> {
    build_requests(program, scoping).requests.into_iter().next()
}

pub fn primitive_value(value: Decoded) -> Option<Decoded> {
    match value {
        Decoded::Object => None,
        primitive => Some(primitive),
    }
}

pub fn decoded_expression<'a>(
    value: &Decoded,
    span: Span,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
    decoded_to_expression(value, span, b)
}
