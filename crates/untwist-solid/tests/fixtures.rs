use std::path::Path;

fn restored(path: &Path) -> String {
    let code = std::fs::read_to_string(path).expect("fixture reads");
    let restore = untwist_solid::restore(&code);
    let mut text = String::new();
    for component in &restore.components {
        text.push_str(&format!("==== {}.jsx\n{}", component.name, component.text));
    }
    for (name, reason) in &restore.kept {
        text.push_str(&format!("==== {name} kept as JS: {reason}\n"));
    }
    text
}

#[test]
fn every_compiled_corpus_component_restores_to_the_recorded_jsx() {
    insta::glob!("fixtures/*.js", |path| {
        insta::assert_snapshot!(restored(path));
    });
}

#[test]
fn code_without_solid_output_restores_nothing() {
    let restore = untwist_solid::restore("function get(x) { return x; } get(1);");
    assert!(restore.components.is_empty() && restore.kept.is_empty());
}

#[test]
fn an_unknown_statement_in_an_element_block_keeps_the_component() {
    let code = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Counter.js"),
    )
    .unwrap()
    .replace("return _el$;", "_el$.foo();\n    return _el$;");
    let restore = untwist_solid::restore(&code);
    assert!(restore.components.is_empty());
    assert_eq!(restore.kept.len(), 1);
    assert_eq!(restore.kept[0].0, "Counter");
}
