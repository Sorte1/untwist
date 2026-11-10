use std::cell::Cell;

thread_local! {
    static SHARED_GLOBALS: Cell<bool> = const { Cell::new(false) };
}

pub struct SharedGlobalsGuard {
    previous: bool,
}

impl Drop for SharedGlobalsGuard {
    fn drop(&mut self) {
        SHARED_GLOBALS.with(|cell| cell.set(self.previous));
    }
}

pub fn assume_shared_globals(shared: bool) -> SharedGlobalsGuard {
    let previous = SHARED_GLOBALS.with(|cell| cell.replace(shared));
    SharedGlobalsGuard { previous }
}

pub fn shares_globals() -> bool {
    SHARED_GLOBALS.with(Cell::get)
}
