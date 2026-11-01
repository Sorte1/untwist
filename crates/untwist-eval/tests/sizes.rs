use std::mem::size_of;

use untwist_eval::bail::{Abrupt, Bail, EvalResult};
use untwist_eval::env::{Bindings, ScopeChainTooDeep};
use untwist_eval::frame::{Frame, FrameBody, FrameRef};
use untwist_eval::value::{Attrs, Eval, JsObject, JsString, PropKey, PropMap, StoredKey, Value};

const _: () = assert!(size_of::<JsObject<'static>>() == 104);

#[test]
fn an_object_carries_its_array_charge_in_one_more_word() {
    assert_eq!(size_of::<JsObject<'static>>(), 104);
}

#[test]
fn a_string_is_one_word_and_a_value_two() {
    assert_eq!(size_of::<JsString>(), 8);
    assert_eq!(size_of::<Value<'static>>(), 16);
    assert_eq!(size_of::<Option<Value<'static>>>(), 16);
}

#[test]
fn an_evaluation_result_is_two_words() {
    assert_eq!(size_of::<Abrupt<'static>>(), 8);
    assert_eq!(size_of::<Eval<'static, Value<'static>>>(), 16);
}

#[test]
fn a_bail_and_a_unit_result_are_two_words() {
    assert_eq!(size_of::<Bail>(), 16);
    assert_eq!(size_of::<EvalResult<()>>(), 16);
}

#[test]
fn a_scope_lookup_is_two_words_with_the_cap_type_and_three_with_a_bail() {
    assert_eq!(size_of::<Bail>(), 16);
    assert_eq!(size_of::<EvalResult<Option<Value<'static>>>>(), 24);
    assert_eq!(
        size_of::<Result<Option<Value<'static>>, ScopeChainTooDeep>>(),
        16
    );
}

#[test]
fn a_property_bucket_holds_its_attribute_bits_in_the_key_and_stays_five_words() {
    let mut props: PropMap<'static> = PropMap::default();
    props.insert(StoredKey::named("a", Attrs::CONSTANT), Value::Undefined);
    props.insert(PropKey::Index(1).into(), Value::Undefined);
    assert_eq!(size_of::<PropKey>(), 16);
    assert_eq!(size_of::<StoredKey>(), 16);
    assert_eq!(std::mem::size_of_val(props.as_slice()) / props.len(), 40);
}

#[test]
fn a_member_evaluation_is_four_words() {
    assert_eq!(size_of::<PropKey>(), 16);
    assert_eq!(size_of::<Eval<'static, (Value<'static>, PropKey)>>(), 32);
}

#[test]
fn a_tag_first_value_keeps_every_result_that_wraps_it_two_words() {
    assert_eq!(size_of::<Value<'static>>(), 16);
    assert_eq!(size_of::<Option<Value<'static>>>(), 16);
    assert_eq!(size_of::<Eval<'static, Value<'static>>>(), 16);
    assert_eq!(size_of::<Eval<'static, Option<Value<'static>>>>(), 16);
}

const SCOPE_BYTES_AT_739D527: usize = 64;
const BINDINGS_BYTES_AT_739D527: usize = 40;

#[test]
fn a_frame_body_is_no_larger_than_the_walkers_bindings() {
    assert_eq!(size_of::<Bindings<'static>>(), BINDINGS_BYTES_AT_739D527);
    assert!(size_of::<FrameBody<'static>>() <= size_of::<Bindings<'static>>());
}

#[test]
fn a_frame_is_no_larger_than_the_scope_it_replaces() {
    assert!(size_of::<Frame<'static>>() <= SCOPE_BYTES_AT_739D527);
    assert_eq!(size_of::<Option<FrameRef<'static>>>(), 8);
}

#[test]
fn a_closure_keeps_its_compile_memo_in_its_padding() {
    assert_eq!(size_of::<untwist_eval::value::Closure<'static>>(), 40);
}
