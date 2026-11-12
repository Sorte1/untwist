#[path = "common/storage_rows.rs"]
mod storage_rows;

use storage_rows::{labelled, transformed};
use untwist_deobfuscate::mutants::trust_builtin_aliases_for_tests;
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

fn quiet_under(prefix: &str, mutant: &'static str) -> Option<Vec<String>> {
    let node = node_or_skip()?;
    let sabotage = labelled(prefix);
    assert!(!sabotage.is_empty());
    let mutated = |source: &str| transformed(false, Some(mutant), source);
    let modes: [Mode<'_>; 1] = [("mutant", &mutated)];
    let diverged = mismatches(&node, &sabotage, &modes);
    Some(
        sabotage
            .into_iter()
            .map(|(label, _)| label)
            .filter(|label| {
                !diverged
                    .iter()
                    .any(|row| row.split(" (mutant):").next() == Some(label.as_str()))
            })
            .collect(),
    )
}

#[test]
fn the_sabotage_rows_diverge_from_node_once_the_storage_proof_is_the_only_guard() {
    trust_builtin_aliases_for_tests(true);
    let trusted = quiet_under("ref-trust-", "trusted-storage");
    let order = quiet_under("ref-order-", "forward-order");
    trust_builtin_aliases_for_tests(false);
    let (Some(trusted), Some(order)) = (trusted, order) else {
        return;
    };
    assert!(
        trusted.is_empty(),
        "did not diverge from node under the trusted-storage mutant: {trusted:?}"
    );
    assert!(
        order.is_empty(),
        "did not diverge from node under the forward-order mutant: {order:?}"
    );
}
