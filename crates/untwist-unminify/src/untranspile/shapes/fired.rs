use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub type Shape = (&'static str, &'static str);

static FIRED: Mutex<BTreeMap<Shape, u64>> = Mutex::new(BTreeMap::new());
static ARMED: AtomicBool = AtomicBool::new(false);

pub fn note(pass: &'static str, shape: &'static str) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    let mut fired = FIRED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *fired.entry((pass, shape)).or_insert(0) += 1;
}

pub fn take() -> BTreeMap<Shape, u64> {
    ARMED.store(true, Ordering::Relaxed);
    let mut fired = FIRED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::mem::take(&mut *fired)
}
