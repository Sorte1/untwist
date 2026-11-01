use std::cell::{Cell, RefCell};

use oxc_span::Span;
use rustc_hash::FxHashSet;

use crate::value::{JsString, ObjRef, PropKey, Value};

use super::monitor::{
    address_of, breach, gates_of, in_mapped_range, state_tracing, ARGUMENTS_SUB, MONITOR_MARK,
};
use super::registry::sub_owner;

const PROBE_SENTINEL: &str = "untwist:probe";

thread_local! {
    static PROBING: Cell<bool> = const { Cell::new(false) };
    static SKIPPED: RefCell<FxHashSet<(usize, PropKey)>> = RefCell::new(FxHashSet::default());
    static FIRED: RefCell<Vec<Span>> = const { RefCell::new(Vec::new()) };
    static SENTINEL: RefCell<Option<JsString>> = const { RefCell::new(None) };
}

pub(super) fn reset() {
    PROBING.with(|flag| flag.set(false));
    SKIPPED.with(|skipped| drop(std::mem::take(&mut *skipped.borrow_mut())));
    FIRED.with(|fired| drop(std::mem::take(&mut *fired.borrow_mut())));
    SENTINEL.with(|sentinel| drop(sentinel.borrow_mut().take()));
}

pub(super) fn forget(addr: usize) {
    SKIPPED.with(|skipped| skipped.borrow_mut().retain(|(entry, _)| *entry != addr));
}

pub(super) fn note_gate_entered(span: Span) {
    FIRED.with(|fired| {
        let mut fired = fired.borrow_mut();
        if !fired.contains(&span) {
            fired.push(span);
        }
    });
}

fn gates_fired(owner: usize) -> bool {
    let gates = gates_of(owner);
    FIRED.with(|fired| {
        let fired = fired.borrow();
        gates.iter().all(|gate| fired.contains(gate))
    })
}

pub struct ProbeGuard(());

impl Drop for ProbeGuard {
    fn drop(&mut self) {
        PROBING.with(|flag| flag.set(false));
    }
}

pub fn probe_dead_terms() -> ProbeGuard {
    if state_tracing() {
        PROBING.with(|flag| flag.set(true));
    }
    ProbeGuard(())
}

pub(super) fn probing() -> bool {
    PROBING.with(Cell::get)
}

pub(super) fn was_skipped(addr: usize, key: &PropKey) -> bool {
    SKIPPED.with(|skipped| {
        let skipped = skipped.borrow();
        !skipped.is_empty() && skipped.contains(&(addr, key.clone()))
    })
}

#[inline(always)]
pub fn probe_miss<'a>(container: &ObjRef<'a>, key: &PropKey) -> Option<Value<'a>> {
    if !probing() {
        return None;
    }
    note_probe_miss(container, key)
}

#[cold]
#[inline(never)]
fn note_probe_miss<'a>(container: &ObjRef<'a>, key: &PropKey) -> Option<Value<'a>> {
    match judge(container, key) {
        Judgement::Unmarked => None,
        Judgement::Breach => {
            breach();
            None
        }
        Judgement::Insert(addr) => Some(insert_sentinel(container, addr, key)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecipeInsert {
    Inserted,
    Hit,
    Breach,
    Unmarked,
}

enum Judgement {
    Unmarked,
    Breach,
    Insert(usize),
}

fn judge(container: &ObjRef<'_>, key: &PropKey) -> Judgement {
    let (tag, is_arguments) = {
        let borrowed = container.borrow();
        (borrowed.trace_tag(), borrowed.kind.is_arguments())
    };
    if tag & MONITOR_MARK == 0 && !(is_arguments && ARGUMENTS_SUB.with(Cell::get)) {
        return Judgement::Unmarked;
    }
    let addr = address_of(container);
    let Some(owner) = sub_owner(addr) else {
        return Judgement::Unmarked;
    };
    if in_mapped_range(addr, key) || !gates_fired(owner) {
        return Judgement::Breach;
    }
    Judgement::Insert(addr)
}

fn shared_sentinel() -> JsString {
    SENTINEL.with(|sentinel| {
        sentinel
            .borrow_mut()
            .get_or_insert_with(|| JsString::from_str(PROBE_SENTINEL))
            .clone()
    })
}

fn insert_sentinel<'a>(container: &ObjRef<'a>, addr: usize, key: &PropKey) -> Value<'a> {
    let sentinel = Value::Str(shared_sentinel());
    SKIPPED.with(|skipped| skipped.borrow_mut().insert((addr, key.clone())));
    container
        .borrow_mut()
        .props
        .insert(key.clone().into(), sentinel.clone());
    sentinel
}

