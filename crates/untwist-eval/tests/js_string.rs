use untwist_eval::bail::Bail;
use untwist_eval::value::{decode_lone_surrogate_escapes, encode_with_lone_surrogates, JsString};

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
fn lone_surrogate_escape_scheme_matches_oxc() {
    let (text, lone) = encode_with_lone_surrogates(&[0xD800, 0x61]);
    assert!(lone);
    assert_eq!(text, "\u{FFFD}d800a");
    assert_eq!(decode_lone_surrogate_escapes(&text), vec![0xD800, 0x61]);

    let (plain, no_lone) = encode_with_lone_surrogates(&[0x61, 0x62]);
    assert!(!no_lone);
    assert_eq!(plain, "ab");

    let (replacement, lone_marker) = encode_with_lone_surrogates(&[0xFFFD, 0x61]);
    assert!(lone_marker);
    assert_eq!(replacement, "\u{FFFD}fffda");
    assert_eq!(
        decode_lone_surrogate_escapes(&replacement),
        vec![0xFFFD, 0x61]
    );
}

#[test]
fn surrogate_pairs_are_not_escaped() {
    let (text, lone) = encode_with_lone_surrogates(&[0xD83D, 0xDE00]);
    assert!(!lone);
    assert_eq!(text, "\u{1F600}");
}
