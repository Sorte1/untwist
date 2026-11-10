use std::cell::RefCell;

use rustc_hash::FxHashSet;

thread_local! {
    static UNBOUND: RefCell<FxHashSet<String>> = RefCell::new(FxHashSet::default());
}

pub fn note_unbound(name: &str) {
    UNBOUND.with(|unbound| {
        let mut unbound = unbound.borrow_mut();
        if !unbound.contains(name) {
            unbound.insert(name.to_string());
        }
    });
}

pub fn is_unbound_name(name: &str) -> bool {
    UNBOUND.with(|unbound| unbound.borrow().contains(name))
}

pub(crate) fn forget() {
    UNBOUND.with(|unbound| unbound.borrow_mut().clear());
}