pub(crate) fn predict_recipe_miss(container: &ObjRef<'_>, key: &PropKey) -> RecipeInsert {
    if crate::object::has_own_property(container, key) {
        return RecipeInsert::Hit;
    }
    match judge(container, key) {
        Judgement::Unmarked => RecipeInsert::Unmarked,
        Judgement::Breach => RecipeInsert::Breach,
        Judgement::Insert(_) => RecipeInsert::Inserted,
    }
}

pub(crate) fn note_recipe_miss(container: &ObjRef<'_>, key: &PropKey) -> RecipeInsert {
    if crate::object::has_own_property(container, key) {
        return RecipeInsert::Hit;
    }
    match judge(container, key) {
        Judgement::Unmarked => RecipeInsert::Unmarked,
        Judgement::Breach => {
            breach();
            RecipeInsert::Breach
        }
        Judgement::Insert(addr) => {
            insert_sentinel(container, addr, key);
            RecipeInsert::Inserted
        }
    }
}

pub(crate) fn skipped_count() -> usize {
    SKIPPED.with(|skipped| skipped.borrow().len())
}

pub(crate) fn skipped_here(container: &ObjRef<'_>, key: &PropKey) -> bool {
    was_skipped(address_of(container), key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{set_state_tracing, take_violation, Shape};
    use crate::value::ObjKind;

    fn registered_arguments<'a>(items: Vec<Value<'a>>, passed: usize) -> ObjRef<'a> {
        let value = Value::object(ObjKind::Arguments(items), None);
        let container = value.as_object().cloned().expect("an object");
        let addr = address_of(&container);
        super::super::registry::remember_sub(addr, 0);
        super::super::registry::remember_passed(addr, passed);
        ARGUMENTS_SUB.with(|flag| flag.set(true));
        container
    }

    #[test]
    fn a_key_in_a_mapped_arguments_range_breaches_exactly_as_the_probe() {
        let shape = Shape::default();
        let _tracing = set_state_tracing(Some(&shape));
        let container = registered_arguments(Vec::new(), 2);
        let mapped = PropKey::Index(1);
        assert_eq!(
            predict_recipe_miss(&container, &mapped),
            RecipeInsert::Breach
        );
        assert!(take_violation().is_none());
        assert_eq!(note_recipe_miss(&container, &mapped), RecipeInsert::Breach);
        assert!(take_violation().is_some());
        let _probe = probe_dead_terms();
        assert!(probe_miss(&container, &mapped).is_none());
        assert!(take_violation().is_some());
        assert_eq!(skipped_count(), 0);
        let free = PropKey::Name(JsString::from_str("7x"));
        assert_eq!(note_recipe_miss(&container, &free), RecipeInsert::Inserted);
        assert!(skipped_here(&container, &free));
        assert_eq!(note_recipe_miss(&container, &free), RecipeInsert::Hit);
        assert!(take_violation().is_none());
    }

    #[test]
    fn an_unregistered_container_is_unmarked_for_the_recipe_and_untouched_by_the_probe() {
        let shape = Shape::default();
        let _tracing = set_state_tracing(Some(&shape));
        let value = Value::object(ObjKind::Arguments(Vec::new()), None);
        let container = value.as_object().cloned().expect("an object");
        let key = PropKey::Name(JsString::from_str("1a"));
        assert_eq!(note_recipe_miss(&container, &key), RecipeInsert::Unmarked);
        let _probe = probe_dead_terms();
        assert!(probe_miss(&container, &key).is_none());
        assert!(take_violation().is_none());
        assert_eq!(skipped_count(), 0);
    }
}
