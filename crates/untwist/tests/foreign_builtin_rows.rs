use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/foreign_builtin_rows.txt");
const INPUT_BREAK: &str = " @@ ";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn run(program: &str) -> String {
    let inputs: Vec<&str> = program.split(INPUT_BREAK).collect();
    let options = Options {
        unpack: false,
        ..Options::default()
    };
    Pipeline::new(options).run_many(&inputs).codes.join("\n")
}

#[test]
fn a_built_in_another_input_may_replace_is_not_folded_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let all = rows();
    assert!(all.len() >= 10, "{} rows", all.len());
    let programs: Vec<(String, String)> = all
        .iter()
        .map(|(label, program)| (label.clone(), program.replace(INPUT_BREAK, "\n")))
        .collect();
    let split = |joined: &str| {
        all.iter()
            .find(|(_, program)| program.replace(INPUT_BREAK, "\n") == joined)
            .map(|(_, program)| run(program))
            .expect("a known row")
    };
    let modes: [Mode<'_>; 1] = [("default", &split)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
