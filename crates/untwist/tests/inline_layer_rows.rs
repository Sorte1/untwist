use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::layers::INLINE_LAYER_PASS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/inline_layer_rows.txt");

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        ..Options::default()
    }
}

fn run(unsafe_transforms: bool, source: &str) -> Output {
    Pipeline::new(options(unsafe_transforms)).run(source)
}

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn inline_peels(out: &Output) -> usize {
    out.stages
        .iter()
        .filter(|stage| stage.stage == INLINE_LAYER_PASS)
        .count()
}

#[test]
fn each_inline_row_peels_or_keeps_its_eval_as_labelled_in_both_lanes() {
    let programs = rows();
    assert!(programs.len() >= 45, "found {} rows", programs.len());
    let mut wrong = Vec::new();
    for (label, source) in &programs {
        for unsafe_transforms in [false, true] {
            let want_peel = label.starts_with("peel:")
                || (unsafe_transforms && label.starts_with("unsafe-peel:"));
            let peeled = inline_peels(&run(unsafe_transforms, source)) > 0;
            if peeled != want_peel {
                wrong.push(format!(
                    "{label} unsafe={unsafe_transforms}: peeled={peeled}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_kept_row_says_why_its_layer_was_kept() {
    let mut silent = Vec::new();
    let kept = |label: &str| label.starts_with("keep:") || label.starts_with("unsafe-peel:");
    for (label, source) in rows().iter().filter(|(label, _)| kept(label)) {
        let out = run(false, source);
        let noted = out.notes.iter().any(|note| {
            note.pass == INLINE_LAYER_PASS && note.message.contains("a layer was kept")
        });
        if !noted {
            silent.push(label.clone());
        }
    }
    assert!(silent.is_empty(), "{silent:?}");
}

#[test]
fn every_inline_row_runs_like_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let run_safe = |source: &str| run(false, source).code().to_string();
    let run_unsafe = |source: &str| run(true, source).code().to_string();
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_peeled_row_leaves_no_direct_eval_of_its_text() {
    let mut left = Vec::new();
    for (label, source) in rows()
        .iter()
        .filter(|(label, _)| label.starts_with("peel:"))
    {
        let code = run(false, source).code().to_string();
        if code.contains("sourceURL") || code.contains("sourceMappingURL") {
            left.push(format!("{label}: {code}"));
        }
    }
    assert!(left.is_empty(), "{left:#?}");
}
