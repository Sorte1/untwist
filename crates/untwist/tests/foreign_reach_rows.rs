#[path = "common/host_alias.rs"]
mod host_alias;

use host_alias::{
    decoded, foreign_reach_rows, hosted, transformed, transformed_under_host, PRELUDE,
};
use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

#[test]
fn the_default_lane_refuses_every_global_object_reached_through_foreign_or_made_code() {
    let rows = foreign_reach_rows();
    assert!(rows.len() >= 20, "found {} rows", rows.len());
    let mut wrong = Vec::new();
    for (label, source) in rows {
        if decoded(&transformed(false, &source)) != label.starts_with("dec-") {
            wrong.push(label);
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn the_default_lane_runs_like_node_under_a_host() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = hosted(&foreign_reach_rows());
    let run = |source: &str| transformed_under_host(false, source);
    let modes: [Mode<'_>; 1] = [("default", &run)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn unsafe_lane(hosted: &str) -> String {
    let program = hosted.strip_prefix(PRELUDE).unwrap_or(hosted);
    let options = Options {
        unpack: false,
        jsx: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    let code = Pipeline::new(options).run(program).code().to_string();
    format!("{PRELUDE}{code}")
}

#[test]
fn every_refused_row_prints_otherwise_once_decoded() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let refused: Vec<(String, String)> = foreign_reach_rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    let modes: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &hosted(&refused), &modes);
    let silent: Vec<&String> = refused
        .iter()
        .map(|(label, _)| label)
        .filter(|label| {
            !wrong
                .iter()
                .any(|line| line.starts_with(&format!("{label} ")))
        })
        .collect();
    assert!(silent.is_empty(), "{silent:?}");
}
