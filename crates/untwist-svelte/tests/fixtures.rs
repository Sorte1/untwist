use std::path::Path;

fn restored(path: &Path) -> String {
    let code = std::fs::read_to_string(path).expect("fixture reads");
    let restore = untwist_svelte::restore(&code);
    let mut text = String::new();
    for component in &restore.components {
        text.push_str(&format!(
            "==== {}.svelte\n{}",
            component.name, component.text
        ));
    }
    for (name, reason) in &restore.kept {
        text.push_str(&format!("==== {name} kept as JS: {reason}\n"));
    }
    text
}

#[test]
fn every_compiled_corpus_component_restores_to_the_recorded_svelte_source() {
    insta::glob!("fixtures/*.js", |path| {
        insta::assert_snapshot!(restored(path));
    });
}

#[test]
fn code_without_svelte_output_restores_nothing() {
    let restore = untwist_svelte::restore("function get(x) { return x; } get(1);");
    assert!(restore.components.is_empty() && restore.kept.is_empty());
}
