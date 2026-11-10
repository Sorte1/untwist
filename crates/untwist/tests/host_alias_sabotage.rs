#[path = "common/host_alias.rs"]
mod host_alias;

use host_alias::{hosted, rows, transformed_under_host};
use untwist_deobfuscate::mutants::{
    drop_back_routes_for_tests, drop_unbound_this_for_tests, drop_world_constructors_for_tests,
    trust_builtin_aliases_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const REFUSED_ELSEWHERE: &[&str] = &[];

#[derive(Clone, Copy, Default)]
struct Mutant {
    routes: bool,
    constructors: bool,
    this: bool,
    layer: bool,
}

fn set(mutant: Mutant) {
    drop_back_routes_for_tests(mutant.routes);
    drop_world_constructors_for_tests(mutant.constructors);
    drop_unbound_this_for_tests(mutant.this);
    trust_builtin_aliases_for_tests(mutant.layer);
}

fn undiverged(prefix: &str, mutant: Mutant, node: &std::path::Path) -> Vec<String> {
    let sabotage: Vec<(String, String)> = hosted(&rows())
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix) && !REFUSED_ELSEWHERE.contains(&&**label))
        .collect();
    assert!(!sabotage.is_empty(), "no rows start with {prefix}");
    let run = |source: &str| transformed_under_host(true, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    set(mutant);
    let diverged = node_rows::mismatches(node, &sabotage, &modes);
    set(Mutant::default());
    sabotage
        .into_iter()
        .map(|(label, _)| label)
        .filter(|label| {
            !diverged
                .iter()
                .any(|row| row.starts_with(&format!("{label} ")))
        })
        .collect()
}

#[test]
fn every_refused_row_diverges_from_node_once_its_rule_is_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let on = true;
    let cases = [
        (
            "ref-route-",
            Mutant {
                routes: on,
                ..Mutant::default()
            },
        ),
        (
            "ref-ctor-",
            Mutant {
                constructors: on,
                ..Mutant::default()
            },
        ),
        (
            "ref-both-",
            Mutant {
                routes: on,
                constructors: on,
                ..Mutant::default()
            },
        ),
        (
            "ref-this-",
            Mutant {
                this: on,
                ..Mutant::default()
            },
        ),
        (
            "ref-",
            Mutant {
                layer: on,
                ..Mutant::default()
            },
        ),
    ];
    let mut missing = Vec::new();
    for (prefix, mutant) in cases {
        for label in undiverged(prefix, mutant, &node) {
            missing.push(format!("{label} (rows {prefix}*)"));
        }
    }
    assert!(
        missing.is_empty(),
        "did not diverge from node under the mutant: {missing:?}"
    );
}
