use untwist::{Options, Output, Pipeline};

const HIGH: &str = include_str!("../../../fixtures/reference/deobfuscate/obfuscator.io-high.js");
const MULTI_ENCODERS: &str =
    include_str!("../../../fixtures/reference/deobfuscate/obfuscator.io-multi-encoders.js");

const CORE_JS_STYLE_POLYFILL: &str = "\
(function () {
  var defineBuiltIn = function (O, key, value) { O[key] = value; return O; };
  if (!String.prototype.at) { defineBuiltIn(String.prototype, 'at', function (n) { return this.charAt(n); }); }
  defineBuiltIn(Object.prototype, 'toString', Object.prototype.toString);
})();
";

const HIGH_MACHINERY: [&str; 2] = ["_0x4c3e", "_0x4d6b"];
const MULTI_MACHINERY: [&str; 4] = ["function a(", "function b(", "function c(", "function d("];

const ACCEPTED_EARLY_REFUSAL: &str =
    "a stateful decoder is referenced outside the calls untwist proves";

fn modes() -> [(&'static str, Options); 2] {
    [
        ("default", Options::default()),
        (
            "conservative",
            Options {
                extended: false,
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
    ]
}

fn decode_notes(output: &Output) -> Vec<&str> {
    output
        .notes
        .iter()
        .filter(|note| note.pass == "inline-decoded-strings")
        .map(|note| note.message.as_str())
        .collect()
}

fn assert_machinery_gone(label: &str, code: &str, machinery: &[&str]) {
    for name in machinery {
        assert!(
            !code.contains(name),
            "{label}: `{name}` is left in the output"
        );
    }
}

fn assert_machinery_kept_under_name_observation(label: &str, output: &Output) {
    assert!(
        output
            .notes
            .iter()
            .any(|note| note.message == untwist_deobfuscate::notes::MACHINERY_NAME_OBSERVED),
        "{label}: the machinery was not kept under name observation"
    );
}

fn assert_no_kept_machinery_note(label: &str, output: &Output) {
    assert!(
        !output
            .notes
            .iter()
            .any(|note| note.message == untwist_deobfuscate::notes::MACHINERY_NAME_OBSERVED),
        "{label}: the report says the machinery was kept, but the output removed it"
    );
}

fn assert_high_decodes_fully(label: &str, source: &str, observed_in_conservative: bool) {
    for (mode, options) in modes() {
        let output = Pipeline::new(options).run(source);
        let label_mode = format!("{label} ({mode})");
        if mode == "default" || observed_in_conservative {
            assert_machinery_kept_under_name_observation(&label_mode, &output);
        } else {
            assert_machinery_gone(&label_mode, output.code(), &HIGH_MACHINERY);
            assert_no_kept_machinery_note(&label_mode, &output);
        }
        let refusals: Vec<&str> = decode_notes(&output)
            .into_iter()
            .filter(|note| !note.ends_with(ACCEPTED_EARLY_REFUSAL))
            .collect();
        assert!(refusals.is_empty(), "{label} ({mode}): {refusals:?}");
    }
}

#[test]
fn the_high_preset_decodes_fully_in_both_modes() {
    assert_high_decodes_fully("high", HIGH, false);
}

#[test]
fn the_high_preset_beside_a_core_js_style_polyfill_decodes_fully_in_both_modes() {
    let source = format!("{HIGH}\n{CORE_JS_STYLE_POLYFILL}");
    assert_high_decodes_fully("high + polyfill", &source, true);
}

#[test]
fn the_multi_encoder_units_decode_fully_in_both_modes() {
    for (mode, options) in modes() {
        let output = Pipeline::new(options).run(MULTI_ENCODERS);
        assert_machinery_gone(mode, output.code(), &MULTI_MACHINERY);
        assert!(
            output.code().contains("console.log(\"Hello World!\")"),
            "{mode}: {}",
            output.code()
        );
        assert_eq!(decode_notes(&output), Vec::<&str>::new(), "{mode}");
    }
}
