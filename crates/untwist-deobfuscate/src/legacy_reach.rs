use std::cell::RefCell;

use crate::mutants::deferred_legacy_reach_trusted;

#[derive(Default)]
struct Deferral {
    rerunnable: bool,
    forbidden: bool,
    open: bool,
    pending: Vec<String>,
    unverified: bool,
}

thread_local! {
    static DEFERRAL: RefCell<Deferral> = RefCell::new(Deferral::default());
}

pub(crate) struct Stage {
    open: bool,
    pending: Vec<String>,
}

pub(crate) fn open_stage(on: bool) -> Stage {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        let open = on && deferral.rerunnable && !deferral.forbidden;
        Stage {
            open: std::mem::replace(&mut deferral.open, open),
            pending: std::mem::take(&mut deferral.pending),
        }
    })
}

impl Drop for Stage {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| {
            let mut deferral = deferral.borrow_mut();
            deferral.unverified |= !deferral.pending.is_empty();
            deferral.open = self.open;
            deferral.pending = std::mem::take(&mut self.pending);
        });
    }
}

pub(crate) fn defers() -> bool {
    DEFERRAL.with(|deferral| deferral.borrow().open)
}

pub(crate) fn defer(name: String) {
    if deferred_legacy_reach_trusted() {
        return;
    }
    DEFERRAL.with(|deferral| deferral.borrow_mut().pending.push(name));
}

pub(crate) fn verify(name: &str) {
    DEFERRAL.with(|deferral| {
        deferral
            .borrow_mut()
            .pending
            .retain(|pending| pending != name)
    });
}

pub fn unverified() -> bool {
    DEFERRAL.with(|deferral| deferral.borrow().unverified)
}

pub(crate) fn forget() {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        deferral.unverified = false;
        deferral.pending.clear();
    });
}

pub struct Forbidden(bool);

impl Forbidden {
    pub fn enter() -> Self {
        Self(
            DEFERRAL.with(|deferral| std::mem::replace(&mut deferral.borrow_mut().forbidden, true)),
        )
    }
}

impl Drop for Forbidden {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| deferral.borrow_mut().forbidden = self.0);
    }
}

pub struct Rerunnable(bool);

impl Rerunnable {
    pub fn enter() -> Self {
        Self(
            DEFERRAL
                .with(|deferral| std::mem::replace(&mut deferral.borrow_mut().rerunnable, true)),
        )
    }
}

impl Drop for Rerunnable {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| deferral.borrow_mut().rerunnable = self.0);
    }
}
