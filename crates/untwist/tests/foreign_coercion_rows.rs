use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/foreign_coercion_rows.txt");
const LIBRARY: &str = "function lib9() { return { a: 1 }; }";
const DECODED: &str = "hello abcbye ";

fn owner(line: &str) -> String {
    format!(
        "var arr = [\"hello \", \"bye \"];\nfunction dec(i) {{ return arr[i]; }}\nfunction show() {{ var out = dec(0) + \"abc\" + dec(1); console.log(out); }}\nshow();\n{line}\nshow();\n"
    )
}

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, line)| (label.to_string(), owner(line)))
        .collect()
}

fn outputs(owner: &str, unsafe_transforms: bool) -> Vec<String> {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        ..Options::default()
    };
    Pipeline::new(options).run_many(&[LIBRARY, owner]).codes
}

fn joined(owner: &str, unsafe_transforms: bool) -> String {
    outputs(owner, unsafe_transforms).join("\n")
}

fn decoded(owner: &str, unsafe_transforms: bool) -> bool {
    outputs(owner, unsafe_transforms)
        .last()
        .is_some_and(|code| code.contains(DECODED))
}

#[test]
fn foreign_values_handed_to_built_ins_print_the_same_in_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let all = rows();
    assert!(all.len() >= 20, "{} rows", all.len());
    let programs: Vec<(String, String)> = all
        .iter()
        .map(|(label, owner)| (label.clone(), format!("{LIBRARY}\n{owner}")))
        .collect();
    let find = |program: &str| {
        all.iter()
            .find(|(_, owner)| format!("{LIBRARY}\n{owner}") == program)
            .map(|(_, owner)| owner.clone())
            .expect("a known row")
    };
    let default = |program: &str| joined(&find(program), false);
    let unsafe_lane = |program: &str| joined(&find(program), true);
    let modes: [Mode<'_>; 2] = [("default", &default), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_foreign_value_given_only_to_a_coercing_built_in_keeps_decoding() {
    let wrong: Vec<String> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("fold-"))
        .filter(|(_, owner)| !decoded(owner, false) || !decoded(owner, true))
        .map(|(label, _)| label)
        .collect();
    assert!(wrong.is_empty(), "not decoded: {wrong:?}");
}

#[test]
fn a_foreign_value_given_to_a_built_in_that_may_call_it_stays_kept_in_the_default_lane() {
    let wrong: Vec<String> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("keep-"))
        .filter(|(_, owner)| decoded(owner, false))
        .map(|(label, _)| label)
        .collect();
    assert!(wrong.is_empty(), "decoded: {wrong:?}");
}
