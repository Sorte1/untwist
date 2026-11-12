use oxc_ast::ast::Argument;
use oxc_ast::ast::FormalParameters;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::functions::{param_symbols, ArgName, CallFacts};
use super::values::is_naming_property;

const SHARED_METHOD_NAMES: [&str; 40] = [
    "push", "pop", "shift", "unshift", "add", "set", "get", "has", "delete", "clear", "call",
    "apply", "bind", "then", "catch", "map", "filter", "forEach", "reduce", "join", "split",
    "slice", "splice", "indexOf", "includes", "toString", "valueOf", "emit", "on", "off", "once",
    "send", "remove", "append", "copy", "clone", "dispose", "replace", "concat", "find",
];

enum Definition {
    One(Vec<Option<SymbolId>>),
    Many,
}

#[derive(Default)]
pub struct MethodSites {
    definitions: FxHashMap<String, Definition>,
    calls: FxHashMap<String, CallFacts>,
}

fn is_tracked(name: &str) -> bool {
    !SHARED_METHOD_NAMES.contains(&name) && is_naming_property(name)
}

impl MethodSites {
    pub fn define(&mut self, name: &str, params: &FormalParameters<'_>) {
        if !is_tracked(name) {
            return;
        }
        self.definitions
            .entry(name.to_string())
            .and_modify(|definition| *definition = Definition::Many)
            .or_insert_with(|| Definition::One(param_symbols(params)));
    }

    pub fn call(&mut self, name: &str, arguments: &[Argument<'_>]) {
        if !is_tracked(name) {
            return;
        }
        self.calls
            .entry(name.to_string())
            .or_default()
            .record(arguments, false);
    }

    pub fn offers(self) -> Vec<(SymbolId, String)> {
        let mut found = Vec::new();
        for (name, definition) in self.definitions {
            let Definition::One(params) = definition else {
                continue;
            };
            let Some(calls) = self.calls.get(&name) else {
                continue;
            };
            for (param, arg) in params.iter().zip(&calls.args) {
                if let (Some(param), ArgName::Named(arg)) = (param, arg) {
                    found.push((*param, arg.clone()));
                }
            }
        }
        found
    }
}
