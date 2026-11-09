use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_states_class_reads_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/states_class_rows.txt");
const CLASS: &str = "newState";

fn rows(prefix: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with(prefix))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn a_kept_states_class_is_dropped_and_diverges_from_node_once_its_reads_are_trusted() {
    let guarded = rows("guard-");
    assert!(!guarded.is_empty());
    trust_states_class_reads_for_tests(true);
    let dropped: Vec<String> = guarded
        .iter()
        .map(|(_, source)| transformed(source))
        .collect();
    trust_states_class_reads_for_tests(false);
    for code in &dropped {
        assert!(!code.contains(CLASS), "{code}");
    }
    let Some(node) = node_or_skip() else {
        return;
    };
    trust_states_class_reads_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &guarded, &modes);
    trust_states_class_reads_for_tests(false);
    assert_eq!(diverged.len(), guarded.len(), "{diverged:?}");
}
