use untwist_eval::coerce::number_to_string;
use untwist_eval::ops::string_key;
use untwist_eval::value::{JsString, PropKey};

fn reference_string_key(text: JsString) -> PropKey {
    if let Some(rendered) = text.as_string() {
        if let Ok(index) = rendered.parse::<u32>() {
            if number_to_string(f64::from(index)) == rendered {
                return PropKey::Index(index);
            }
        }
    }
    PropKey::Name(text)
}

const CASES: [&str; 20] = [
    "",
    "0",
    "1",
    "42",
    "007",
    "+5",
    "-1",
    "4294967295",
    "4294967296",
    "99999999999",
    "1.0",
    "1e3",
    "length",
    "prototype",
    " 1",
    "1 ",
    "٣",
    "0x10",
    "00",
    "10",
];

#[test]
fn string_key_agrees_with_the_string_round_trip_it_replaces() {
    for case in CASES {
        let text = JsString::from_str(case);
        assert_eq!(
            string_key(text.clone()),
            reference_string_key(text),
            "{case:?}"
        );
    }
    let lone = JsString::from_units(vec![0xD800, u16::from(b'1')]);
    assert_eq!(string_key(lone.clone()), reference_string_key(lone));
}
