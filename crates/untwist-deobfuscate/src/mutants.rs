use std::sync::atomic::{AtomicBool, Ordering};

static WRAPPER_GUARDS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_wrapper_guards_for_tests(on: bool) {
    WRAPPER_GUARDS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn wrapper_guards_dropped() -> bool {
    WRAPPER_GUARDS_OFF.load(Ordering::Relaxed)
}
