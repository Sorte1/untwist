use std::sync::atomic::{AtomicBool, Ordering};

static VOID_OPERANDS_UNCHECKED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn read_every_void_as_undefined_for_tests(on: bool) {
    VOID_OPERANDS_UNCHECKED.store(on, Ordering::SeqCst);
}

pub(crate) fn void_operands_unchecked() -> bool {
    VOID_OPERANDS_UNCHECKED.load(Ordering::Relaxed)
}

static RESET_NAMES_LOOSE: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn resolve_reset_names_loosely_for_tests(on: bool) {
    RESET_NAMES_LOOSE.store(on, Ordering::SeqCst);
}

pub(crate) fn reset_names_loose() -> bool {
    RESET_NAMES_LOOSE.load(Ordering::Relaxed)
}

static DEAD_ZONE_NAMES_LOOSE: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn resolve_dead_zone_names_loosely_for_tests(on: bool) {
    DEAD_ZONE_NAMES_LOOSE.store(on, Ordering::SeqCst);
}

pub(crate) fn dead_zone_names_loose() -> bool {
    DEAD_ZONE_NAMES_LOOSE.load(Ordering::Relaxed)
}

static NESTED_NAMES_REBOUND: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn read_nested_names_as_rebound_for_tests(on: bool) {
    NESTED_NAMES_REBOUND.store(on, Ordering::SeqCst);
}

pub(crate) fn nested_names_rebound() -> bool {
    NESTED_NAMES_REBOUND.load(Ordering::Relaxed)
}

static MEMO_KEY_READS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_memo_key_reads_for_tests(on: bool) {
    MEMO_KEY_READS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn memo_key_reads_trusted() -> bool {
    MEMO_KEY_READS_TRUSTED.load(Ordering::Relaxed)
}

static MEMO_MISSES_IGNORED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn ignore_memo_misses_for_tests(on: bool) {
    MEMO_MISSES_IGNORED.store(on, Ordering::SeqCst);
}

pub(crate) fn memo_misses_ignored() -> bool {
    MEMO_MISSES_IGNORED.load(Ordering::Relaxed)
}

static CAPTURED_STATIC_USES_IGNORED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn ignore_static_uses_of_captured_memos_for_tests(on: bool) {
    CAPTURED_STATIC_USES_IGNORED.store(on, Ordering::SeqCst);
}

pub(crate) fn captured_static_uses_ignored() -> bool {
    CAPTURED_STATIC_USES_IGNORED.load(Ordering::Relaxed)
}

static BLOCK_LEXICALS_HOISTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn hoist_block_lexicals_for_tests(on: bool) {
    BLOCK_LEXICALS_HOISTED.store(on, Ordering::SeqCst);
}

pub(crate) fn block_lexicals_hoisted() -> bool {
    BLOCK_LEXICALS_HOISTED.load(Ordering::Relaxed)
}

static BLOCK_SHADOWS_LEAK: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn leak_block_shadows_for_tests(on: bool) {
    BLOCK_SHADOWS_LEAK.store(on, Ordering::SeqCst);
}

pub(crate) fn block_shadows_leak() -> bool {
    BLOCK_SHADOWS_LEAK.load(Ordering::Relaxed)
}

static TEMPLATE_PARAMETERS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_template_parameters_for_tests(on: bool) {
    TEMPLATE_PARAMETERS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn template_parameters_trusted() -> bool {
    TEMPLATE_PARAMETERS_TRUSTED.load(Ordering::Relaxed)
}

static TEMPLATE_ARITY_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_template_arity_for_tests(on: bool) {
    TEMPLATE_ARITY_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn template_arity_trusted() -> bool {
    TEMPLATE_ARITY_TRUSTED.load(Ordering::Relaxed)
}

static TEMPLATE_PREFIX_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_template_prefix_for_tests(on: bool) {
    TEMPLATE_PREFIX_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn template_prefix_trusted() -> bool {
    TEMPLATE_PREFIX_TRUSTED.load(Ordering::Relaxed)
}

static VALUE_GATES_RECOGNIZED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn recognize_gates_in_value_position_for_tests(on: bool) {
    VALUE_GATES_RECOGNIZED.store(on, Ordering::SeqCst);
}

pub(crate) fn value_gates_recognized() -> bool {
    VALUE_GATES_RECOGNIZED.load(Ordering::Relaxed)
}

static MISS_RESULTS_UNCHECKED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn leave_memo_miss_results_unchecked_for_tests(on: bool) {
    MISS_RESULTS_UNCHECKED.store(on, Ordering::SeqCst);
}

pub(crate) fn miss_results_unchecked() -> bool {
    MISS_RESULTS_UNCHECKED.load(Ordering::Relaxed)
}

static GATE_STATE_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_state_left_by_gates_for_tests(on: bool) {
    GATE_STATE_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn gate_state_trusted() -> bool {
    GATE_STATE_TRUSTED.load(Ordering::Relaxed)
}

static ROTATOR_WRAPPERS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_rotator_wrappers_by_declaration_for_tests(on: bool) {
    ROTATOR_WRAPPERS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn rotator_wrappers_trusted() -> bool {
    ROTATOR_WRAPPERS_TRUSTED.load(Ordering::Relaxed)
}
