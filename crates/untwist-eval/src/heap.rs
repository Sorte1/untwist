use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use crate::bail::{Bail, EvalResult};

pub struct HeapLedger {
    limit: u64,
    live: Cell<u64>,
    peak: Cell<u64>,
}

impl HeapLedger {
    pub fn new(limit: u64) -> Rc<Self> {
        Rc::new(Self {
            limit,
            live: Cell::new(0),
            peak: Cell::new(0),
        })
    }

    pub fn live(&self) -> u64 {
        self.live.get()
    }

    pub fn peak(&self) -> u64 {
        self.peak.get()
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn is_over(&self) -> bool {
        self.peak.get() > self.limit
    }

    #[inline]
    pub fn check(&self) -> EvalResult<()> {
        if self.is_over() {
            return Err(Bail::Budget);
        }
        Ok(())
    }

    fn take_lenient(&self, bytes: u64) {
        let live = self.live.get().saturating_add(bytes);
        self.live.set(live);
        if live > self.peak.get() {
            self.peak.set(live);
        }
    }

    fn take(&self, bytes: u64) -> EvalResult<()> {
        let live = self.live.get().saturating_add(bytes);
        if live > self.limit {
            return Err(Bail::Budget);
        }
        self.live.set(live);
        if live > self.peak.get() {
            self.peak.set(live);
        }
        Ok(())
    }

    fn give_back(&self, bytes: u64) {
        self.live.set(self.live.get().saturating_sub(bytes));
    }
}

pub fn reserve(ledger: &Rc<HeapLedger>, bytes: u64) -> EvalResult<Charge> {
    ledger.take(bytes)?;
    Ok(Charge {
        ledger: Rc::clone(ledger),
        bytes: Cell::new(bytes),
    })
}

pub fn reserve_lenient(ledger: &Rc<HeapLedger>, bytes: u64) -> Charge {
    ledger.take_lenient(bytes);
    Charge {
        ledger: Rc::clone(ledger),
        bytes: Cell::new(bytes),
    }
}

pub struct Charge {
    ledger: Rc<HeapLedger>,
    bytes: Cell<u64>,
}

impl Charge {
    pub fn grow(&self, bytes: u64) -> EvalResult<()> {
        self.ledger.take(bytes)?;
        self.bytes.set(self.bytes.get().saturating_add(bytes));
        Ok(())
    }

    pub fn grow_lenient(&self, bytes: u64) {
        self.ledger.take_lenient(bytes);
        self.bytes.set(self.bytes.get().saturating_add(bytes));
    }

    pub fn shrink(&self, bytes: u64) {
        let credit = bytes.min(self.bytes.get());
        self.ledger.give_back(credit);
        self.bytes.set(self.bytes.get() - credit);
    }

    pub fn shrink_to(&self, bytes: u64) {
        self.shrink(self.bytes.get().saturating_sub(bytes));
    }

    pub fn absorb(&self, other: Charge) {
        if Rc::ptr_eq(&self.ledger, &other.ledger) {
            self.bytes
                .set(self.bytes.get().saturating_add(other.bytes.replace(0)));
            return;
        }
        self.grow_lenient(other.bytes.get());
    }

    pub fn bytes(&self) -> u64 {
        self.bytes.get()
    }

    pub(crate) fn is_on(&self, ledger: &Rc<HeapLedger>) -> bool {
        Rc::ptr_eq(&self.ledger, ledger)
    }
}

impl Drop for Charge {
    fn drop(&mut self) {
        self.ledger.give_back(self.bytes.get());
    }
}

impl fmt::Debug for Charge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Charge({})", self.bytes.get())
    }
}

#[cfg(test)]
mod tests {
    use super::{reserve, reserve_lenient, HeapLedger};

    #[test]
    fn a_dropped_charge_is_credited_and_the_peak_kept() {
        let ledger = HeapLedger::new(100);
        let first = reserve(&ledger, 60).expect("fits");
        assert!(reserve(&ledger, 50).is_err());
        drop(first);
        let second = reserve(&ledger, 50).expect("fits after the credit");
        assert!(second.grow(40).is_ok());
        assert!(second.grow(20).is_err());
        assert_eq!(ledger.live(), 90);
        drop(second);
        assert_eq!(ledger.live(), 0);
        assert_eq!(ledger.peak(), 90);
    }

    #[test]
    fn a_shrink_credits_the_ledger_and_never_below_zero() {
        let ledger = HeapLedger::new(100);
        let charge = reserve(&ledger, 80).expect("fits");
        charge.shrink(30);
        assert_eq!((charge.bytes(), ledger.live()), (50, 50));
        charge.shrink_to(20);
        assert_eq!((charge.bytes(), ledger.live()), (20, 20));
        charge.shrink(500);
        assert_eq!((charge.bytes(), ledger.live()), (0, 0));
        assert_eq!(ledger.peak(), 80);
    }

    #[test]
    fn a_lenient_charge_over_the_limit_is_kept_and_marks_the_ledger() {
        let ledger = HeapLedger::new(100);
        let charge = reserve_lenient(&ledger, 60);
        assert!(ledger.check().is_ok());
        charge.grow_lenient(60);
        assert_eq!(ledger.live(), 120);
        assert!(ledger.is_over());
        assert!(ledger.check().is_err());
        let other = reserve_lenient(&ledger, 5);
        charge.absorb(other);
        assert_eq!((charge.bytes(), ledger.live()), (125, 125));
        drop(charge);
        assert_eq!(ledger.live(), 0);
        assert!(ledger.is_over());
    }
}
