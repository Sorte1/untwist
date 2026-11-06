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
