mod collect;
mod containers;
mod gates;
mod miss_path;
mod monitor;
mod params;
mod probe;
pub(crate) mod recipe;
mod registry;
mod reset;

pub use monitor::{
    address_of, class_of, enter_gate, enter_invocation, leave_gate, leave_invocation, note_capture,
    note_delete, note_inherited_link, note_key_observation, note_object_drop, note_prototype,
    note_read, note_rebind, note_rotation_step, note_write, register_machinery_self,
    set_state_tracing, state_tracing, take_violation, ObjClass, StateTracingGuard,
};
pub(crate) use monitor::{invocation_quiet, machinery_owner, violation_pending};
pub(crate) use probe::{
    note_recipe_miss, predict_recipe_miss, skipped_count, skipped_here, RecipeInsert,
};
pub use probe::{probe_dead_terms, probe_miss, ProbeGuard};

use oxc_ast::ast::Function;
use oxc_span::Span;

use crate::decode::DecoderProgram;
use crate::value::{JsString, PropKey};

use collect::{
    collect_for_function, collect_for_rotator, rotator_function, Collected, FinalKey, MemberPeel,
    TriggerKind,
};

const UNPROVEN_GENERIC: &str =
    "a stateful decoder keeps state that is not a proven memo or init gate";
const UNPROVEN_CLOSURE: &str = "a stateful decoder writes a variable outside its own machinery";
const UNPROVEN_THIS: &str = "a stateful decoder writes through a sloppy-mode this";
const UNPROVEN_CROSS: &str =
    "a stateful decoder's state is written by a different machinery function";
const UNPROVEN_DUPLICATE_GATE: &str = "two init gates share the same guarded property";

#[derive(Clone)]
pub struct GateShape {
    pub container: Span,
    pub key: PropKey,
    pub guard: Span,
}

#[derive(Clone)]
pub struct MemoShape {
    pub container: Span,
    pub key: Span,
    pub owner: usize,
    pub container_key: Option<PropKey>,
}

#[derive(Clone)]
pub struct ResetShape {
    pub guard: Span,
    pub memo: PropKey,
}

#[derive(Clone, Default)]
pub struct Shape {
    pub gates: Vec<GateShape>,
    pub resets: Vec<ResetShape>,
    pub memos: Vec<MemoShape>,
    pub used_params: Vec<Vec<bool>>,
    pub numeric_params: Vec<Vec<bool>>,
    pub conditional: Vec<(Span, PropKey)>,
    pub rewrites: Vec<(Span, usize)>,
    pub captured_memo: bool,
    pub miss_paths_proven: bool,
    pub gate_owners: Vec<(Span, usize)>,
}

pub enum MachineryState {
    Stateless,
    Refused(&'static &'static str),
    Candidate(Shape),
}

pub fn keeps_state(function: &Function<'_>) -> bool {
    let own_name = function.id.as_ref().map(|id| id.name.as_str());
    let machinery: Vec<&str> = own_name.into_iter().collect();
    !collect_for_function(function, own_name, &machinery, false)
        .triggers
        .is_empty()
}

fn machinery_names<'a>(program: &DecoderProgram<'a>) -> Vec<&'a str> {
    std::iter::once(program.string_array.name)
        .chain(program.decoders.iter().map(|decoder| decoder.name))
        .collect()
}

