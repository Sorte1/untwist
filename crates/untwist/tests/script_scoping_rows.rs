use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/script_scoping_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn run(options: Options, source: &str) -> String {
    Pipeline::new(options).run(source).code().to_string()
}

fn renamed(rename: RenameMode, source: &str) -> String {
    run(
        Options {
            rename,
            ..Options::default()
        },
        source,
    )
}

#[test]
fn a_shadowed_global_is_not_folded() {
    for (label, source) in rows() {
        if label.starts_with("shadowed-") {
            let code = run(Options::default(), &source);
            assert!(code.contains("shadow:"), "{label}:\n{code}");
        }
    }
}

#[test]
fn sloppy_block_functions_run_like_node_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let default = |source: &str| run(Options::default(), source);
    let unsafe_lane = |source: &str| {
        run(
            Options {
                unsafe_transforms: true,
                ..Options::default()
            },
            source,
        )
    };
    let simple = |source: &str| renamed(RenameMode::Simple, source);
    let unique = |source: &str| renamed(RenameMode::Unique, source);
    let mangle = |source: &str| renamed(RenameMode::Mangle, source);
    let smart = |source: &str| renamed(RenameMode::Smart, source);
    let modes: [Mode<'_>; 6] = [
        ("default", &default),
        ("unsafe", &unsafe_lane),
        ("rename simple", &simple),
        ("rename unique", &unique),
        ("rename mangle", &mangle),
        ("rename smart", &smart),
    ];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
