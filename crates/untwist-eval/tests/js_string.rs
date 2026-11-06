use oxc_allocator::Allocator;
use untwist_eval::bail::Bail;
use untwist_eval::value::{js_str_in, js_string_from_js_str, JsString};

#[test]
fn round_trips_plain_text() {
    let s = JsString::from_str("Hello World!");
    assert_eq!(s.len(), 12);
    assert_eq!(s.to_lossy_string(), "Hello World!");
    assert_eq!(s.as_string().as_deref(), Some("Hello World!"));
}

#[test]
fn length_is_code_units_not_scalars() {
    let s = JsString::from_str("\u{1F600}");
    assert_eq!(s.len(), 2);
    assert_eq!(s.code_unit_at(0), Some(0xD83D));
    assert_eq!(s.code_unit_at(1), Some(0xDE00));
    assert_eq!(s.code_unit_at(2), None);
}

#[test]
fn lone_surrogates_survive() {
    let s = JsString::from_units(vec![0xD800, 0x61, 0x62]);
    assert_eq!(s.units(), &[0xD800, 0x61, 0x62]);
    assert_eq!(s.as_string(), None);
    assert_eq!(s.to_lossy_string(), "\u{FFFD}ab");
}

#[test]
fn concat_builds_a_new_string() {
    let a = JsString::from_str("ab");
    let b = JsString::from_units(vec![0xDFFF]);
    let joined = a.concat(&b).expect("under cap");
    assert_eq!(joined.units(), &[0x61, 0x62, 0xDFFF]);
    assert_eq!(a.len(), 2);
}

#[test]
fn concat_respects_the_string_cap() {
    let big = JsString::from_units(vec![0x61; 16 << 20]);
    assert!(matches!(
        big.concat(&JsString::from_str("x")),
        Err(Bail::StringLength)
    ));
}

#[test]
fn lone_surrogates_round_trip_through_js_str() {
    let allocator = Allocator::default();
    let lone = js_str_in(&[0xD800, 0x61], &allocator);
    assert!(lone.has_lone_surrogate());
    assert_eq!(lone.encode_utf16().collect::<Vec<_>>(), vec![0xD800, 0x61]);
    assert_eq!(js_string_from_js_str(lone).units(), &[0xD800, 0x61]);

    let plain = js_str_in(&[0x61, 0x62], &allocator);
    assert_eq!(plain.as_str(), Some("ab"));

    let replacement = js_str_in(&[0xFFFD, 0x61], &allocator);
    assert_eq!(replacement.as_str(), Some("\u{FFFD}a"));
}

#[test]
fn surrogate_pairs_are_not_lone() {
    let allocator = Allocator::default();
    let pair = js_str_in(&[0xD83D, 0xDE00], &allocator);
    assert_eq!(pair.as_str(), Some("\u{1F600}"));
}
