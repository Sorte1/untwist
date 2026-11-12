use untwist::{detect_obfuscation, Obfuscation, Options, Pipeline, TechniqueState};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/obfuscator-io/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn detected(source: &str) -> Obfuscation {
    detect_obfuscation(&Pipeline::new(Options::default()).run(source))
}

fn detail(found: &Obfuscation, name: &str) -> Option<String> {
    found
        .techniques
        .iter()
        .find(|technique| technique.name == name)
        .and_then(|technique| technique.detail.clone())
}

#[test]
fn an_rc4_string_array_is_labelled_with_its_encoding() {
    let found = detected(&fixture("encoding-rc4--control-1.js"));
    assert_eq!(found.families, ["javascript-obfuscator"], "{found:?}");
    assert!(found.has("string-array"), "{found:?}");
    assert!(found.has("string-array-rotate"), "{found:?}");
    let decoders = detail(&found, "string-array-decoders").unwrap_or_default();
    assert!(decoders.ends_with("rc4"), "{found:?}");
}

#[test]
fn a_base64_string_array_is_labelled_with_its_encoding() {
    let found = detected(&fixture("encoding-base64--control-1.js"));
    let decoders = detail(&found, "string-array-decoders").unwrap_or_default();
    assert!(decoders.ends_with("base64"), "{found:?}");
    assert!(!decoders.contains("rc4"), "{found:?}");
}

#[test]
fn mixed_encodings_name_both_decoders() {
    let found = detected(&fixture("encoding-mixed--control-1.js"));
    let decoders = detail(&found, "string-array-decoders").unwrap_or_default();
    assert!(
        decoders.contains("rc4") && decoders.contains("base64"),
        "{found:?}"
    );
}

#[test]
fn a_plain_index_decoder_names_no_encoding() {
    let found = detected(&fixture("preset-low--control-1.js"));
    let decoders = detail(&found, "string-array-decoders").unwrap_or_default();
    assert!(
        !decoders.contains("rc4") && !decoders.contains("base64"),
        "{found:?}"
    );
}

#[test]
fn every_technique_comes_from_a_pass_that_fired() {
    let rows = [
        ("cff-1--control-1.js", "control-flow-storage"),
        ("dead-code-1--control-1.js", "dead-code-injection"),
        ("self-defending--control-1.js", "self-defending"),
        ("debug-protection--control-1.js", "debug-protection"),
        ("calls-transform-1--control-1.js", "proxy-objects"),
        ("split-strings-2--control-1.js", "split-strings"),
        ("wrappers-2--control-1.js", "decoder-wrappers"),
    ];
    for (name, technique) in rows {
        let found = detected(&fixture(name));
        assert!(found.has(technique), "{name}: {found:?}");
        assert!(
            found
                .families
                .iter()
                .any(|family| family == "javascript-obfuscator"),
            "{name}: {found:?}"
        );
    }
}

#[test]
fn a_fully_undone_string_array_is_reported_undone() {
    let found = detected(&fixture("encoding-rc4--control-1.js"));
    let array = found
        .techniques
        .iter()
        .find(|technique| technique.name == "string-array")
        .expect("string array");
    assert_eq!(array.state, TechniqueState::Undone, "{found:?}");
}

#[test]
fn kept_machinery_reports_the_string_array_partly_undone() {
    let found = detected(&fixture("preset-high--control-1.js"));
    let array = found
        .techniques
        .iter()
        .find(|technique| technique.name == "string-array")
        .expect("string array");
    assert_eq!(array.state, TechniqueState::Partly, "{found:?}");
}

#[test]
fn obfuscator_output_without_a_string_array_names_no_string_array() {
    let found = detected(&fixture("string-array-off--control-1.js"));
    assert!(!found.has("string-array"), "{found:?}");
    assert!(found.families.is_empty(), "{found:?}");
}

#[test]
fn plain_code_reports_nothing() {
    let found = detected("function add(a, b) { return a + b; }\nconsole.log(add(1, 2));\n");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_peeled_function_layer_is_reported() {
    let found = detected("Function(\"console.log('hi')\")();\n");
    assert!(found.has("function-layer"), "{found:?}");
}

#[test]
fn a_bundle_names_its_bundler() {
    let path = format!(
        "{}/../../fixtures/unpack-bundlers/esbuild/esbuild-cjs.js",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&path).unwrap();
    let found = detected(&source);
    assert_eq!(found.families, ["esbuild"], "{found:?}");
    assert!(found.techniques.is_empty(), "{found:?}");
    assert_eq!(found.summary(), "esbuild");
}

#[test]
fn detection_reads_the_run_and_leaves_the_code_alone() {
    let source = fixture("pair-cff+dead--control-1.js");
    let output = Pipeline::new(Options::default()).run(&source);
    let before = output.code().to_string();
    let found = detect_obfuscation(&output);
    assert!(!found.is_empty());
    assert_eq!(output.code(), before);
    let again = Pipeline::new(Options::default()).run(&source);
    assert_eq!(again.code(), before);
}
