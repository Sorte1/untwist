use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collisions {
    pub bound_in_scope: u32,
    pub free_reference: u32,
    pub label: u32,
    pub excluded: u32,
}

impl Collisions {
    pub fn merged(self, other: Collisions) -> Collisions {
        Collisions {
            bound_in_scope: self.bound_in_scope.saturating_add(other.bound_in_scope),
            free_reference: self.free_reference.saturating_add(other.free_reference),
            label: self.label.saturating_add(other.label),
            excluded: self.excluded.saturating_add(other.excluded),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Reason {
    BoundInScope,
    FreeReference,
    Label,
    Excluded,
}

thread_local! {
    static RECORDED: Cell<Collisions> = const {
        Cell::new(Collisions {
            bound_in_scope: 0,
            free_reference: 0,
            label: 0,
            excluded: 0,
        })
    };
}

#[derive(Default)]
pub(crate) struct Tally {
    counts: Cell<Collisions>,
}

impl Tally {
    #[inline]
    pub(crate) fn note(&self, reason: Reason) {
        let mut counts = self.counts.get();
        let slot = match reason {
            Reason::BoundInScope => &mut counts.bound_in_scope,
            Reason::FreeReference => &mut counts.free_reference,
            Reason::Label => &mut counts.label,
            Reason::Excluded => &mut counts.excluded,
        };
        *slot = slot.saturating_add(1);
        self.counts.set(counts);
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        let counts = self.counts.get();
        if counts != Collisions::default() {
            RECORDED.with(|recorded| recorded.set(recorded.get().merged(counts)));
        }
    }
}

pub fn take() -> Collisions {
    RECORDED.with(|recorded| recorded.replace(Collisions::default()))
}
