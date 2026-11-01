use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::semantic::build_scoping;

use super::layout::{Def, Id, Layout};
use super::{ENTRY_ID, MAX_PARAMS, NAMES};
use crate::scan::{is_direct_eval, Frame, Policy, Scan, Step};

const REQUIRE: &str = "require";

struct Rules<'s, 'l> {
    scoping: &'s Scoping,
    ids: &'l FxHashSet<&'l Id>,
    params: [Option<SymbolId>; MAX_PARAMS],
    ok: bool,
}

impl Rules<'_, '_> {
    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn param(&self, at: usize) -> Option<SymbolId> {
        self.params[at]
    }

    fn is_param(&self, expr: &Expression<'_>, at: usize) -> bool {
        matches!(expr, Expression::Identifier(id) if self.param(at).is_some() && self.symbol(id) == self.param(at))
    }

    fn context_member<'k>(&self, expr: &'k Expression<'_>) -> Option<&'k str> {
        let Expression::StaticMemberExpression(member) = expr else {
            return None;
        };
        (!member.optional && self.is_param(&member.object, 0))
            .then(|| member.property.name.as_str())
    }

    fn resolved(&self, it: &IdentifierReference<'_>, symbol: SymbolId) -> bool {
        if Some(symbol) == self.param(0) || Some(symbol) == self.param(1) {
            return false;
        }
        Some(symbol) != self.param(2)
            || it
                .reference_id
                .get()
                .is_some_and(|reference| !self.scoping.get_reference(reference).is_write())
    }

    fn free(name: &str, frame: Frame) -> bool {
        match name {
            REQUIRE | "module" | "exports" | "__filename" | "__dirname" => false,
            "arguments" => frame.receiver(),
            _ => true,
        }
    }

    fn single<'k, 'a>(call: &'k CallExpression<'a>) -> Option<&'k Expression<'a>> {
        match call.arguments.as_slice() {
            [argument] => argument.as_expression(),
            _ => None,
        }
    }

    fn required(&self, call: &CallExpression<'_>) -> bool {
        Self::single(call)
            .and_then(Id::of)
            .is_some_and(|id| self.ids.contains(&id))
    }

    fn exported<'k, 'a>(&self, expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
        let Expression::CallExpression(call) = expr else {
            return None;
        };
        if self.context_member(&call.callee) != Some("v") {
            return None;
        }
        Self::single(call)
    }
}

impl<'a> Policy<'a> for Rules<'_, '_> {
    fn done(&self) -> bool {
        !self.ok
    }

    fn item<'k>(&mut self, it: &'k Expression<'a>) -> &'k Expression<'a> {
        self.exported(it).unwrap_or(it)
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        self.ok &= match self.symbol(it) {
            Some(symbol) => self.resolved(it, symbol),
            None => Self::free(it.name.as_str(), frame),
        };
    }

    fn binding(&mut self, it: &BindingIdentifier<'a>) {
        let named = matches!(it.name.as_str(), REQUIRE | "module" | "exports");
        self.ok &= !named || self.params.contains(&it.symbol_id.get());
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        if let Some(method) = self.context_member(&it.callee) {
            self.ok &= !it.optional
                && match method {
                    "r" => self.required(it),
                    _ => false,
                };
            return Step::Skip;
        }
        if is_direct_eval(self.scoping, &it.callee) {
            self.ok = false;
            return Step::Skip;
        }
        Step::Walk
    }

    fn static_member(&mut self, it: &StaticMemberExpression<'a>) -> Step {
        let exports = it.property.name == "exports" && self.is_param(&it.object, 1);
        let this = it.property.name == "e" && self.is_param(&it.object, 0);
        match it.optional || !(exports || this) {
            true => Step::Walk,
            false => Step::Skip,
        }
    }

    fn target(&mut self, it: &SimpleAssignmentTarget<'a>) -> Step {
        if let SimpleAssignmentTarget::StaticMemberExpression(member) = it {
            if self.is_param(&member.object, 0) {
                self.ok = false;
                return Step::Skip;
            }
        }
        Step::Walk
    }

    fn unary(&mut self, it: &UnaryExpression<'a>) -> Step {
        if it.operator == UnaryOperator::Delete {
            if let Expression::StaticMemberExpression(member) = &it.argument {
                self.ok &= !self.is_param(&member.object, 0);
            }
        }
        Step::Walk
    }

    fn tagged(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.ok &= self.context_member(&it.tag).is_none();
    }

    fn this(&mut self, frame: Frame) {
        self.ok &= frame.receiver();
    }

    fn import_meta(&mut self) {
        self.ok = false;
    }

    fn new_target(&mut self, frame: Frame) {
        self.ok &= frame.receiver();
    }

    fn returns(&mut self, frame: Frame) {
        self.ok &= frame.deferred();
    }

    fn with(&mut self) {
        self.ok = false;
    }
}

fn param_symbols(
    scoping: &Scoping,
    params: &FormalParameters<'_>,
) -> Option<[Option<SymbolId>; MAX_PARAMS]> {
    if params.rest.is_some() || params.items.len() > MAX_PARAMS {
        return None;
    }
    let mut symbols = [None; MAX_PARAMS];
    for (at, item) in params.items.iter().enumerate() {
        let BindingPattern::BindingIdentifier(id) = &item.pattern else {
            return None;
        };
        let name = id.name.as_str();
        let placed = NAMES.iter().position(|canon| *canon == name);
        let symbol = id.symbol_id.get()?;
        let fits = item.initializer.is_none()
            && name != REQUIRE
            && placed.is_none_or(|index| index + 1 == at)
            && scoping.symbol_redeclarations(symbol).is_empty();
        if !fits {
            return None;
        }
        symbols[at] = Some(symbol);
    }
    Some(symbols)
}

fn factory(scoping: &Scoping, ids: &FxHashSet<&Id>, def: &Def<'_, '_>) -> bool {
    let Some(params) = param_symbols(scoping, def.params) else {
        return false;
    };
    let mut rules = Rules {
        scoping,
        ids,
        params,
        ok: true,
    };
    Scan::new(&mut rules).visit_function_body(def.body);
    rules.ok
}

pub fn verify(
    source: &Program<'_>,
    layout: &Layout<'_, '_>,
    foreign: Option<&FxHashSet<Id>>,
) -> bool {
    let ids: FxHashSet<&Id> = layout
        .defs
        .iter()
        .map(|def| &def.id)
        .chain(foreign.into_iter().flatten())
        .collect();
    if layout.entries.len() > 1 && layout.defs.iter().any(|def| def.id.key == ENTRY_ID) {
        return false;
    }
    let scoping = build_scoping(source).scoping;
    layout.defs.iter().all(|def| factory(&scoping, &ids, def))
}
