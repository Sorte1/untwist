use std::cell::RefCell;
use std::rc::Rc;

use untwist_eval::bail::Bail;
use untwist_eval::budget::ARRAY_LEN_CAP;
use untwist_eval::object;
use untwist_eval::object::{
    array_items, array_length, array_push, array_rotate_left, array_shift, lookup_property,
    own_property, set_property, string_property, PROTO_CHAIN_CAP,
};
use untwist_eval::value::{JsObject, JsString, NativeId, ObjKind, ObjRef, PropKey, PropMap, Value};

fn array(items: Vec<Value<'static>>) -> ObjRef<'static> {
    Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Array(items),
        props: PropMap::default(),
        proto: None,
        charge: None,
    }))
}

fn name(text: &str) -> PropKey {
    PropKey::Name(JsString::from_str(text))
}

fn plain(proto: Option<ObjRef<'static>>) -> ObjRef<'static> {
    Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Plain(0),
        props: PropMap::default(),
        proto,
        charge: None,
    }))
}

fn chain_with_property_at_depth(
    depth: usize,
    key: &PropKey,
    value: Value<'static>,
) -> ObjRef<'static> {
    let base = plain(None);
    base.borrow_mut().props.insert(key.clone().into(), value);
    let mut current = base;
    for _ in 0..depth {
        current = plain(Some(current));
    }
    current
}

#[test]
fn push_shift_rotates_in_place_like_the_rotator_needs() {
    let arr = array(vec![
        Value::string("a"),
        Value::string("b"),
        Value::string("c"),
    ]);
    let alias = arr.clone();
    let head = array_shift(&arr).expect("array");
    array_push(&arr, head).unwrap();
    let first = own_property(&alias, &PropKey::Index(0));
    assert!(matches!(&first, Some(Value::Str(s)) if s.to_lossy_string() == "b"));
    assert_eq!(array_length(&alias), Some(3));
}

#[test]
fn array_rotate_left_moves_the_front_elements_to_the_back_in_one_step() {
    let arr = array(vec![
        Value::string("a"),
        Value::string("b"),
        Value::string("c"),
        Value::string("d"),
    ]);
    array_rotate_left(&arr, 1).expect("array");
    let items = array_items(&arr).expect("array");
    let rendered: Vec<String> = items
        .iter()
        .map(|value| match value {
            Value::Str(s) => s.to_lossy_string(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(rendered, vec!["b", "c", "d", "a"]);
}

#[test]
fn array_rotate_left_wraps_the_step_count_and_tolerates_empty_arrays() {
    let arr = array(vec![Value::string("a"), Value::string("b")]);
    array_rotate_left(&arr, 5).expect("array");
    let items = array_items(&arr).expect("array");
    assert!(matches!(&items[0], Value::Str(s) if s.to_lossy_string() == "b"));

    let empty = array(Vec::new());
    assert!(array_rotate_left(&empty, 3).is_some());
    assert_eq!(array_length(&empty), Some(0));
}

#[test]
fn out_of_range_index_reads_nothing_and_shift_of_empty_is_undefined() {
    let arr = array(Vec::new());
    assert!(own_property(&arr, &PropKey::Index(7)).is_none());
    assert!(matches!(array_shift(&arr), Some(Value::Undefined)));
}

#[test]
fn writing_past_the_end_grows_with_holes() {
    let arr = array(vec![Value::Number(1.0)]);
    set_property(&arr, PropKey::Index(3), Value::Number(4.0)).unwrap();
    assert_eq!(array_length(&arr), Some(4));
    assert!(matches!(
        own_property(&arr, &PropKey::Index(2)),
        Some(Value::Undefined)
    ));
}

#[test]
fn length_is_readable_and_writable() {
    let arr = array(vec![Value::Number(1.0), Value::Number(2.0)]);
    assert!(matches!(own_property(&arr, &name("length")), Some(Value::Number(n)) if n == 2.0));
    set_property(&arr, name("length"), Value::Number(0.0)).unwrap();
    assert_eq!(array_length(&arr), Some(0));
}

#[test]
fn functions_are_property_bags() {
    let func = Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Native(NativeId(0), 0),
        props: PropMap::default(),
        proto: None,
        charge: None,
    }));
    set_property(&func, name("HujJww"), Value::Bool(true)).unwrap();
    assert!(matches!(
        own_property(&func, &name("HujJww")),
        Some(Value::Bool(true))
    ));
}

