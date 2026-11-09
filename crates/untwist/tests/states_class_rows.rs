use untwist::{Options, Pipeline};
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
fn the_unsafe_lane_drops_a_states_class_and_decodes_its_decoder() {
    let decoded = rows("dec-");
    assert!(!decoded.is_empty());
    for (label, source) in &decoded {
        let code = transformed(source);
        assert!(
            !code.contains(CLASS) && !code.contains("arr0 = "),
            "{label}:\n{code}"
        );
    }
    let Some(node) = node_or_skip() else {
        return;
    };
    let modes: [Mode<'_>; 1] = [("unsafe", &transformed)];
    let wrong = node_rows::mismatches(&node, &decoded, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_states_class_whose_variable_is_read_later_is_kept() {
    for (label, source) in rows("guard-") {
        let code = transformed(&source);
        assert!(code.contains(CLASS), "{label}:\n{code}");
    }
}
