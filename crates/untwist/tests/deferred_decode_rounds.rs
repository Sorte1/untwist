use untwist::{Options, Pipeline};

const MEDIUM_GREETING: &str =
    include_str!("../../../fixtures/obfuscator-io/preset-medium--greeting-1.js");
const MEDIUM_FUNCTIONS: &str =
    include_str!("../../../fixtures/obfuscator-io/preset-medium--functions-1.js");

fn unsafe_lane(source: &str) -> String {
    Pipeline::new(Options {
        unsafe_transforms: true,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

#[test]
fn a_decode_deferred_until_the_rounds_settle_still_runs() {
    for (label, source, decoder, decoded) in [
        ("greeting", MEDIUM_GREETING, "_0x31a4", "\"hello, \""),
        ("functions", MEDIUM_FUNCTIONS, "_0x45d3", "\"factorial\""),
    ] {
        let code = unsafe_lane(source);
        assert!(
            !code.contains(decoder),
            "{label}: the decoder is left:\n{code}"
        );
        assert!(
            code.contains(decoded),
            "{label}: {decoded} was not decoded:\n{code}"
        );
    }
}