pub fn shape(program: &DecoderProgram<'_>) -> MachineryState {
    let machinery = machinery_names(program);
    let mut owned: Vec<(Owner, Collected<'_>)> = Vec::new();
    owned.push((
        Owner::Array,
        collect_for_function(
            program.string_array.function,
            Some(program.string_array.name),
            &machinery,
            program.string_array.strict,
        ),
    ));
    for (index, decoder) in program.decoders.iter().enumerate() {
        owned.push((
            Owner::Decoder(index),
            collect_for_function(
                decoder.function,
                Some(decoder.name),
                &machinery,
                decoder.strict,
            ),
        ));
    }
    if let Some(rotator) = program.rotator {
        if let Some(function) = rotator_function(rotator) {
            owned.push((
                Owner::Rotator,
                collect_for_rotator(function, &machinery, program.strict),
            ));
        }
    }

    let any_triggers = owned.iter().any(|(_, c)| !c.triggers.is_empty());
    if !any_triggers {
        return MachineryState::Stateless;
    }
    let resets = reset_gates(program, &owned);
    classify(program, owned, &resets)
}

fn reset_gates<'a>(
    program: &DecoderProgram<'a>,
    owned: &[(Owner, Collected<'a>)],
) -> Vec<reset::ResetGate<'a>> {
    program
        .decoders
        .iter()
        .enumerate()
        .filter_map(|(index, decoder)| {
            let decoder = reset::Decoder {
                function: decoder.function,
                name: decoder.name,
                index,
                string_array: program.string_array.name,
            };
            reset::find(&decoder, owned)
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Array,
    Decoder(usize),
    Rotator,
}

fn owner_of_name(program: &DecoderProgram<'_>, name: &str) -> Option<Owner> {
    if program.string_array.name == name {
        return Some(Owner::Array);
    }
    program
        .decoders
        .iter()
        .position(|d| d.name == name)
        .map(Owner::Decoder)
}

fn span_contains(region: Span, point: Span) -> bool {
    region.start <= point.start && point.end <= region.end
}

fn span_text(span: Span, source_text: &str) -> &str {
    source_text
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
        .trim()
}

fn matching_gate_region(
    collected: &Collected<'_>,
    peel: &MemberPeel<'_>,
    span: Span,
) -> Option<Span> {
    collected
        .gates
        .iter()
        .find(|gate| {
            gate.root == peel.root
                && gate.segment == peel.segment
                && span_contains(gate.region, span)
        })
        .map(|gate| gate.region)
}

fn matching_read(
    collected: &Collected<'_>,
    peel: &MemberPeel<'_>,
    key_text: &str,
    source_text: &str,
) -> bool {
    collected.accesses.iter().any(|access| {
        !access.is_write
            && access.peel.container_key() == peel.container_key()
            && matches!(access.peel.final_key, FinalKey::Computed(other) if span_text(other, source_text) == key_text)
    })
}

fn classify<'a>(
    program: &DecoderProgram<'a>,
    owned: Vec<(Owner, Collected<'a>)>,
    resets: &[reset::ResetGate<'a>],
) -> MachineryState {
    if let Some(reason) = refuse_on_non_member_triggers(&owned) {
        return MachineryState::Refused(reason);
    }
    if let Some(reason) = refuse_on_duplicate_gates(&owned) {
        return MachineryState::Refused(reason);
    }
    if !gates::gates_are_proven(&owned) {
        return MachineryState::Refused(&UNPROVEN_GENERIC);
    }
    if refuse_on_read_arguments_store(&owned) {
        return MachineryState::Refused(&UNPROVEN_GENERIC);
    }
    match classify_member_triggers(program, &owned, resets) {
        Ok(shape) if containers::uses_are_recognized(&owned) => MachineryState::Candidate(shape),
        Ok(_) => MachineryState::Refused(&UNPROVEN_GENERIC),
        Err(reason) => MachineryState::Refused(reason),
    }
}

fn refuse_on_non_member_triggers(
    owned: &[(Owner, Collected<'_>)],
) -> Option<&'static &'static str> {
    for (_, collected) in owned {
        for trigger in &collected.triggers {
            match &trigger.kind {
                TriggerKind::This => return Some(&UNPROVEN_THIS),
                TriggerKind::Identifier => return Some(&UNPROVEN_CLOSURE),
                TriggerKind::Unrecognized => return Some(&UNPROVEN_GENERIC),
                TriggerKind::Member(_) | TriggerKind::CapturedMemo(_) | TriggerKind::Monitored => {}
            }
        }
    }
    None
}

fn refuse_on_read_arguments_store(owned: &[(Owner, Collected<'_>)]) -> bool {
    owned.iter().any(|(_, collected)| {
        collected.arguments_stores.iter().any(|(root, key)| {
            owned.iter().any(|(_, other)| {
                other.accesses.iter().any(|access| {
                    !access.is_write
                        && access.peel.root == *root
                        && access.peel.segment.is_none()
                        && matches!(access.peel.final_key, FinalKey::Static(read) if read == *key)
                })
            })
        })
    })
}

fn refuse_on_duplicate_gates(owned: &[(Owner, Collected<'_>)]) -> Option<&'static &'static str> {
    let mut seen_flags: Vec<(&str, Option<&str>, &str)> = Vec::new();
    for (_, collected) in owned {
        for gate in &collected.gates {
            let key = (gate.root, gate.segment, gate.flag_key);
            if seen_flags.contains(&key) {
                return Some(&UNPROVEN_DUPLICATE_GATE);
            }
            seen_flags.push(key);
        }
    }
    None
}

fn reset_write(
    resets: &[reset::ResetGate<'_>],
    owner: Owner,
    span: Span,
    peel: &MemberPeel<'_>,
) -> Option<GateShape> {
    let found = resets
        .iter()
        .find(|gate| owner == Owner::Decoder(gate.owner) && gate.writes.contains(&span))?;
    let FinalKey::Static(key) = peel.final_key else {
        return None;
    };
    Some(GateShape {
        container: peel.container_span,
        key: PropKey::Name(JsString::from_str(key)),
        guard: found.region,
    })
}

fn reset_shapes(resets: &[reset::ResetGate<'_>]) -> Vec<ResetShape> {
    resets
        .iter()
        .map(|gate| ResetShape {
            guard: gate.region,
            memo: PropKey::Name(JsString::from_str(gate.memo_key)),
        })
        .collect()
}

fn classify_member_triggers<'a>(
    program: &DecoderProgram<'a>,
    owned: &[(Owner, Collected<'a>)],
    resets: &[reset::ResetGate<'a>],
) -> Result<Shape, &'static &'static str> {
    let mut gates_out = Vec::new();
    let mut memos_out = Vec::new();
    for (owner, collected) in owned {
        for trigger in &collected.triggers {
            match &trigger.kind {
                TriggerKind::Member(peel) => {
                    if let Some(gate) = reset_write(resets, *owner, trigger.span, peel) {
                        gates_out.push(gate);
                        continue;
                    }
                    classify_one_member(
                        program,
                        *owner,
                        collected,
                        trigger.span,
                        peel,
                        &mut gates_out,
                        &mut memos_out,
                    )?
                }
                TriggerKind::CapturedMemo(peel) => {
                    check_captured_memo(program, *owner, collected, peel)?
                }
                _ => {}
            }
        }
    }
    let used_params = params_for_memo_owners(program, &memos_out, params::used_params);
    let numeric_params = params_for_memo_owners(program, &memos_out, params::numeric_params);
    let captured_memo = owned.iter().any(|(_, collected)| {
        collected
            .triggers
            .iter()
            .any(|trigger| matches!(trigger.kind, TriggerKind::CapturedMemo(_)))
    });
    let memoised = captured_memo || !memos_out.is_empty();
    Ok(Shape {
        gates: gates_out,
        resets: reset_shapes(resets),
        memos: memos_out,
        used_params,
        numeric_params,
        conditional: gates::conditional_flags(owned),
        rewrites: template_rewrites(owned),
        captured_memo,
        miss_paths_proven: memoised && miss_path::proven(program, owned),
        gate_owners: gate_owners(owned),
    })
}

fn gate_owners(owned: &[(Owner, Collected<'_>)]) -> Vec<(Span, usize)> {
    owned
        .iter()
        .filter_map(|(owner, collected)| match owner {
            Owner::Decoder(index) => Some((*index, collected)),
            _ => None,
        })
        .flat_map(|(index, collected)| {
            let unconditional = collected.gates.iter().filter(|gate| !gate.conditional);
            unconditional.map(move |gate| (gate.region, index))
        })
        .collect()
}

fn template_rewrites(owned: &[(Owner, Collected<'_>)]) -> Vec<(Span, usize)> {
    owned
        .iter()
        .filter_map(|(owner, collected)| match owner {
            Owner::Decoder(index) => collected.rewrite.map(|span| (span, *index)),
            _ => None,
        })
        .collect()
}

fn params_for_memo_owners(
    program: &DecoderProgram<'_>,
    memos: &[MemoShape],
    judge: fn(&Function<'_>) -> Vec<bool>,
) -> Vec<Vec<bool>> {
    program
        .decoders
        .iter()
        .enumerate()
        .map(|(index, decoder)| {
            if memos.iter().any(|memo| memo.owner == index) {
                judge(decoder.function)
            } else {
                Vec::new()
            }
        })
        .collect()
}

fn classify_one_member(
    program: &DecoderProgram<'_>,
    owner: Owner,
    collected: &Collected<'_>,
    span: Span,
    peel: &MemberPeel<'_>,
    gates_out: &mut Vec<GateShape>,
    memos_out: &mut Vec<MemoShape>,
) -> Result<(), &'static &'static str> {
    let Some(container_owner) = owner_of_name(program, peel.root) else {
        return Err(&UNPROVEN_GENERIC);
    };
    if container_owner != owner {
        return Err(&UNPROVEN_CROSS);
    }
    if let Some(region) = matching_gate_region(collected, peel, span) {
        if let FinalKey::Static(text) = peel.final_key {
            gates_out.push(GateShape {
                container: peel.container_span,
                key: PropKey::Name(JsString::from_str(text)),
                guard: region,
            });
        }
        return Ok(());
    }
    let key = matched_memo_key(program, collected, peel)?;
    let Some(segment) = peel.segment else {
        return Err(&UNPROVEN_GENERIC);
    };
    let owner_index = match owner {
        Owner::Decoder(index) => index,
        _ => usize::MAX,
    };
    memos_out.push(MemoShape {
        container: peel.container_span,
        key,
        owner: owner_index,
        container_key: Some(PropKey::Name(JsString::from_str(segment))),
    });
    Ok(())
}

fn matched_memo_key(
    program: &DecoderProgram<'_>,
    collected: &Collected<'_>,
    peel: &MemberPeel<'_>,
) -> Result<Span, &'static &'static str> {
    let FinalKey::Computed(key) = peel.final_key else {
        return Err(&UNPROVEN_GENERIC);
    };
    let key_text = span_text(key, program.source_text);
    if !matching_read(collected, peel, key_text, program.source_text) {
        return Err(&UNPROVEN_GENERIC);
    }
    Ok(key)
}

fn check_captured_memo(
    program: &DecoderProgram<'_>,
    owner: Owner,
    collected: &Collected<'_>,
    peel: &MemberPeel<'_>,
) -> Result<(), &'static &'static str> {
    let Owner::Decoder(_) = owner else {
        return Err(&UNPROVEN_GENERIC);
    };
    matched_memo_key(program, collected, peel).map(|_| ())
}
