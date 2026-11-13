use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedGlobals {
    pub every_name: bool,
    pub names: FxHashSet<String>,
    pub other_scripts: bool,
    pub observed_elsewhere: bool,
    pub named_elsewhere: FxHashSet<String>,
}

impl SharedGlobals {
    pub fn is_empty(&self) -> bool {
        !self.every_name
            && self.names.is_empty()
            && !self.other_scripts
            && !self.observed_elsewhere
            && self.named_elsewhere.is_empty()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.every_name || self.names.contains(name)
    }
}

thread_local! {
    static SHARED_GLOBALS: RefCell<Option<Rc<SharedGlobals>>> = const { RefCell::new(None) };
}

pub struct SharedGlobalsGuard {
    previous: Option<Rc<SharedGlobals>>,
}

impl Drop for SharedGlobalsGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        SHARED_GLOBALS.with(|cell| *cell.borrow_mut() = previous);
    }
}

pub fn assume_shared_globals(shared: Option<SharedGlobals>) -> SharedGlobalsGuard {
    let next = shared.filter(|shared| !shared.is_empty()).map(Rc::new);
    let previous = SHARED_GLOBALS.with(|cell| cell.replace(next));
    SharedGlobalsGuard { previous }
}

pub fn is_shared_global(name: &str) -> bool {
    SHARED_GLOBALS.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|shared| shared.contains(name))
    })
}

pub fn top_level_names_fixed(counter_shared: bool) -> bool {
    SHARED_GLOBALS.with(|cell| {
        cell.borrow().as_ref().is_some_and(|shared| {
            shared.observed_elsewhere || (shared.other_scripts && !counter_shared)
        })
    })
}

pub fn named_by_another_input(name: &str) -> bool {
    SHARED_GLOBALS.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|shared| shared.named_elsewhere.contains(name))
    })
}
