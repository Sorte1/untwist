use std::rc::Rc;

use crate::bail::EvalResult;
use crate::budget::{BINDING_BYTES, ENV_BYTES};
use crate::builtins::args::text_bytes;
use crate::builtins::Realm;
use crate::env::SCOPE_CHAIN_CAP;
use crate::frame::FrameRef;
use crate::heap::{reserve, reserve_lenient, Charge, HeapLedger};
use crate::value::JsString;

const PAGE_UNITS: usize = 256;
pub(crate) const WIDE_SCOPE_BINDINGS: usize = 16;
const PAGES: usize = (u16::MAX as usize + 1) / PAGE_UNITS;
const PAGE_BYTES: u64 = (PAGE_UNITS * std::mem::size_of::<Option<JsString>>()) as u64;

type Page = Box<[Option<JsString>]>;

#[derive(Default)]
pub(crate) struct UnitStrings {
    pages: Vec<Option<Page>>,
}

impl UnitStrings {
    fn cached(&self, unit: u16) -> Option<&JsString> {
        let (page, slot) = split(unit);
        self.pages.get(page)?.as_ref()?.get(slot)?.as_ref()
    }

    fn page_mut(&mut self, page: usize) -> Option<&mut Page> {
        if self.pages.is_empty() {
            self.pages.resize_with(PAGES, || None);
        }
        let entry = self.pages.get_mut(page)?;
        Some(entry.get_or_insert_with(|| (0..PAGE_UNITS).map(|_| None).collect()))
    }
}

fn split(unit: u16) -> (usize, usize) {
    let unit = usize::from(unit);
    (unit / PAGE_UNITS, unit % PAGE_UNITS)
}

fn charged(ledger: Option<&Rc<HeapLedger>>, bytes: u64) -> EvalResult<Option<Charge>> {
    ledger.map(|ledger| reserve(ledger, bytes)).transpose()
}

impl<'a> Realm<'a> {
    pub fn set_heap(&mut self, heap: Option<Rc<HeapLedger>>) {
        self.heap = heap;
    }

    pub fn heap_charge(&self, bytes: u64) -> Option<Charge> {
        self.heap
            .as_ref()
            .map(|ledger| reserve_lenient(ledger, bytes))
    }

    #[inline]
    pub fn charge_wide(&self, env: &FrameRef<'a>) {
        if self.heap.is_none() || env.borrow().charge.is_some() {
            return;
        }
        if env.borrow().body.len() > WIDE_SCOPE_BINDINGS {
            self.charge_wide_scope(env);
        }
    }

    #[cold]
    fn charge_wide_scope(&self, env: &FrameRef<'a>) {
        let Some(ledger) = &self.heap else {
            return;
        };
        let mut scope = env.borrow_mut();
        let bindings = scope.body.len();
        if scope.charge.is_some() {
            return;
        }
        let bytes = ENV_BYTES.saturating_add((bindings as u64).saturating_mul(BINDING_BYTES));
        scope.charge = Some(reserve_lenient(ledger, bytes));
    }

    pub fn capture(&self, env: &FrameRef<'a>) {
        let Some(ledger) = &self.heap else {
            return;
        };
        let mut current = env.clone();
        for _ in 0..SCOPE_CHAIN_CAP {
            let parent = {
                let mut scope = current.borrow_mut();
                if scope.charge.is_some() {
                    return;
                }
                let bindings = scope.body.len() as u64;
                let bytes = ENV_BYTES.saturating_add(bindings.saturating_mul(BINDING_BYTES));
                scope.charge = Some(reserve_lenient(ledger, bytes));
                scope.parent.clone()
            };
            match parent {
                Some(parent) => current = parent,
                None => return,
            }
        }
    }

    pub fn unit_string(&mut self, unit: u16) -> EvalResult<JsString> {
        if let Some(found) = self.units.cached(unit) {
            return Ok(found.clone());
        }
        let (page, slot) = split(unit);
        let fresh_page = self.units.pages.get(page).is_none_or(Option::is_none);
        let page_charge = charged(self.heap.as_ref(), if fresh_page { PAGE_BYTES } else { 0 })?;
        let charge = charged(self.heap.as_ref(), text_bytes(1))?;
        if let Some(page_charge) = page_charge {
            if let Some(charge) = &charge {
                charge.absorb(page_charge);
            }
        }
        let text = JsString::charged(vec![unit], charge);
        if let Some(entry) = self
            .units
            .page_mut(page)
            .and_then(|page| page.get_mut(slot))
        {
            *entry = Some(text.clone());
        }
        Ok(text)
    }
}
