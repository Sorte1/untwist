use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use rustc_hash::FxHashMap;

use super::tokens::Key;

pub(super) struct Names {
    keys: FxHashMap<u32, Key>,
}

impl Names {
    pub(super) fn resolve(&self, start: u32) -> Option<Key> {
        self.keys.get(&start).copied()
    }
}

struct Collect<'s> {
    scoping: &'s Scoping,
    keys: FxHashMap<u32, Key>,
}

impl<'a> VisitJs<'a> for Collect<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.keys
                .insert(it.span.start, Key::Binding(symbol.index() as u32));
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        let key = match symbol {
            Some(symbol) => Key::Binding(symbol.index() as u32),
            None => Key::Free,
        };
        self.keys.insert(it.span.start, key);
    }

    fn visit_label_identifier(&mut self, it: &LabelIdentifier<'a>) {
        self.keys.insert(it.span.start, Key::Label);
        walk_js::walk_label_identifier(self, it);
    }
}

pub(super) fn names(program: &Program<'_>, scoping: &Scoping) -> Names {
    let mut collect = Collect {
        scoping,
        keys: FxHashMap::default(),
    };
    collect.visit_program(program);
    Names { keys: collect.keys }
}
