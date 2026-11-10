use std::cell::RefCell;

use oxc_allocator::{Address, GetAddress};
use oxc_span::Span;
use rustc_hash::FxHashMap;

const ALIAS_HOPS: usize = 64;

#[derive(Default)]
pub struct Origins {
    replaced: FxHashMap<Address, Span>,
    aliases: FxHashMap<Span, Span>,
}

impl Origins {
    pub fn replaced(&self, address: Address) -> Option<Span> {
        self.replaced.get(&address).copied()
    }

    pub fn resolved(&self, span: Span) -> Span {
        let mut span = span;
        for _ in 0..ALIAS_HOPS {
            match self.aliases.get(&span) {
                Some(earlier) => span = *earlier,
                None => break,
            }
        }
        span
    }
}

thread_local! {
    static RECORDED: RefCell<Option<Origins>> = const { RefCell::new(None) };
}

pub struct Recording {
    outer: Option<Option<Origins>>,
}

impl Recording {
    pub fn start() -> Self {
        let outer = RECORDED.with(|recorded| recorded.replace(Some(Origins::default())));
        Self { outer: Some(outer) }
    }

    pub fn finish(mut self) -> Origins {
        self.restore().unwrap_or_default()
    }

    fn restore(&mut self) -> Option<Origins> {
        let outer = self.outer.take()?;
        RECORDED.with(|recorded| recorded.replace(outer))
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.restore();
    }
}

pub fn is_recording() -> bool {
    RECORDED.with(|recorded| recorded.borrow().is_some())
}

pub fn note_replaced(node: &impl GetAddress, span: Span) {
    RECORDED.with(|recorded| {
        if let Some(origins) = recorded.borrow_mut().as_mut() {
            origins.replaced.insert(node.address(), span);
        }
    });
}

pub fn note_respanned(pairs: &[(Span, Span)]) {
    RECORDED.with(|recorded| {
        if let Some(origins) = recorded.borrow_mut().as_mut() {
            for &(fresh, earlier) in pairs {
                if fresh != earlier && earlier.start < earlier.end {
                    origins.aliases.insert(fresh, earlier);
                }
            }
        }
    });
}
