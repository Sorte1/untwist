use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use rustc_hash::FxHashMap;

use oxc_syntax::symbol::SymbolId;
use untwist_eval::fold::Const;

use super::tokens::{Key, Literals, Token};

pub(super) struct Names {
    keys: FxHashMap<u32, Key>,
    literals: Option<(Key, Vec<Option<Vec<Token>>>)>,
}

fn literal_tokens(value: &Const) -> Option<Vec<Token>> {
    match value {
        Const::Num(n) if *n > 0.0 || (*n == 0.0 && n.is_sign_positive()) => {
            Some(vec![Token::Num(n.to_bits())])
        }
        Const::Num(n) if *n < 0.0 => Some(vec![
            Token::Punct("-".to_string()),
            Token::Num((-n).to_bits()),
        ]),
        Const::Str(text) => Some(vec![Token::Str(text.encode_utf16().collect())]),
        Const::Bool(true) => Some(vec![Token::Word("true")]),
        Const::Bool(false) => Some(vec![Token::Word("false")]),
        Const::Null => Some(vec![Token::Word("null")]),
        Const::Undefined => Some(vec![Token::Word("void"), Token::Num(0f64.to_bits())]),
        _ => None,
    }
}

impl Names {
    pub(super) fn resolve(&self, start: u32) -> Option<Key> {
        self.keys.get(&start).copied()
    }

    pub(super) fn literals(&self) -> Option<Literals<'_>> {
        self.literals
            .as_ref()
            .map(|(key, values)| Literals { key: *key, values })
    }

    pub(super) fn with_literals(mut self, array: SymbolId, values: &[Const]) -> Self {
        self.literals = Some((
            Key::Binding(array.index() as u32),
            values.iter().map(literal_tokens).collect(),
        ));
        self
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
    Names {
        keys: collect.keys,
        literals: None,
    }
}
