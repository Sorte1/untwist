use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::cjs_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const DIR: &str = "fixtures/commonjs";
const DECLARERS: [&str; 4] = ["function ", "const ", "let ", "var "];
const DECOY: &str = "decoy";
const UNSAFE_PREMISE: &str = "folded-checksum-not-global.cjs";

fn run(source: &str, options: Options) -> String {
    Pipeline::new(options).run(source).code().to_string()
}

fn commonjs(unsafe_transforms: bool, rename: RenameMode) -> Options {
    Options {
        unsafe_transforms,
        rename,
        commonjs_goal: true,
        ..Options::default()
    }
}

fn either(unsafe_transforms: bool, rename: RenameMode) -> Options {
    Options {
        unsafe_transforms,
        rename,
        ..Options::default()
    }
}

fn commonjs_default(source: &str) -> String {
    run(source, commonjs(false, RenameMode::None))
}

fn commonjs_unsafe(source: &str) -> String {
    run(source, commonjs(true, RenameMode::None))
}

fn commonjs_smart(source: &str) -> String {
    run(source, commonjs(false, RenameMode::Smart))
}

fn either_default(source: &str) -> String {
    run(source, either(false, RenameMode::None))
}

fn either_unsafe(source: &str) -> String {
    run(source, either(true, RenameMode::None))
}

fn either_smart(source: &str) -> String {
    run(source, either(false, RenameMode::Smart))
}

fn fixtures() -> Vec<(String, String)> {
    let fixtures = cjs_files_in(DIR);
    assert_eq!(fixtures.len(), 12);
    fixtures
}

fn named(label: &str) -> &str {
    label.rsplit('/').next().unwrap_or(label)
}

fn declared_decoys(code: &str) -> Vec<String> {
    let mut names: Vec<String> = DECLARERS
        .iter()
        .flat_map(|declarer| {
            code.match_indices(&format!("{declarer}{DECOY}"))
                .map(|(at, _)| {
                    let rest = &code[at + declarer.len()..];
                    let end = rest
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                        .unwrap_or(rest.len());
                    rest[..end].to_string()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn every_commonjs_row_prints_what_node_prints_when_it_loads_the_file_as_cjs() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let modes: [Mode<'_>; 3] = [
        ("commonjs default", &commonjs_default),
        ("commonjs unsafe", &commonjs_unsafe),
        ("commonjs smart rename", &commonjs_smart),
    ];
    let wrong = node_rows::commonjs_mismatches(&node, &fixtures(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_same_rows_as_classic_scripts_print_what_node_prints_both_ways() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let (premise, both): (Vec<_>, Vec<_>) = fixtures()
        .into_iter()
        .partition(|(label, _)| named(label) == UNSAFE_PREMISE);
    assert_eq!(premise.len(), 1);
    let modes: [Mode<'_>; 3] = [
        ("default", &either_default),
        ("unsafe", &either_unsafe),
        ("smart rename", &either_smart),
    ];
    let mut wrong = node_rows::mismatches(&node, &both, &modes);
    let safe: [Mode<'_>; 2] = [
        ("default", &either_default),
        ("smart rename", &either_smart),
    ];
    wrong.extend(node_rows::mismatches(&node, &premise, &safe));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn unread_decoys_go_only_when_the_input_is_commonjs() {
    for (label, source) in fixtures() {
        let declared = declared_decoys(&source);
        for code in [either_default(&source), either_unsafe(&source)] {
            assert_eq!(declared_decoys(&code), declared, "{label}:\n{code}");
        }
        let removed = named(&label).starts_with("removed-");
        for code in [commonjs_default(&source), commonjs_unsafe(&source)] {
            match removed {
                true => assert!(declared_decoys(&code).is_empty(), "{label}:\n{code}"),
                false => assert_eq!(declared_decoys(&code), declared, "{label}:\n{code}"),
            }
        }
    }
}

fn fixture(name: &str) -> String {
    fixtures()
        .into_iter()
        .find(|(label, _)| named(label) == name)
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{name} in {DIR}"))
}

#[test]
fn a_checksum_this_cannot_reach_folds_only_in_commonjs() {
    let source = fixture("folded-checksum-not-global.cjs");
    let folded = commonjs_default(&source);
    assert!(!folded.contains("Math.imul(seed"), "{folded}");
    let kept = either_default(&source);
    assert!(kept.contains("Math.imul(seed"), "{kept}");
}

#[test]
fn export_keys_name_their_bindings_only_in_commonjs() {
    let source = fixture("export-names.cjs");
    let named = commonjs_smart(&source);
    for expected in ["function tryUnlock(", "function decode(", "const SCOPE = 7"] {
        assert!(named.contains(expected), "{expected}:\n{named}");
    }
    let kept = either_smart(&source);
    assert!(
        kept.contains("function computeSessionDigest_a1b("),
        "{kept}"
    );
}

#[test]
fn a_cjs_name_or_the_goal_makes_an_input_commonjs_and_script_overrides_the_name() {
    let source = fixture("removed-decoy-functions.cjs");
    let by_name = run(
        &source,
        Options {
            input_names: vec!["sample.cjs".to_string()],
            ..Options::default()
        },
    );
    assert_eq!(by_name, commonjs_default(&source));
    let as_script = run(
        &source,
        Options {
            input_names: vec!["sample.cjs".to_string()],
            script_goal: true,
            ..Options::default()
        },
    );
    assert_eq!(as_script, either_default(&source));
}
