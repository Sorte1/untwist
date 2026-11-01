use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::decode::{decoded_to_expression, Call, Decoded, DecoderProgram, FnRef};

use crate::array_rotator::find_rotator;
use crate::calls::{collect_base, Collected};
use crate::notes;
use crate::proof::{
    decoder_refusal, object_operand_is_intrinsic, patch_refusal, rotator_refusal_at, used_names,
    with_statement, ForeignUnits, PatchKey, PatchSite, PendingPrototype, ProtoWrite, Prototype,
    Refusal, Reliance, RotatorSite, UsedNames,
};
use crate::stamp::{
    context_is_strict, find_stamped_named, original_name, referenced_units, Stamped, StampedFn,
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
    let foreign = Some((&evidence.foreign, unit));
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
    let array_strict = context_is_strict(program, array.statement);
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
                strict: context_is_strict(program, decoder.statement),
            })
            .collect(),
        rotator: rotator.map(|(_, expression)| expression),
        strict: rotator.map_or(array_strict, |(statement, _)| {
            context_is_strict(program, statement)
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
    let used = used_names(inputs.scoping, &inputs.decoders);
    let own = own_pending_spans(patches, calls);
    let verdict = patch_refusal(patches, inputs.scoping, &used, &own);
    if let Some(refusal) = verdict.refusal {
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
    program: &'p Program<'p>,
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
    let located = find_rotator(program, array.name);
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

fn refuse_unsupported(plan: &mut DecodePlan<'_>) {
    loop {
        let decoding: Vec<usize> = plan
            .requests
            .iter()
            .filter(|request| !request.calls.is_empty())
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

pub fn build_requests<'p>(program: &'p Program<'p>, scoping: &Scoping) -> DecodePlan<'p> {
    build_requests_named(program, scoping, false)
}

pub fn build_requests_named<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    unedited: bool,
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
        return finish_plan(DecodePlan::default(), &stamped, &orphans);
    }
    let sites = local_indices(&stamped, &members);
    let mut collected = collect_base(program, decoder_units_of(&sites));
    let evidence = evidence(program, &stamped, &orphans, &collected, &sites, scoping);
    let mut escaped_by_unit = escaped_decoders_by_unit(&collected, &sites);
    let mut plan = DecodePlan {
        intrinsic_object: object_operand_is_intrinsic(&collected.patches),
        requests: Vec::with_capacity(members.len()),
        ..DecodePlan::default()
    };
    let mut calls = group_calls(std::mem::take(&mut collected.calls), &sites);
    for unit in stamped.units() {
        let (Some(array), Some(positions)) = (stamped.array(unit), members.get(&unit)) else {
            continue;
        };
        let outcome = plan_unit(
            program,
            scoping,
            &stamped,
            unit,
            array,
            positions,
            calls.remove(&unit).unwrap_or_default(),
            &evidence,
            &collected.patches,
            escaped_by_unit.remove(&unit).unwrap_or_default(),
        );
        push_outcome(&mut plan, outcome);
    }
    finish_plan(plan, &stamped, &orphans)
}

fn finish_plan<'p>(
    mut plan: DecodePlan<'p>,
    stamped: &Stamped<'p>,
    orphans: &[usize],
) -> DecodePlan<'p> {
    refuse_unsupported(&mut plan);
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
