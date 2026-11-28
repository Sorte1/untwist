use std::cell::RefCell;
use std::thread::LocalKey;

use rustc_hash::{FxHashMap, FxHashSet};

type Owners = RefCell<FxHashMap<usize, usize>>;

thread_local! {
    static SELF_CONTAINERS: Owners = RefCell::new(FxHashMap::default());
    static SUB_CONTAINERS: Owners = RefCell::new(FxHashMap::default());
    static ARGUMENTS_PASSED: Owners = RefCell::new(FxHashMap::default());
    static REWRITTEN: RefCell<FxHashSet<usize>> = RefCell::new(FxHashSet::default());
}

pub(super) fn release() {
    for list in [&SELF_CONTAINERS, &SUB_CONTAINERS, &ARGUMENTS_PASSED] {
        list.with(|entries| drop(std::mem::take(&mut *entries.borrow_mut())));
    }
    REWRITTEN.with(|addrs| drop(std::mem::take(&mut *addrs.borrow_mut())));
}

#[inline(always)]
fn lookup(list: &'static LocalKey<Owners>, addr: usize) -> Option<usize> {
    list.with(|entries| entries.borrow().get(&addr).copied())
}

fn remember(list: &'static LocalKey<Owners>, addr: usize, owner: usize) {
    list.with(|entries| {
        entries.borrow_mut().entry(addr).or_insert(owner);
    });
}

#[inline(always)]
pub(super) fn self_owner(addr: usize) -> Option<usize> {
    lookup(&SELF_CONTAINERS, addr)
}

#[inline(always)]
pub(super) fn sub_owner(addr: usize) -> Option<usize> {
    lookup(&SUB_CONTAINERS, addr)
}

pub(super) fn passed(addr: usize) -> Option<usize> {
    lookup(&ARGUMENTS_PASSED, addr)
}

pub(super) fn remember_self(addr: usize, owner: usize) {
    remember(&SELF_CONTAINERS, addr, owner);
}

pub(super) fn remember_sub(addr: usize, owner: usize) {
    remember(&SUB_CONTAINERS, addr, owner);
}

pub(super) fn remember_passed(addr: usize, passed: usize) {
    remember(&ARGUMENTS_PASSED, addr, passed);
}

pub(super) fn note_rewritten(addr: usize) {
    REWRITTEN.with(|addrs| addrs.borrow_mut().insert(addr));
}

pub(super) fn is_rewritten(addr: usize) -> bool {
    REWRITTEN.with(|addrs| addrs.borrow().contains(&addr))
}

pub(super) fn forget(addr: usize) -> bool {
    let mut known = false;
    for list in [&SELF_CONTAINERS, &SUB_CONTAINERS, &ARGUMENTS_PASSED] {
        list.with(|entries| {
            known |= entries.borrow_mut().remove(&addr).is_some();
        });
    }
    REWRITTEN.with(|addrs| addrs.borrow_mut().remove(&addr));
    known
}
