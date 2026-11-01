use untwist::{Options, Pipeline};

const A_ROTATOR: &str = concat!(
    include_str!("../../../fuzz/artifacts/evaluator/leak-3968f0c1984eb067dd5f0aa3136c4da24f3d77e8"),
    "console.log(b(1));\n"
);

fn options(fuel: Option<u64>) -> Options {
    Options {
        fuel,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

#[test]
fn the_default_fuel_decodes_the_call_site() {
    let out = Pipeline::new(options(None)).run(A_ROTATOR);
    assert!(
        out.code().contains("console.log(\"Hello World!\")"),
        "{}",
        out.code()
    );
}

#[test]
fn one_step_of_fuel_bails_the_decode_and_keeps_the_machinery() {
    let out = Pipeline::new(options(Some(1))).run(A_ROTATOR);
    assert!(
        !out.code().contains("console.log(\"Hello World!\")"),
        "{}",
        out.code()
    );
    assert!(
        out.notes
            .iter()
            .any(|note| note.message.contains("step budget exhausted")),
        "{:?}",
        out.notes
    );
}
