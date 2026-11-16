use std::cell::RefCell;

use crate::mutants::deferred_legacy_reach_trusted;

#[derive(Clone, Debug, Default)]
pub(crate) struct Layered {
    pub(crate) names: Vec<String>,
    pub(crate) used: Option<Vec<String>>,
    pub(crate) points_to: bool,
    pub(crate) late_reads: Option<Vec<untwist_eval::object::ProtoRead>>,
    pub(crate) roots: Vec<String>,
}

#[derive(Default)]
struct Deferral {
    rerunnable: bool,
    forbidden: bool,
    open: bool,
    pending: Vec<String>,
    layered: Vec<(String, Layered)>,
    observations: Vec<Vec<String>>,
    unverified: bool,
}

thread_local! {
    static DEFERRAL: RefCell<Deferral> = RefCell::new(Deferral::default());
}

pub(crate) struct Stage {
    open: bool,
    pending: Vec<String>,
    layered: Vec<(String, Layered)>,
}

pub(crate) fn open_stage(on: bool) -> Stage {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        let open = on && deferral.rerunnable && !deferral.forbidden;
        Stage {
            open: std::mem::replace(&mut deferral.open, open),
            pending: std::mem::take(&mut deferral.pending),
            layered: std::mem::take(&mut deferral.layered),
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
            deferral.layered = std::mem::take(&mut self.layered);
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

pub(crate) fn defer_layered(token: String, layered: Layered) {
    defer(token.clone());
    DEFERRAL.with(|deferral| deferral.borrow_mut().layered.push((token, layered)));
}

pub(crate) fn widen_layered(token: &str, used: Vec<String>, points_to: bool) {
    DEFERRAL.with(|deferral| {
        for (pending, layered) in &mut deferral.borrow_mut().layered {
            if pending == token {
                layered.used = Some(used.clone());
                layered.points_to = points_to;
            }
        }
    });
}

pub(crate) fn drop_layered(token: &str) {
    verify(token);
    DEFERRAL.with(|deferral| {
        deferral
            .borrow_mut()
            .layered
            .retain(|(pending, _)| pending != token)
    });
}

pub(crate) fn widen_late(token: &str, reads: &[untwist_eval::object::ProtoRead]) {
    DEFERRAL.with(|deferral| {
        for (pending, layered) in &mut deferral.borrow_mut().layered {
            if pending == token {
                layered.late_reads = Some(reads.to_vec());
            }
        }
    });
}

pub(crate) fn widen_root(token: &str, root: &str) {
    DEFERRAL.with(|deferral| {
        for (pending, layered) in &mut deferral.borrow_mut().layered {
            if pending == token && !layered.roots.iter().any(|held| held == root) {
                layered.roots.push(root.to_string());
            }
        }
    });
}

pub(crate) fn waive_observation(names: &[&str]) -> bool {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        let pending = |name: &&str| {
            deferral
                .layered
                .iter()
                .any(|(_, layered)| layered.names.iter().any(|held| held == name))
        };
        if !deferral.open || names.is_empty() || !names.iter().all(pending) {
            return false;
        }
        let names: Vec<String> = names.iter().map(|name| (*name).to_string()).collect();
        if !deferral.observations.contains(&names) {
            deferral.observations.push(names);
        }
        true
    })
}

pub fn settle_observations(mut observed: impl FnMut(&[&str]) -> bool) {
    let observations =
        DEFERRAL.with(|deferral| std::mem::take(&mut deferral.borrow_mut().observations));
    let failed = observations.iter().any(|names| {
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        observed(&names)
    });
    if failed {
        DEFERRAL.with(|deferral| deferral.borrow_mut().unverified = true);
    }
}

pub(crate) fn settle_layered(mut holds: impl FnMut(&Layered) -> bool) {
    let layered = DEFERRAL.with(|deferral| std::mem::take(&mut deferral.borrow_mut().layered));
    for (token, obligation) in layered {
        if holds(&obligation) {
            verify(&token);
        }
    }
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
    DEFERRAL.with(|deferral| {
        let deferral = deferral.borrow();
        deferral.unverified || !deferral.observations.is_empty()
    })
}

pub(crate) fn forget() {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        deferral.unverified = false;
        deferral.pending.clear();
        deferral.layered.clear();
        deferral.observations.clear();
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