#[test]
fn prototype_chain_is_walked() {
    let base = Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Plain(0),
        props: PropMap::default(),
        proto: None,
        charge: None,
    }));
    set_property(&base, name("tlFPzk"), Value::Number(1.0)).unwrap();
    let derived = Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Plain(0),
        props: PropMap::default(),
        proto: Some(base),
        charge: None,
    }));
    assert!(own_property(&derived, &name("tlFPzk")).is_none());
    assert!(matches!(
        lookup_property(&derived, &name("tlFPzk")),
        Ok(Some(Value::Number(n))) if n == 1.0
    ));
}

#[test]
fn primitive_strings_expose_length_and_indexes() {
    let text = JsString::from_str("ab");
    let length: Option<Value<'static>> = string_property(&text, &name("length"));
    assert!(matches!(length, Some(Value::Number(n)) if n == 2.0));
    let unit: Option<Value<'static>> = string_property(&text, &PropKey::Index(1));
    assert!(matches!(unit, Some(Value::Str(s)) if s.to_lossy_string() == "b"));
    let missing: Option<Value<'static>> = string_property(&text, &PropKey::Index(9));
    assert!(missing.is_none());
}

#[test]
fn lone_surrogate_strings_expose_length_and_raw_units() {
    let text = JsString::from_units(vec![0xD800]);
    assert!(matches!(
        string_property(&text, &name("length")),
        Some(Value::Number(n)) if n == 1.0
    ));
    let unit = string_property(&text, &PropKey::Index(0));
    assert!(matches!(&unit, Some(Value::Str(s)) if s.units() == [0xD800]));
}

#[test]
fn array_items_snapshots_dense_storage_and_none_for_non_arrays() {
    let arr = array(vec![Value::Number(1.0), Value::Number(2.0)]);
    let items = array_items(&arr).expect("array");
    assert!(
        matches!(items.as_slice(), [Value::Number(a), Value::Number(b)] if *a == 1.0 && *b == 2.0)
    );

    let func = Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Native(NativeId(0), 0),
        props: PropMap::default(),
        proto: None,
        charge: None,
    }));
    assert!(array_items(&func).is_none());
}

#[test]
fn length_write_extends_with_undefined_holes() {
    let arr = array(vec![Value::Number(1.0)]);
    set_property(&arr, name("length"), Value::Number(4.0)).unwrap();
    assert_eq!(array_length(&arr), Some(4));
    for index in 1..4 {
        assert!(matches!(
            own_property(&arr, &PropKey::Index(index)),
            Some(Value::Undefined)
        ));
    }
}

#[test]
fn prototype_chain_lookup_finds_the_last_hop_within_the_cap() {
    let key = name("cwjyEo");
    let deepest_allowed =
        chain_with_property_at_depth(PROTO_CHAIN_CAP - 1, &key, Value::Number(1.0));
    assert!(matches!(
        lookup_property(&deepest_allowed, &key),
        Ok(Some(Value::Number(n))) if n == 1.0
    ));

    let one_hop_too_deep = chain_with_property_at_depth(PROTO_CHAIN_CAP, &key, Value::Number(1.0));
    assert!(matches!(
        lookup_property(&one_hop_too_deep, &key),
        Err(Bail::ChainTooDeep(&"prototype"))
    ));
}

#[test]
fn prototype_chain_cycles_terminate_instead_of_looping_forever() {
    let cyclic = plain(None);
    cyclic.borrow_mut().proto = Some(cyclic.clone());
    assert!(matches!(
        lookup_property(&cyclic, &name("missing")),
        Err(Bail::ChainTooDeep(&"prototype"))
    ));
}

#[test]
fn indexes_at_or_beyond_the_array_len_cap_bail_and_leave_the_array_unchanged() {
    let arr = array(Vec::new());

    let cap_index = u32::try_from(ARRAY_LEN_CAP).expect("cap fits in u32");
    assert_eq!(
        set_property(&arr, PropKey::Index(cap_index), Value::Number(1.0)),
        Err(Bail::ArrayLength)
    );
    assert_eq!(
        set_property(&arr, PropKey::Index(u32::MAX), Value::Number(2.0)),
        Err(Bail::ArrayLength)
    );

    assert_eq!(array_length(&arr), Some(0));
    assert!(array_items(&arr).expect("array").is_empty());
}

#[test]
fn a_length_write_beyond_the_array_len_cap_bails_and_leaves_the_array_unchanged() {
    let arr = array(vec![Value::Number(1.0)]);

    assert_eq!(
        set_property(&arr, name("length"), Value::Number(4_294_967_294.0)),
        Err(Bail::ArrayLength)
    );

    assert_eq!(array_length(&arr), Some(1));
    let items = array_items(&arr).expect("array");
    assert!(matches!(items.as_slice(), [Value::Number(n)] if *n == 1.0));
}

