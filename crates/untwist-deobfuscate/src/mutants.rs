use std::sync::atomic::{AtomicBool, Ordering};

static WRAPPER_GUARDS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_wrapper_guards_for_tests(on: bool) {
    WRAPPER_GUARDS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn wrapper_guards_dropped() -> bool {
    WRAPPER_GUARDS_OFF.load(Ordering::Relaxed)
}

static ROTATOR_PLACEMENT_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_rotator_placement_guards_for_tests(on: bool) {
    ROTATOR_PLACEMENT_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn rotator_placement_dropped() -> bool {
    ROTATOR_PLACEMENT_OFF.load(Ordering::Relaxed)
}

static INERT_PREFIX_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_inert_prefix_guards_for_tests(on: bool) {
    INERT_PREFIX_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn inert_prefix_dropped() -> bool {
    INERT_PREFIX_OFF.load(Ordering::Relaxed)
}

static DIRECT_ARRAY_USES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_direct_array_uses_for_tests(on: bool) {
    DIRECT_ARRAY_USES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn direct_array_uses_trusted() -> bool {
    DIRECT_ARRAY_USES_TRUSTED.load(Ordering::Relaxed)
}

static BUILTIN_IMPORTS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_builtin_import_guards_for_tests(on: bool) {
    BUILTIN_IMPORTS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn builtin_imports_dropped() -> bool {
    BUILTIN_IMPORTS_OFF.load(Ordering::Relaxed)
}

static OBJECT_VALUES_UNCHECKED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn accept_any_object_value_before_the_rotator_for_tests(on: bool) {
    OBJECT_VALUES_UNCHECKED.store(on, Ordering::SeqCst);
}

pub(crate) fn object_values_unchecked() -> bool {
    OBJECT_VALUES_UNCHECKED.load(Ordering::Relaxed)
}
