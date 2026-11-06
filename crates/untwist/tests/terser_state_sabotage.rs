use untwist::{Options, Pipeline};
use untwist_eval::mutants::{
    read_every_void_as_undefined_for_tests, resolve_dead_zone_names_loosely_for_tests,
    resolve_reset_names_loosely_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/terser_state_rows.txt");

type Switch = fn(bool);

const MUTANTS: [(&str, Switch); 2] = [
    ("ref-void-", read_every_void_as_undefined_for_tests),
    ("ref-zone-", resolve_dead_zone_names_loosely_for_tests),
];

const SWITCHES: [Switch; 3] = [
    read_every_void_as_undefined_for_tests,
    resolve_reset_names_loosely_for_tests,
    resolve_dead_zone_names_loosely_for_tests,
];

fn rows_with(prefix: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with(prefix))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn guards_decoded_under(switch: Switch) -> Vec<String> {
    switch(true);
    let decoded = rows_with("guard-")
        .into_iter()
        .filter(|(_, source)| !transformed(source).contains("bQ("))
        .map(|(label, _)| label)
        .collect();
    switch(false);
    decoded
}

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn diverged_under(node: &std::path::Path, prefix: &str, switch: Switch) -> Vec<String> {
    let sabotage = rows_with(prefix);
    assert!(!sabotage.is_empty(), "no {prefix} rows");
    switch(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(node, &sabotage, &modes);
    switch(false);
    let names: Vec<String> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .map(str::to_string)
        .collect();
    sabotage
        .into_iter()
        .map(|(label, _)| label)
        .filter(|label| !names.contains(label))
        .collect()
}

#[test]
fn sabotage_rows_diverge_and_guard_rows_stay_refused_once_a_check_is_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let kept: Vec<String> = MUTANTS
        .iter()
        .flat_map(|(prefix, switch)| diverged_under(&node, prefix, *switch))
        .collect();
    assert!(
        kept.is_empty(),
        "these rows still print like node under their mutant: {kept:?}"
    );
    for switch in SWITCHES {
        let decoded = guards_decoded_under(switch);
        assert!(
            decoded.is_empty(),
            "guard rows decoded under a mutant: {decoded:?}"
        );
    }
}
