use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::symbol::SymbolId;

use super::Walker;

const WELL_KNOWN_SYMBOLS: [&str; 15] = [
    "asyncDispose",
    "asyncIterator",
    "dispose",
    "hasInstance",
    "isConcatSpreadable",
    "iterator",
    "match",
    "matchAll",
    "replace",
    "search",
    "species",
    "split",
    "toPrimitive",
    "toStringTag",
    "unscopables",
];

const KEY_CALLBACKS: [&str; 5] = ["forEach", "map", "filter", "some", "every"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Holder<'a> {
    Bound(SymbolId),
    Free(&'a str),
}

pub(super) enum KeyProof {
    Open,
    Symbol,
    Enumerated,
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn key_proof(&self, object: &Expression<'a>, key: &Expression<'a>) -> KeyProof {
        if self.well_known_symbol(key) {
            self.symbol_proof_used.set(true);
            return KeyProof::Symbol;
        }
        let Expression::Identifier(id) = key.without_parentheses() else {
            return KeyProof::Open;
        };
        let enumerated = self
            .symbol(id)
            .and_then(|symbol| self.enumerated_holder(symbol))
            .zip(self.holder_of(object))
            .is_some_and(|(from, read)| from == read);
        if enumerated {
            KeyProof::Enumerated
        } else {
            KeyProof::Open
        }
    }

    fn well_known_symbol(&self, key: &Expression<'a>) -> bool {
        let Expression::StaticMemberExpression(member) = key.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(owner) = member.object.without_parentheses() else {
            return false;
        };
        self.trust_symbol
            && owner.name == "Symbol"
            && self.symbol(owner).is_none()
            && WELL_KNOWN_SYMBOLS.contains(&member.property.name.as_str())
    }

    fn holder_of(&self, expr: &Expression<'a>) -> Option<Holder<'a>> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        self.holder_of_id(id)
    }

    fn holder_of_id(&self, id: &IdentifierReference<'a>) -> Option<Holder<'a>> {
        match self.symbol(id) {
            Some(symbol) => self
                .scoping
                .symbol_redeclarations(symbol)
                .is_empty()
                .then_some(Holder::Bound(symbol)),
            None => {
                (!self.free_written(id.name.as_str())).then_some(Holder::Free(id.name.as_str()))
            }
        }
    }

    fn settled(&self, symbol: SymbolId) -> bool {
        !crate::calls::agnostic::rebound(self.scoping, symbol)
            && self.scoping.symbol_redeclarations(symbol).is_empty()
    }

    pub(super) fn free_written(&self, name: &str) -> bool {
        self.scoping
            .root_unresolved_references()
            .get(name)
            .is_some_and(|references| {
                references
                    .iter()
                    .any(|reference| self.scoping.get_reference(*reference).is_write())
            })
    }

    fn enumerated_holder(&self, key: SymbolId) -> Option<Holder<'a>> {
        let source = self.bound_keys.get(&key).copied().unwrap_or(key);
        if source != key && !self.settled(key) {
            return None;
        }
        let (holder, start) = *self.enumerations.get(&source)?;
        let held = match holder {
            Holder::Bound(symbol) => self.written_before(symbol, start),
            Holder::Free(_) => true,
        };
        (held && self.written_before(source, 0)).then_some(holder)
    }

    fn written_before(&self, symbol: SymbolId, start: u32) -> bool {
        let writes = self
            .scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .filter(|reference| self.scoping.get_reference(**reference).is_write())
            .count();
        let assigned = self.assigned_at.get(&symbol).map_or(&[][..], Vec::as_slice);
        self.scoping.symbol_redeclarations(symbol).is_empty()
            && writes == assigned.len()
            && assigned.iter().all(|span| span.end <= start)
    }

    pub(super) fn collect_enumeration(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::ForInStatement(each) => {
                let Some(holder) = self.holder_of(&each.right) else {
                    return;
                };
                if let Some(key) = loop_key(&each.left, false) {
                    self.record_key(key, holder, each.span.start);
                }
            }
            AstKind::ForOfStatement(each) => {
                let Some((holder, pairs)) = self.listed_keys(&each.right) else {
                    return;
                };
                if let Some(key) = loop_key(&each.left, pairs) {
                    self.record_key(key, holder, each.span.start);
                }
            }
            AstKind::CallExpression(call) => self.collect_key_callback(call),
            _ => {}
        }
    }

    fn record_key(&mut self, symbol: SymbolId, holder: Holder<'a>, start: u32) {
        self.enumerations.insert(symbol, (holder, start));
    }

    fn listed_keys(&self, expr: &Expression<'a>) -> Option<(Holder<'a>, bool)> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(owner) = member.object.without_parentheses() else {
            return None;
        };
        let pairs = match member.property.name.as_str() {
            "keys" => false,
            "entries" => true,
            _ => return None,
        };
        let free_object =
            owner.name == "Object" && self.symbol(owner).is_none() && !self.free_written("Object");
        let [Argument::Identifier(target)] = call.arguments.as_slice() else {
            return None;
        };
        if !free_object {
            return None;
        }
        self.holder_of_id(target).map(|holder| (holder, pairs))
    }

    fn collect_key_callback(&mut self, call: &'a CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return;
        };
        if member.property.name == "bind" {
            self.collect_bound_key(member, call);
            return;
        }
        if !KEY_CALLBACKS.contains(&member.property.name.as_str()) {
            return;
        }
        let Some((holder, pairs)) = self.listed_keys(&member.object) else {
            return;
        };
        let Some(params) = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(callback_params)
        else {
            return;
        };
        if let Some(entry) = params
            .items
            .first()
            .and_then(|param| pattern_key(&param.pattern, pairs))
        {
            self.record_key(entry, holder, call.span.start);
        }
    }

    fn collect_bound_key(
        &mut self,
        member: &StaticMemberExpression<'a>,
        call: &CallExpression<'a>,
    ) {
        let Expression::FunctionExpression(function) = member.object.without_parentheses() else {
            return;
        };
        let Some(Argument::Identifier(source)) = call.arguments.get(1) else {
            return;
        };
        let Some(source) = self.symbol(source) else {
            return;
        };
        let Some(BindingPattern::BindingIdentifier(param)) =
            function.params.items.first().map(|param| &param.pattern)
        else {
            return;
        };
        if let Some(param) = param.symbol_id.get() {
            self.bound_keys.insert(param, source);
        }
    }
}

fn callback_params<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k FormalParameters<'a>> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(&function.params),
        Expression::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        _ => None,
    }
}

fn pattern_key(pattern: &BindingPattern<'_>, pairs: bool) -> Option<SymbolId> {
    match (pattern, pairs) {
        (BindingPattern::BindingIdentifier(id), false) => id.symbol_id.get(),
        (BindingPattern::ArrayPattern(array), true) => match array.elements.first()? {
            Some(BindingPattern::BindingIdentifier(id)) => id.symbol_id.get(),
            _ => None,
        },
        _ => None,
    }
}

fn loop_key(left: &ForStatementLeft<'_>, pairs: bool) -> Option<SymbolId> {
    match left {
        ForStatementLeft::VariableDeclaration(declaration) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            pattern_key(&declarator.id, pairs)
        }
        ForStatementLeft::AssignmentTargetIdentifier(_) => None,
        _ => None,
    }
}
