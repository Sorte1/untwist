use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traversal {
    pub changes: u32,
    pub walked: bool,
    pub steered: bool,
    pub lists_parked: u32,
    pub statements_shelved: u32,
    pub confirming: bool,
}

thread_local! {
    static LOG: RefCell<Vec<Traversal>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record(traversal: Traversal) {
    LOG.with(|log| log.borrow_mut().push(traversal));
}

#[doc(hidden)]
pub fn take_work_for_tests() -> Vec<Traversal> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}
