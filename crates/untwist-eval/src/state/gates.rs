use oxc_span::Span;

use crate::value::{JsString, PropKey};

use super::collect::{Collected, FinalKey, MemberPeel, RecognizedGate, TriggerKind};
use super::{span_contains, Owner};

fn on_container(gate: &RecognizedGate<'_>, peel: &MemberPeel<'_>) -> bool {
    peel.root == gate.root && peel.segment == gate.segment
}

fn written_keys<'a>(collected: &Collected<'a>, gate: &RecognizedGate<'a>) -> Option<Vec<&'a str>> {
    let mut keys = vec![gate.flag_key];
    for trigger in &collected.triggers {
        if !span_contains(gate.region, trigger.span) {
            continue;
        }
        if let TriggerKind::Member(peel) = &trigger.kind {
            if on_container(gate, peel) {
                match peel.final_key {
                    FinalKey::Static(key) => keys.push(key),
                    FinalKey::Computed(_) => return None,
                }
            }
        }
    }
    Some(keys)
}

fn writes_only_its_flag(collected: &Collected<'_>, gate: &RecognizedGate<'_>) -> bool {
    let inside = |span: Span| span_contains(gate.region, span);
    let triggers_ok = collected
        .triggers
        .iter()
        .filter(|trigger| inside(trigger.span))
        .all(|trigger| match &trigger.kind {
            TriggerKind::Monitored => true,
            TriggerKind::Member(peel) => {
                on_container(gate, peel)
                    && matches!(peel.final_key, FinalKey::Static(key) if key == gate.flag_key)
            }
            _ => false,
        });
    triggers_ok
        && collected.arguments_stores.is_empty()
        && !collected.captures.iter().any(|span| inside(*span))
}

fn read_is_allowed(gate: &RecognizedGate<'_>, at: u32, same_function: bool) -> bool {
    if !same_function {
        return false;
    }
    let in_test = gate.test.start <= at && at < gate.test.end;
    in_test || (!gate.conditional && at >= gate.region.start)
}

fn reads_are_ordered(
    owned: &[(Owner, Collected<'_>)],
    home: usize,
    gate: &RecognizedGate<'_>,
    keys: &[&str],
) -> bool {
    owned.iter().enumerate().all(|(index, (_, collected))| {
        collected
            .accesses
            .iter()
            .filter(|access| !access.is_write && !access.local && on_container(gate, &access.peel))
            .all(|access| match access.peel.final_key {
                FinalKey::Static(key) if !keys.contains(&key) => true,
                FinalKey::Static(_) => read_is_allowed(gate, access.at, index == home),
                FinalKey::Computed(_) => false,
            })
    })
}

fn gate_is_proven(
    owned: &[(Owner, Collected<'_>)],
    home: usize,
    collected: &Collected<'_>,
    gate: &RecognizedGate<'_>,
) -> bool {
    if gate.conditional && !writes_only_its_flag(collected, gate) {
        return false;
    }
    match written_keys(collected, gate) {
        Some(keys) => reads_are_ordered(owned, home, gate, &keys),
        None => false,
    }
}

pub(super) fn gates_are_proven(owned: &[(Owner, Collected<'_>)]) -> bool {
    owned.iter().enumerate().all(|(home, (_, collected))| {
        collected
            .gates
            .iter()
            .all(|gate| gate_is_proven(owned, home, collected, gate))
    })
}

pub(super) fn conditional_flags(owned: &[(Owner, Collected<'_>)]) -> Vec<(Span, PropKey)> {
    owned
        .iter()
        .flat_map(|(_, collected)| collected.gates.iter())
        .filter(|gate| gate.conditional)
        .map(|gate| {
            (
                gate.region,
                PropKey::Name(JsString::from_str(gate.flag_key)),
            )
        })
        .collect()
}
