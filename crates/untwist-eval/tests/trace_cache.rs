use untwist_eval::object::{
    note_intrinsic, set_recording, take_trace, trace_cache_owns_its_keys, trace_slot, Consulted,
};
use untwist_eval::value::{IntrinsicProto, JsString, PropKey};

const CANDIDATES: usize = 1 << 16;

fn key(text: &str) -> PropKey {
    PropKey::Name(JsString::from_str(text))
}

fn traced(reads: &[(IntrinsicProto, Consulted)], text: &str) -> bool {
    reads.iter().any(|(proto, what)| {
        *proto == IntrinsicProto::String
            && matches!(what, Consulted::Key(PropKey::Name(name)) if name.eq_str(text))
    })
}

fn same_slot_twin(home: usize, spare: &mut Vec<PropKey>) -> PropKey {
    for _ in 0..CANDIDATES {
        let candidate = key("ab");
        if trace_slot(IntrinsicProto::String, &candidate) == home {
            return candidate;
        }
        spare.push(candidate);
    }
    panic!("no content-equal key fell in the slot within {CANDIDATES} tries");
}

#[test]
fn a_content_hit_leaves_the_cache_owning_the_key_its_pointer_names() {
    set_recording(true);
    let first = key("ab");
    let home = trace_slot(IntrinsicProto::String, &first);
    note_intrinsic(IntrinsicProto::String, &first);
    drop(first);
    let mut spare = Vec::new();
    let twin = same_slot_twin(home, &mut spare);
    note_intrinsic(IntrinsicProto::String, &twin);
    let owned = trace_cache_owns_its_keys();
    let stale = trace_slot(IntrinsicProto::String, &twin);
    drop(twin);
    drop(spare);
    let fresh = key("cd");
    let reused = trace_slot(IntrinsicProto::String, &fresh) == stale;
    note_intrinsic(IntrinsicProto::String, &fresh);
    let reads = take_trace();
    set_recording(false);
    assert!(owned, "the entry names a key it does not hold");
    assert!(traced(&reads, "ab"), "{reads:?}");
    assert!(!reused || traced(&reads, "cd"), "{reads:?}");
}

#[test]
fn content_equal_keys_share_a_trace_slot_wherever_they_live() {
    let keys: Vec<PropKey> = (0..64).map(|_| key("charCodeAt")).collect();
    let home = trace_slot(IntrinsicProto::String, &keys[0]);
    let moved = keys
        .iter()
        .filter(|candidate| trace_slot(IntrinsicProto::String, candidate) != home)
        .count();
    assert_eq!(moved, 0, "a key's trace slot depends on its address");
}
