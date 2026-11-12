#[path = "common/filtered_rows.rs"]
mod filtered_rows;

use filtered_rows::{options, rows, transformed};
use untwist_deobfuscate::mutants::trust_builtin_aliases_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const NOT_SABOTAGE: &[&str] = &["ref-domain-lock-redirect-through-the-global-object"];

fn undiverged(mutant: &'static str, prefix: &str, node: &std::path::Path) -> Vec<String> {
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix) && !NOT_SABOTAGE.contains(&label.as_str()))
        .collect();
    assert!(!sabotage.is_empty(), "no {prefix} rows");
    let mutated = options(false, Some(mutant));
    let run_mutant = |source: &str| transformed(&mutated, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run_mutant)];
    let diverged = node_rows::mismatches(node, &sabotage, &modes);
    sabotage
        .into_iter()
        .map(|(label, _)| label)
        .filter(|label| {
            !diverged
                .iter()
                .any(|row| row.starts_with(&format!("{label} (mutant):")))
        })
        .collect()
}

#[test]
fn the_sabotage_rows_diverge_from_node_once_the_filter_is_the_only_guard() {
    let Some(node) = node_or_skip() else {
        return;
    };
    trust_builtin_aliases_for_tests(true);
    let enumeration = undiverged("filtered-enumeration", "ref-", &node);
    let order = undiverged("filtered-rewrite-order", "ref-order-", &node);
    trust_builtin_aliases_for_tests(false);
    assert!(
        enumeration.is_empty(),
        "filtered-enumeration: {enumeration:?}"
    );
    assert!(order.is_empty(), "filtered-rewrite-order: {order:?}");
}
