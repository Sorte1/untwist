use untwist::{Options, Pipeline};

const A_STRING_ARRAY_AND_A_MISSING_PROPERTY: &str = concat!(
    include_str!("../../../fuzz/artifacts/evaluator/leak-3968f0c1984eb067dd5f0aa3136c4da24f3d77e8"),
    "var o = { k: 1 };\nconsole.log(o.k);\nconsole.log(o.missing);\n"
);

fn options() -> Options {
    Options {
        unpack: false,
        jsx: false,
        unsafe_transforms: true,
        ..Options::default()
    }
}

#[test]
fn a_refusal_both_inline_object_props_registrations_emit_is_reported_once() {
    let out = Pipeline::new(options()).run(A_STRING_ARRAY_AND_A_MISSING_PROPERTY);
    let missing: Vec<_> = out
        .notes
        .iter()
        .filter(|note| note.message.contains("property missing is not defined"))
        .collect();
    assert_eq!(missing.len(), 1, "{:?}", out.notes);
    assert!(missing[0].span.is_some(), "{:?}", missing[0]);
}

#[test]
fn under_conservative_the_single_registration_still_reports_it_once() {
    let out = Pipeline::new(Options {
        extended: false,
        ..options()
    })
    .run(A_STRING_ARRAY_AND_A_MISSING_PROPERTY);
    let missing = out
        .notes
        .iter()
        .filter(|note| note.message.contains("property missing is not defined"))
        .count();
    assert_eq!(missing, 1, "{:?}", out.notes);
}

#[test]
fn two_identical_inputs_keep_their_own_copies() {
    let out = Pipeline::new(options()).run_many(&[
        A_STRING_ARRAY_AND_A_MISSING_PROPERTY,
        A_STRING_ARRAY_AND_A_MISSING_PROPERTY,
    ]);
    let missing = out
        .notes
        .iter()
        .filter(|note| note.message.contains("property missing is not defined"))
        .count();
    assert_eq!(missing, 2, "{:?}", out.notes);
}
