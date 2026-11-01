#[test]
fn the_skip_message_is_stable() {
    assert_eq!(
        untwist_testing::node::SKIP_MESSAGE,
        "skipping behaviour differential: `node` not found on PATH and UNTWIST_NODE is unset"
    );
}

#[test]
fn the_required_env_var_turns_the_skip_into_a_failure() {
    assert!(untwist_testing::node::skip_allowed(None));
    assert!(untwist_testing::node::skip_allowed(Some("")));
    assert!(!untwist_testing::node::skip_allowed(Some("1")));
}
