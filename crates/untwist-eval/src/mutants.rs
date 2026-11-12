#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, Ordering};

macro_rules! switch {
    ($flag:ident, $set:ident, $get:ident) => {
        #[cfg(feature = "test-hooks")]
        static $flag: AtomicBool = AtomicBool::new(false);

        #[cfg(feature = "test-hooks")]
        #[doc(hidden)]
        pub fn $set(on: bool) {
            $flag.store(on, Ordering::SeqCst);
        }

        #[cfg(feature = "test-hooks")]
        pub(crate) fn $get() -> bool {
            $flag.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "test-hooks"))]
        pub(crate) const fn $get() -> bool {
            false
        }
    };
}

switch!(
    VOID_OPERANDS_UNCHECKED,
    read_every_void_as_undefined_for_tests,
    void_operands_unchecked
);
switch!(
    RESET_NAMES_LOOSE,
    resolve_reset_names_loosely_for_tests,
    reset_names_loose
);
switch!(
    DEAD_ZONE_NAMES_LOOSE,
    resolve_dead_zone_names_loosely_for_tests,
    dead_zone_names_loose
);
switch!(
    NESTED_NAMES_REBOUND,
    read_nested_names_as_rebound_for_tests,
    nested_names_rebound
);
switch!(
    MEMO_KEY_READS_TRUSTED,
    trust_memo_key_reads_for_tests,
    memo_key_reads_trusted
);
switch!(
    MEMO_MISSES_IGNORED,
    ignore_memo_misses_for_tests,
    memo_misses_ignored
);
switch!(
    CAPTURED_STATIC_USES_IGNORED,
    ignore_static_uses_of_captured_memos_for_tests,
    captured_static_uses_ignored
);
switch!(
    BLOCK_LEXICALS_HOISTED,
    hoist_block_lexicals_for_tests,
    block_lexicals_hoisted
);
switch!(
    BLOCK_SHADOWS_LEAK,
    leak_block_shadows_for_tests,
    block_shadows_leak
);
switch!(
    TEMPLATE_PARAMETERS_TRUSTED,
    trust_template_parameters_for_tests,
    template_parameters_trusted
);
switch!(
    TEMPLATE_ARITY_TRUSTED,
    trust_template_arity_for_tests,
    template_arity_trusted
);
switch!(
    TEMPLATE_PREFIX_TRUSTED,
    trust_template_prefix_for_tests,
    template_prefix_trusted
);
switch!(
    VALUE_GATES_RECOGNIZED,
    recognize_gates_in_value_position_for_tests,
    value_gates_recognized
);
switch!(
    MISS_RESULTS_UNCHECKED,
    leave_memo_miss_results_unchecked_for_tests,
    miss_results_unchecked
);
switch!(
    GATE_STATE_TRUSTED,
    trust_state_left_by_gates_for_tests,
    gate_state_trusted
);
switch!(
    ROTATOR_WRAPPERS_TRUSTED,
    trust_rotator_wrappers_by_declaration_for_tests,
    rotator_wrappers_trusted
);
