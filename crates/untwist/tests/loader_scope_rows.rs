use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/loader_scope_rows.txt");

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

#[test]
fn a_loader_text_keeps_the_scope_function_or_eval_gives_it_as_commonjs_and_as_a_script() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert_eq!(programs.len(), 18);
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_direct_eval_loader_that_reads_module_bindings_is_still_peeled() {
    let source =
        "var k = \"A\"; eval([\"console.log(typeof k, \", \"typeof module)\"].join(\"\"));";
    let out = Pipeline::new(Options::default()).run(source);
    assert!(
        out.code().contains("console.log(typeof k, typeof module)"),
        "{}",
        out.code()
    );
}
