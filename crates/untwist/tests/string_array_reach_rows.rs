use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, printed, Mode};

const ROWS: &str = include_str!("data/string_array_reach_rows.txt");
const SHELL_ROWS: &str = include_str!("data/string_array_shell_rows.txt");

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

const DYNAMIC_PATH_WITNESSES: [&str; 22] = [
    "amb-object-unknown",
    "amb-reflect-unknown",
    "dr-d12",
    "dr-d15",
    "dr-d16",
    "dr-d18",
    "dr-d21",
    "dr-n-assign",
    "dr-n-charcode",
    "dr-n-descriptors",
    "dr-n-entries",
    "dr-n-globalthis",
    "dr-n-ownkeys",
    "dr-n-param",
    "dr-n-rest",
    "dr-n-spread",
    "s2-n-entries",
    "s2-n-forin",
    "s2-n-keys",
    "s2-n-stringify",
    "s2-r14",
    "sw-n-values",
];

fn both_modes() -> [(&'static str, Options); 2] {
    [
        (
            "default",
            Options {
                disabled_passes: vec!["inline-layers"],
                ..Options::default()
            },
        ),
        (
            "unsafe",
            Options {
                unsafe_transforms: true,
                disabled_passes: vec!["inline-layers"],
                ..Options::default()
            },
        ),
    ]
}

#[test]
fn a_string_array_reached_only_by_its_own_value_reads_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert!(programs.len() >= 200, "found {} rows", programs.len());
    let [(_, safe), (_, unsafe_transforms)] = both_modes();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (wrong, witnesses) = node_rows::unsafe_dynamic_code_witnesses(wrong, &programs);
    let (wrong, mut paths) = node_rows::listed_unsafe_witnesses(wrong, &DYNAMIC_PATH_WITNESSES);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(witnesses.len(), 42, "{witnesses:?}");
    paths.sort();
    let mut want = DYNAMIC_PATH_WITNESSES.to_vec();
    want.sort();
    assert_eq!(paths, want);
}

#[test]
fn a_shell_wrapped_string_array_is_judged_on_the_scoping_of_the_input() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(SHELL_ROWS);
    assert!(programs.len() >= 10, "found {} rows", programs.len());
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for (label, shell) in &programs {
        let before = printed(&node, dir.path(), &format!("{shell}()"));
        for (mode, options) in both_modes() {
            let after = printed(&node, dir.path(), &transformed(&options, shell));
            if before.is_err() || after != before {
                wrong.push(format!(
                    "{label} ({mode}): the called shell printed {before:?}, untwist's output \
                     printed {after:?}"
                ));
            }
        }
    }
    let (wrong, witnesses) = node_rows::unsafe_dynamic_code_witnesses(wrong, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(witnesses.len(), 1, "{witnesses:?}");
}
