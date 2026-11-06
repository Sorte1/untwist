use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::super::Decoded;
use crate::heap::{reserve_lenient, HeapLedger};
use crate::value::JsString;

enum Slot {
    Plain(Decoded),
    Text(usize),
}

struct Text {
    units: Box<[u16]>,
    bytes: u64,
}

pub(super) struct KeptValues {
    slots: Vec<Slot>,
    texts: Vec<Text>,
}

fn is_held(text: &JsString, ledger: Option<&Rc<HeapLedger>>) -> bool {
    text.charged_bytes() == 0 || ledger.is_some_and(|ledger| text.is_charged_on(ledger))
}

impl KeptValues {
    pub(super) fn of(values: &[Option<Decoded>], ledger: Option<&Rc<HeapLedger>>) -> Option<Self> {
        let mut seen: FxHashMap<usize, usize> = FxHashMap::default();
        let mut texts = Vec::new();
        let mut slots = Vec::with_capacity(values.len());
        for value in values {
            let slot = match value.as_ref()? {
                Decoded::Str(text) if !is_held(text, ledger) => return None,
                Decoded::Str(text) => {
                    Slot::Text(*seen.entry(text.identity()).or_insert_with(|| {
                        texts.push(Text {
                            units: text.units().into(),
                            bytes: text.charged_bytes(),
                        });
                        texts.len() - 1
                    }))
                }
                other => Slot::Plain(other.clone()),
            };
            slots.push(slot);
        }
        Some(Self { slots, texts })
    }

    pub(super) fn charged(&self, ledger: Option<&Rc<HeapLedger>>) -> Vec<Option<Decoded>> {
        let texts: Vec<JsString> = self
            .texts
            .iter()
            .map(|text| {
                let charge = ledger
                    .filter(|_| text.bytes > 0)
                    .map(|ledger| reserve_lenient(ledger, text.bytes));
                JsString::charged(text.units.to_vec(), charge)
            })
            .collect();
        self.slots
            .iter()
            .map(|slot| match slot {
                Slot::Plain(value) => Some(value.clone()),
                Slot::Text(index) => texts.get(*index).cloned().map(Decoded::Str),
            })
            .collect()
    }
}
