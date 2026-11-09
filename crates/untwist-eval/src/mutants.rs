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
