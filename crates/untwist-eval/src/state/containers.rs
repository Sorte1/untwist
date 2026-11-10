use super::collect::{Allowance, BareUse, Collected, FinalKey, MemberPeel, TriggerKind};
use super::Owner;

fn written<'c, 'a>(
    owned: &'c [(Owner, Collected<'a>)],
) -> impl Iterator<Item = &'c MemberPeel<'a>> + 'c {
    owned
        .iter()
        .flat_map(|(_, collected)| collected.triggers.iter())
        .filter_map(|trigger| match &trigger.kind {
            TriggerKind::Member(peel) => Some(peel),
            _ => None,
        })
}

fn is_container(owned: &[(Owner, Collected<'_>)], name: &str) -> bool {
    let gated = owned
        .iter()
        .flat_map(|(_, collected)| collected.gates.iter())
        .any(|gate| gate.root == name);
    gated || written(owned).any(|peel| peel.root == name)
}

fn same_key(written: FinalKey<'_>, read: FinalKey<'_>) -> bool {
    match (written, read) {
        (FinalKey::Static(left), FinalKey::Static(right)) => left == right,
        (FinalKey::Computed(_), FinalKey::Computed(_)) => true,
        _ => false,
    }
}

fn read_is_recognized(owned: &[(Owner, Collected<'_>)], read: &MemberPeel<'_>) -> bool {
    if read.segment.is_none() && matches!(read.final_key, FinalKey::Computed(_)) {
        return false;
    }
    written(owned).any(|peel| {
        peel.container_key() == read.container_key() && same_key(peel.final_key, read.final_key)
    })
}

fn receiver_is_recognized(owned: &[(Owner, Collected<'_>)], root: &str, key: &str) -> bool {
    let mut writes = owned
        .iter()
        .flat_map(|(_, collected)| collected.function_writes.iter())
        .filter(|write| write.root == root && write.key == key)
        .peekable();
    writes.peek().is_some() && writes.all(|write| write.function)
}

fn stored_values_are_settled(owned: &[(Owner, Collected<'_>)]) -> bool {
    crate::mutants::gate_state_trusted()
        || owned
            .iter()
            .flat_map(|(_, collected)| collected.function_writes.iter())
            .all(|write| write.settled)
}

fn after_every_gate(
    owned: &[(Owner, Collected<'_>)],
    home: &Collected<'_>,
    name: &str,
    at: u32,
) -> bool {
    let conditional = owned
        .iter()
        .flat_map(|(_, collected)| collected.gates.iter())
        .any(|gate| gate.root == name && gate.conditional);
    let elsewhere = owned
        .iter()
        .filter(|(_, collected)| !std::ptr::eq(collected, home))
        .flat_map(|(_, collected)| collected.gates.iter())
        .any(|gate| gate.root == name);
    !conditional
        && !elsewhere
        && home
            .gates
            .iter()
            .filter(|gate| gate.root == name)
            .all(|gate| at >= gate.region.end)
}

fn use_is_recognized(
    owned: &[(Owner, Collected<'_>)],
    home: &Collected<'_>,
    use_: &BareUse<'_>,
) -> bool {
    match use_.allowance {
        Allowance::Always => true,
        Allowance::AfterGates(at) => {
            is_container(owned, use_.name) && after_every_gate(owned, home, use_.name, at)
        }
        Allowance::Never => false,
    }
}

fn names_are_recognized(owned: &[(Owner, Collected<'_>)], collected: &Collected<'_>) -> bool {
    collected
        .bare_uses
        .iter()
        .all(|use_| use_is_recognized(owned, collected, use_))
        && collected.receivers.iter().all(|(root, key)| {
            !is_container(owned, root) || receiver_is_recognized(owned, root, key)
        })
}

pub(super) fn uses_are_recognized(owned: &[(Owner, Collected<'_>)]) -> bool {
    stored_values_are_settled(owned)
        && owned.iter().all(|(_, collected)| {
            names_are_recognized(owned, collected)
                && collected
                    .accesses
                    .iter()
                    .filter(|access| {
                        !access.is_write && !access.local && is_container(owned, access.peel.root)
                    })
                    .all(|access| read_is_recognized(owned, &access.peel))
        })
}
