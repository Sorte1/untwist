#[cfg(not(feature = "test-hooks"))]
pub(super) fn trusts_id_order() -> bool {
    false
}

#[cfg(feature = "test-hooks")]
mod switches {
    use std::sync::atomic::{AtomicBool, Ordering};

    static ID_ORDER: AtomicBool = AtomicBool::new(false);

    pub fn trust_id_order_for_tests(on: bool) {
        ID_ORDER.store(on, Ordering::SeqCst);
    }

    pub(in super::super) fn trusts_id_order() -> bool {
        ID_ORDER.load(Ordering::SeqCst)
    }
}

#[cfg(feature = "test-hooks")]
pub use switches::trust_id_order_for_tests;
#[cfg(feature = "test-hooks")]
pub(super) use switches::trusts_id_order;
