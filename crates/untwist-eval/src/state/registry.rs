use std::cell::RefCell;
use std::thread::LocalKey;

type Owners = RefCell<Vec<(usize, usize)>>;

thread_local! {
    static SELF_CONTAINERS: Owners = const { RefCell::new(Vec::new()) };
    static SUB_CONTAINERS: Owners = const { RefCell::new(Vec::new()) };
    static ARGUMENTS_PASSED: Owners = const { RefCell::new(Vec::new()) };
    static REWRITTEN: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn release() {
    for list in [&SELF_CONTAINERS, &SUB_CONTAINERS, &ARGUMENTS_PASSED] {
        list.with(|entries| drop(std::mem::take(&mut *entries.borrow_mut())));
    }
    REWRITTEN.with(|addrs| drop(std::mem::take(&mut *addrs.borrow_mut())));
}

#[inline(always)]
fn lookup(list: &'static LocalKey<Owners>, addr: usize) -> Option<usize> {
    list.with(|entries| {
        entries
            .borrow()
            .iter()
            .find(|(entry_addr, _)| *entry_addr == addr)
            .map(|(_, owner)| *owner)
    })
}

fn remember(list: &'static LocalKey<Owners>, addr: usize, owner: usize) {
    list.with(|entries| {
        let mut entries = entries.borrow_mut();
        if !entries.iter().any(|(entry_addr, _)| *entry_addr == addr) {
            entries.push((addr, owner));
        }
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
    REWRITTEN.with(|addrs| addrs.borrow_mut().push(addr));
}

pub(super) fn is_rewritten(addr: usize) -> bool {
    REWRITTEN.with(|addrs| addrs.borrow().contains(&addr))
}

pub(super) fn forget(addr: usize) -> bool {
    let mut known = false;
    for list in [&SELF_CONTAINERS, &SUB_CONTAINERS, &ARGUMENTS_PASSED] {
        list.with(|entries| {
            let mut entries = entries.borrow_mut();
            let before = entries.len();
            entries.retain(|(entry_addr, _)| *entry_addr != addr);
            known |= entries.len() != before;
        });
    }
    REWRITTEN.with(|addrs| addrs.borrow_mut().retain(|entry| *entry != addr));
    known
}