#[test]
fn array_push_at_the_cap_bails_instead_of_growing_further() {
    let arr = array(Vec::new());
    let cap = f64::from(u32::try_from(ARRAY_LEN_CAP).expect("cap fits in u32"));
    set_property(&arr, name("length"), Value::Number(cap)).unwrap();
    assert_eq!(array_length(&arr), Some(ARRAY_LEN_CAP));

    assert_eq!(array_push(&arr, Value::Number(1.0)), Err(Bail::ArrayLength));
    assert_eq!(array_length(&arr), Some(ARRAY_LEN_CAP));
}

fn prop_keys(obj: &ObjRef<'static>) -> Vec<PropKey> {
    obj.borrow().props.keys().map(|key| key.key()).collect()
}

#[test]
fn insertion_order_survives_a_mixed_sequence_with_a_re_insert_in_place() {
    let obj = plain(None);
    set_property(&obj, name("b"), Value::Number(1.0)).unwrap();
    set_property(&obj, PropKey::Index(3), Value::Number(2.0)).unwrap();
    set_property(&obj, name("a"), Value::Number(3.0)).unwrap();
    set_property(&obj, name("b"), Value::Number(4.0)).unwrap();

    assert_eq!(
        prop_keys(&obj),
        vec![name("b"), PropKey::Index(3), name("a")]
    );
    assert!(matches!(
        own_property(&obj, &name("b")),
        Some(Value::Number(n)) if n == 4.0
    ));
}

#[test]
fn deleting_then_reinserting_a_key_moves_it_to_the_end() {
    let obj = plain(None);
    set_property(&obj, name("a"), Value::Number(1.0)).unwrap();
    set_property(&obj, name("b"), Value::Number(2.0)).unwrap();
    set_property(&obj, name("c"), Value::Number(3.0)).unwrap();

    obj.borrow_mut().props.shift_remove(&name("a"));
    set_property(&obj, name("a"), Value::Number(4.0)).unwrap();

    assert_eq!(prop_keys(&obj), vec![name("b"), name("c"), name("a")]);
}

#[test]
fn enumeration_of_many_keys_matches_insertion_order_across_reallocations() {
    let obj = plain(None);
    let inserted: Vec<PropKey> = (0..200).map(|i| name(&format!("k{i}"))).collect();
    for key in &inserted {
        set_property(&obj, key.clone(), Value::Number(1.0)).unwrap();
    }

    assert_eq!(prop_keys(&obj), inserted);
}

fn chain_of(links: usize) -> ObjRef<'static> {
    let mut current = plain(None);
    for _ in 1..links {
        current = plain(Some(current));
    }
    current
}

#[test]
fn a_missing_key_walks_exactly_the_prototype_cap() {
    let missing = name("missing");
    let within = chain_of(PROTO_CHAIN_CAP);
    assert!(matches!(lookup_property(&within, &missing), Ok(None)));
    let beyond = chain_of(PROTO_CHAIN_CAP + 1);
    assert!(matches!(
        lookup_property(&beyond, &missing),
        Err(Bail::ChainTooDeep(&"prototype"))
    ));
}

#[test]
fn an_own_key_shadows_the_same_key_on_the_prototype() {
    let key = name("k");
    let parent = plain(None);
    parent
        .borrow_mut()
        .props
        .insert(key.clone().into(), Value::Number(1.0));
    let child = plain(Some(parent));
    assert!(matches!(lookup_property(&child, &key), Ok(Some(Value::Number(n))) if n == 1.0));
    child
        .borrow_mut()
        .props
        .insert(key.clone().into(), Value::Number(2.0));
    assert!(matches!(lookup_property(&child, &key), Ok(Some(Value::Number(n))) if n == 2.0));
}

#[test]
fn realm_roots_stay_under_half_the_root_table() {
    let count = untwist_eval::builtins::all_root_names().len();
    assert!(
        count < object::ROOT_TABLE_SLOTS / 2,
        "{count} roots against a {}-slot table leaves too little headroom for open addressing",
        object::ROOT_TABLE_SLOTS,
    );
    assert!(count <= object::MAX_ROOT_REGISTRATIONS);
}

#[test]
fn a_lookup_for_an_unregistered_address_probes_a_bounded_number_of_slots_and_misses() {
    object::reset_root_table();
    for index in 0..64u8 {
        object::register_root_address(0x1000 + index as usize * 8, index);
    }
    for probe in 0..1024usize {
        let address = 0x9000_0000 + probe * 8;
        assert_eq!(object::lookup_root_address(address), None);
    }
    object::reset_root_table();
}
