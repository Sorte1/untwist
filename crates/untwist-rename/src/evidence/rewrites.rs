use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::values::method_name;
use crate::collect::reference_symbol;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Write {
    SelfUpdate,
    Call(String),
    Other,
}

#[derive(Default)]
pub struct Rewrites {
    candidates: FxHashMap<SymbolId, Option<String>>,
    writes: FxHashMap<SymbolId, Vec<Write>>,
}

fn callee_word(value: &Expression<'_>) -> Option<String> {
    let Expression::CallExpression(call) = value.without_parentheses() else {
        return None;
    };
    match call.callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.to_string()),
        callee => method_name(callee).map(|name| name.into_owned()),
    }
}

struct Mentions<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if reference_symbol(it, self.scoping) == Some(self.symbol) {
            self.found = true;
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn mentions(value: &Expression<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    let mut visitor = Mentions {
        scoping,
        symbol,
        found: false,
    };
    visitor.visit_expression(value);
    visitor.found
}

impl Rewrites {
    pub fn declared(&mut self, symbol: SymbolId, init: &Expression<'_>) {
        self.candidates.insert(symbol, callee_word(init));
    }

    pub fn assigned(&mut self, symbol: SymbolId, value: &Expression<'_>, scoping: &Scoping) {
        let write = if mentions(value, symbol, scoping) {
            Write::SelfUpdate
        } else {
            callee_word(value).map_or(Write::Other, Write::Call)
        };
        self.writes.entry(symbol).or_default().push(write);
    }

    pub fn updated(&mut self, symbol: SymbolId) {
        self.writes
            .entry(symbol)
            .or_default()
            .push(Write::SelfUpdate);
    }

    pub fn settle(self, scoping: &Scoping) -> FxHashSet<SymbolId> {
        self.candidates
            .iter()
            .filter(|(symbol, callee)| !self.keeps_value(**symbol, callee.as_deref(), scoping))
            .map(|(symbol, _)| *symbol)
            .collect()
    }

    fn keeps_value(&self, symbol: SymbolId, callee: Option<&str>, scoping: &Scoping) -> bool {
        let Some(writes) = self.writes.get(&symbol) else {
            return false;
        };
        let count = scoping
            .get_resolved_references(symbol)
            .filter(|reference| reference.is_write())
            .count();
        scoping.symbol_redeclarations(symbol).is_empty()
            && count == writes.len()
            && writes.iter().all(|write| match write {
                Write::SelfUpdate => true,
                Write::Call(name) => Some(name.as_str()) == callee,
                Write::Other => false,
            })
    }
}
