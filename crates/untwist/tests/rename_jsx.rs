use untwist::{Options, Pipeline, RenameMode};

const MODES: [RenameMode; 3] = [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique];

const CREATE_ELEMENT: &str = "function a(){}\nfunction b(){}\nconst c = 1;\n\
     React.createElement(a, null, React.createElement(b, { x: c }));\n\
     React.createElement(\"div\", null);\n";

const WRITTEN_JSX: &str = "function Outer(){}\nconst Inner = () => null;\n\
     export const view = <Outer><Inner /><span /></Outer>;\n";

fn options(rename: RenameMode, jsx: bool) -> Options {
    Options {
        unpack: false,
        deobfuscate: true,
        unsafe_transforms: true,
        rename,
        rename_all: true,
        jsx,
        ..Options::default()
    }
}

fn tags(code: &str) -> Vec<&str> {
    code.match_indices('<')
        .filter_map(|(at, _)| code.get(at + 1..))
        .map(|rest| rest.trim_start_matches('/'))
        .map(|rest| {
            let end = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
                .unwrap_or(rest.len());
            &rest[..end]
        })
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn assert_components_capitalised(code: &str, intrinsic: &[&str]) {
    for tag in tags(code) {
        let capital = tag.starts_with(|ch: char| ch.is_ascii_uppercase());
        assert!(
            capital || intrinsic.contains(&tag),
            "component tag <{tag}> lost its capital:\n{code}"
        );
    }
}

fn run(source: &str, mode: RenameMode, jsx: bool) -> String {
    Pipeline::new(options(mode, jsx))
        .run(source)
        .code()
        .to_string()
}

#[test]
fn renamed_jsx_components_keep_an_initial_capital_in_every_mode() {
    for mode in MODES {
        let once = run(CREATE_ELEMENT, mode, true);
        assert_eq!(tags(&once).len(), 4, "{mode:?}:\n{once}");
        assert_components_capitalised(&once, &["div"]);
        assert_eq!(run(&once, mode, true), once, "{mode:?} is not idempotent");
    }
}

#[test]
fn written_jsx_components_keep_an_initial_capital_without_the_jsx_stage() {
    for mode in MODES {
        let once = run(WRITTEN_JSX, mode, false);
        assert_components_capitalised(&once, &["span"]);
        assert!(once.contains("<span />"), "{mode:?}:\n{once}");
        assert_eq!(run(&once, mode, false), once, "{mode:?} is not idempotent");
    }
}

#[test]
fn rename_runs_after_the_lexical_stage_extended() {
    let out = Pipeline::new(options(RenameMode::Simple, true)).run(CREATE_ELEMENT);
    let stages: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    let last = |name: &str| stages.iter().rposition(|stage| *stage == name);
    let rename = last("rename").expect("rename stage reported");
    for earlier in ["jsx", "cleanup", "lexical"] {
        let index = last(earlier).unwrap_or_else(|| panic!("{earlier} missing: {stages:?}"));
        assert!(index < rename, "stages: {stages:?}");
    }
    assert_eq!(stages.iter().filter(|stage| **stage == "rename").count(), 1);
}

#[test]
fn rename_keeps_the_reference_order_under_conservative() {
    let out = Pipeline::new(Options {
        extended: false,
        ..options(RenameMode::Mangle, true)
    })
    .run(CREATE_ELEMENT);
    let stages: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    let first = |name: &str| stages.iter().position(|stage| *stage == name);
    let rename = first("rename").expect("rename stage reported");
    let jsx = first("jsx").expect("jsx stage reported");
    assert!(rename < jsx, "stages: {stages:?}");
}

const INVENTED: &str = "function a(){}\nfunction b(){}\n\
     React.createElement(a, null);\nReact.createElement(b, null);\n";

#[test]
fn unique_reads_the_component_names_the_jsx_stage_invents() {
    let once = run(INVENTED, RenameMode::Unique, true);
    assert_eq!(
        once,
        "function Component1() {}\nfunction Component2() {}\n<Component1 />;\n<Component2 />;"
    );
    assert_eq!(run(&once, RenameMode::Unique, true), once);
}

#[test]
fn keeping_human_names_keeps_the_invented_component_names() {
    for mode in [RenameMode::Simple, RenameMode::Unique] {
        let out = Pipeline::new(Options {
            rename_all: false,
            ..options(mode, true)
        })
        .run(INVENTED);
        assert_eq!(
            out.code(),
            "function Component() {}\nfunction Component_2__() {}\n<Component />;\n<Component_2__ />;",
            "{mode:?}"
        );
    }
}
