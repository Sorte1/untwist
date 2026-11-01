use untwist::{Options, Pipeline, RenameMode};

fn unsafe_options() -> Options {
    Options {
        unpack: false,
        deobfuscate: true,
        unsafe_transforms: true,
        ..Options::default()
    }
}

#[test]
fn the_jsx_stage_runs_and_is_reported() {
    let out = Pipeline::new(unsafe_options()).run("React.createElement(\"div\", null);");
    assert_eq!(out.code(), "<div />;");
    let jsx_stage = out
        .stages
        .iter()
        .find(|stage| stage.stage == "jsx")
        .expect("jsx stage reported");
    let names: Vec<&str> = jsx_stage.per_pass.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, untwist_jsx::JSX_ORDER);
}

#[test]
fn jsx_is_gated_on_the_unsafe_flag() {
    let out = Pipeline::new(Options {
        unsafe_transforms: false,
        ..unsafe_options()
    })
    .run("React.createElement(\"div\", null);");
    assert_eq!(out.code(), "React.createElement(\"div\", null);");
}

#[test]
fn no_jsx_disables_the_stage() {
    let out = Pipeline::new(Options {
        jsx: false,
        ..unsafe_options()
    })
    .run("React.createElement(\"div\", null);");
    assert_eq!(out.code(), "React.createElement(\"div\", null);");
    assert!(!out.stages.iter().any(|stage| stage.stage == "jsx"));
}

#[test]
fn rename_runs_after_jsx_and_keeps_component_names_capitalised() {
    let out = Pipeline::new(Options {
        rename: RenameMode::Simple,
        rename_all: false,
        ..unsafe_options()
    })
    .run("function _0xa(){}\nReact.createElement(_0xa, null);");
    let stages: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    let index = |name: &str| {
        stages
            .iter()
            .position(|stage| *stage == name)
            .unwrap_or_else(|| panic!("{name} missing: {stages:?}"))
    };
    assert!(index("jsx") < index("rename"), "stages: {stages:?}");
    assert!(index("protections") < index("jsx"), "stages: {stages:?}");
    assert!(index("jsx") < index("cleanup"), "stages: {stages:?}");
    assert_eq!(out.code(), "function A() {}\n<A />;");
}

#[test]
fn every_stage_is_implemented_so_nothing_is_reported_as_missing() {
    let out = Pipeline::new(Options::default()).run("1;");
    let messages: Vec<&str> = out.notes.iter().map(|note| note.message.as_str()).collect();
    assert!(
        !messages.iter().any(|m| m.contains("not implemented")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.starts_with("jsx:")),
        "{messages:?}"
    );
    let stages: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    assert!(stages.contains(&"jsx"), "{stages:?}");
}

#[test]
fn the_hoisted_type_note_survives_the_drivers_retraction() {
    let out = Pipeline::new(unsafe_options()).run("jsx(r ? \"a\" : \"div\", {});");
    assert!(
        out.notes
            .iter()
            .any(|note| note.message.contains(untwist_jsx::notes::TYPE_HOISTED)),
        "{:?}",
        out.notes
    );
}
