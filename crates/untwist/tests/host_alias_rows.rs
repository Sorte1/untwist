#[path = "common/host_alias.rs"]
mod host_alias;

use host_alias::{
    decoded, global_routes, hosted, plain_script, rows, transformed, transformed_under_host,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS_THE_DEFAULT_LANE_STILL_DECODES: &[&str] = &[
    "ref-document-default-view",
    "ref-default-parameter",
    "ref-destructuring-default-target",
    "ref-generator-return-value",
    "ref-bound-without-receiver",
    "ref-route-store-through-owner-document",
    "ref-route-resolved-key-on-event-target",
    "ref-route-store-through-event-view",
    "ref-route-unknown-key-on-event-target",
    "ref-route-literal-store-through-event-view-alias",
    "ref-unknown-key-on-event-target",
    "ref-super-returns-the-global-object",
    "ref-default-constructor-returns-the-global-object",
    "ref-reflect-set-receiver",
    "ref-reflect-set-receiver-alias",
];

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows().into_iter().chain(global_routes()) {
        assert!(
            label.starts_with("dec-") || label.starts_with("ref-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn points_to_decodes_only_when_no_route_reaches_a_used_built_in() {
    let all = rows();
    assert!(all.len() >= 50, "found {} rows", all.len());
    let mut wrong = Vec::new();
    for (label, source) in all {
        let code = transformed(true, &source);
        if decoded(&code) != label.starts_with("dec-") {
            wrong.push(label);
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn points_to_refuses_every_global_object_route() {
    let mut wrong = Vec::new();
    for (label, source) in global_routes() {
        if decoded(&transformed(true, &source)) {
            wrong.push(label);
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn the_default_lane_refuses_every_refused_row_outside_the_known_list() {
    let all = rows().into_iter().chain(global_routes());
    for (label, source) in all.filter(|(label, _)| label.starts_with("ref-")) {
        if ROWS_THE_DEFAULT_LANE_STILL_DECODES.contains(&label.as_str()) {
            continue;
        }
        let code = transformed(false, &source);
        assert!(!decoded(&code), "{label}:\n{code}");
    }
}

#[test]
fn points_to_output_runs_like_node_under_a_host() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs = hosted(&rows());
    programs.extend(hosted(&global_routes()));
    let run = |source: &str| transformed_under_host(true, source);
    let modes: [Mode<'_>; 1] = [("points-to", &run)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_root_function_of_a_script_the_host_may_call_is_refused() {
    let source = plain_script(
        "function init(){this[k][j]=E;}setTimeout(\"init()\",0);setTimeout(function(){console.log(dec0(0x1),dec0(0x2));},10);",
    );
    let code = transformed(true, &source);
    assert!(!decoded(&code), "{code}");
}
