use oxc_ast::ast::{Expression, ImportDeclarationSpecifier, Program, Statement};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::ast::expr_symbol;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rt {
    Template,
    Insert,
    CreateComponent,
    Effect,
    Memo,
    DelegateEvents,
    SetAttribute,
    SetAttributeNs,
    SetBoolAttribute,
    ClassName,
    ClassList,
    Style,
    SetStyleProperty,
    AddEventListener,
    Use,
    Spread,
    MergeProps,
    GetOwner,
    Machinery,
    Api(u8),
}

const MACHINERY: &[(&str, &str, Rt)] = &[
    ("template", "template", Rt::Template),
    ("insert", "insert", Rt::Insert),
    ("createComponent", "createComponent", Rt::CreateComponent),
    ("effect", "createRenderEffect", Rt::Effect),
    ("memo", "memo", Rt::Memo),
    ("delegateEvents", "delegateEvents", Rt::DelegateEvents),
    ("setAttribute", "setAttribute", Rt::SetAttribute),
    ("setAttributeNS", "setAttributeNS", Rt::SetAttributeNs),
    ("setBoolAttribute", "setBoolAttribute", Rt::SetBoolAttribute),
    ("className", "className", Rt::ClassName),
    ("classList", "classList", Rt::ClassList),
    ("style", "style", Rt::Style),
    ("setStyleProperty", "setStyleProperty", Rt::SetStyleProperty),
    ("addEventListener", "addEventListener", Rt::AddEventListener),
    ("use", "use", Rt::Use),
    ("spread", "spread", Rt::Spread),
    ("mergeProps", "mergeProps", Rt::MergeProps),
    ("getOwner", "getOwner", Rt::GetOwner),
];

pub const SOLID: &str = "solid-js";
pub const WEB: &str = "solid-js/web";
pub const STORE: &str = "solid-js/store";

pub const API: &[(&str, &str)] = &[
    ("createSignal", SOLID),
    ("createMemo", SOLID),
    ("createEffect", SOLID),
    ("createComputed", SOLID),
    ("createResource", SOLID),
    ("createRoot", SOLID),
    ("createContext", SOLID),
    ("useContext", SOLID),
    ("createSelector", SOLID),
    ("createDeferred", SOLID),
    ("createReaction", SOLID),
    ("createUniqueId", SOLID),
    ("onMount", SOLID),
    ("onCleanup", SOLID),
    ("onError", SOLID),
    ("untrack", SOLID),
    ("batch", SOLID),
    ("on", SOLID),
    ("children", SOLID),
    ("splitProps", SOLID),
    ("lazy", SOLID),
    ("runWithOwner", SOLID),
    ("startTransition", SOLID),
    ("useTransition", SOLID),
    ("observable", SOLID),
    ("from", SOLID),
    ("mapArray", SOLID),
    ("indexArray", SOLID),
    ("catchError", SOLID),
    ("Show", SOLID),
    ("For", SOLID),
    ("Index", SOLID),
    ("Switch", SOLID),
    ("Match", SOLID),
    ("ErrorBoundary", SOLID),
    ("Suspense", SOLID),
    ("SuspenseList", SOLID),
    ("render", WEB),
    ("Portal", WEB),
    ("Dynamic", WEB),
    ("createDynamic", WEB),
    ("createStore", STORE),
    ("produce", STORE),
    ("reconcile", STORE),
    ("unwrap", STORE),
    ("createMutable", STORE),
    ("modifyMutable", STORE),
];

impl Rt {
    pub fn spelling(self) -> Option<(&'static str, &'static str)> {
        match self {
            Rt::Effect => Some(("createRenderEffect", SOLID)),
            Rt::MergeProps => Some(("mergeProps", SOLID)),
            Rt::GetOwner => Some(("getOwner", SOLID)),
            Rt::Api(at) => API.get(usize::from(at)).copied(),
            _ => None,
        }
    }
}

fn bare(name: &str) -> &str {
    match name.rfind('$') {
        Some(at)
            if at > 0
                && at + 1 < name.len()
                && name[at + 1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            &name[..at]
        }
        _ => name,
    }
}

fn bundled(name: &str) -> Option<Rt> {
    if let Some(row) = MACHINERY.iter().find(|row| row.1 == name) {
        return Some(row.2);
    }
    API.iter()
        .position(|(api, _)| *api == name)
        .map(|at| Rt::Api(at as u8))
}

pub fn labelled(name: &str) -> Option<&'static str> {
    let name = bare(name);
    MACHINERY
        .iter()
        .map(|row| row.1)
        .chain(API.iter().map(|row| row.0))
        .find(|known| *known == name)
}

pub fn bundled_name(name: &str) -> Option<Rt> {
    bundled(name)
}

#[derive(Default)]
pub struct Runtime {
    pub symbols: FxHashMap<SymbolId, Rt>,
    pub ambiguous: Vec<SymbolId>,
    pub esm: bool,
    pub minified: bool,
}

impl Runtime {
    pub fn of(program: &Program<'_>, scoping: &Scoping, nodes: &AstNodes<'_>) -> Self {
        let mut runtime = Runtime::default();
        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            if import.source.value.as_str() != Some(WEB) {
                continue;
            }
            for specifier in import.specifiers.iter().flatten() {
                let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier else {
                    continue;
                };
                if !named.local.name.starts_with("_$") {
                    continue;
                }
                runtime.esm = true;
                let imported = named.imported.name();
                let rt = MACHINERY
                    .iter()
                    .find(|row| row.0 == imported.as_str())
                    .map_or(Rt::Machinery, |row| row.2);
                runtime.symbols.insert(named.local.symbol_id(), rt);
            }
        }
        if !runtime.esm {
            runtime.named(scoping, nodes);
        }
        if !runtime.esm && runtime.symbols.is_empty() && runtime.ambiguous.is_empty() {
            runtime.fingerprinted(program.source_text, scoping, nodes);
        }
        runtime
    }

    fn named(&mut self, scoping: &Scoping, nodes: &AstNodes<'_>) {
        let mut candidates: FxHashMap<&str, Vec<SymbolId>> = FxHashMap::default();
        for symbol in scoping.symbol_ids() {
            let name = bare(scoping.symbol_name(symbol));
            if bundled(name).is_some() && is_function_binding(scoping, nodes, symbol) {
                candidates.entry(name).or_default().push(symbol);
            }
        }
        let Some([template]) = candidates.get("template").map(Vec::as_slice) else {
            return;
        };
        let template = *template;
        let home = scoping.symbol_scope_id(template);
        let zones = untwist_restore::region::template_zones(scoping, nodes, template, home);
        for symbols in candidates.values_mut() {
            symbols.retain(|symbol| {
                !untwist_restore::region::declared_in(scoping, nodes, *symbol, &zones)
            });
        }
        candidates.retain(|_, symbols| !symbols.is_empty());
        if !candidates.contains_key("insert") && !candidates.contains_key("createComponent") {
            return;
        }
        for (name, symbols) in candidates {
            let at_home: Vec<SymbolId> = symbols
                .iter()
                .copied()
                .filter(|symbol| scoping.symbol_scope_id(*symbol) == home)
                .collect();
            match (at_home.as_slice(), bundled(name)) {
                ([only], Some(rt)) if symbols.len() == 1 => {
                    self.symbols.insert(*only, rt);
                }
                ([], _) => {}
                _ => self.ambiguous.extend(at_home),
            }
        }
    }

    fn fingerprinted(&mut self, source: &str, scoping: &Scoping, nodes: &AstNodes<'_>) {
        let marks = untwist_restore::fingerprint::Marks::of(nodes, scoping);
        let prints = untwist_restore::fingerprint::bound_functions(source, &marks, scoping, nodes);
        let mut found: FxHashMap<Rt, Vec<SymbolId>> = FxHashMap::default();
        for (symbol, print) in prints {
            if let Some(rt) = crate::fingerprints::lookup(print.hash) {
                found.entry(rt).or_default().push(symbol);
            }
        }
        let Some([template]) = found.get(&Rt::Template).map(Vec::as_slice) else {
            return;
        };
        let template = *template;
        let home = scoping.symbol_scope_id(template);
        let zones = untwist_restore::region::template_zones(scoping, nodes, template, home);
        for symbols in found.values_mut() {
            symbols.retain(|symbol| {
                !untwist_restore::region::declared_in(scoping, nodes, *symbol, &zones)
            });
        }
        found.retain(|_, symbols| !symbols.is_empty());
        self.minified = true;
        for (rt, symbol) in loops(&found, scoping, nodes, home, &zones) {
            found.insert(rt, vec![symbol]);
        }
        for (rt, symbols) in found {
            match symbols.as_slice() {
                [only] if scoping.symbol_scope_id(*only) == home => {
                    self.symbols.insert(*only, rt);
                }
                _ => self.ambiguous.extend(symbols),
            }
        }
    }
}

fn loops(
    found: &FxHashMap<Rt, Vec<SymbolId>>,
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
    home: ScopeId,
    zones: &[(u32, u32)],
) -> Vec<(Rt, SymbolId)> {
    let only = |name: &str| -> Option<SymbolId> {
        let rt = bundled(name)?;
        match found.get(&rt).map(Vec::as_slice) {
            Some([symbol]) => Some(*symbol),
            _ => None,
        }
    };
    let (Some(memo), Some(map), Some(index)) =
        (only("createMemo"), only("mapArray"), only("indexArray"))
    else {
        return Vec::new();
    };
    let mut picks: Vec<(Rt, SymbolId)> = Vec::new();
    for symbol in scoping.get_bindings(home).values().copied() {
        if found.values().any(|symbols| symbols.contains(&symbol))
            || untwist_restore::region::declared_in(scoping, nodes, symbol, zones)
        {
            continue;
        }
        let AstKind::Function(function) = nodes.kind(scoping.symbol_declaration(symbol)) else {
            continue;
        };
        let [Statement::VariableDeclaration(_), Statement::ReturnStatement(ret)] = function
            .body
            .as_ref()
            .map_or(&[][..], |body| body.statements.as_slice())
        else {
            continue;
        };
        let Some(Expression::CallExpression(outer)) =
            ret.argument.as_ref().map(Expression::without_parentheses)
        else {
            continue;
        };
        let Some(Expression::CallExpression(inner)) = outer
            .arguments
            .first()
            .and_then(|arg| arg.as_expression())
            .map(Expression::without_parentheses)
        else {
            continue;
        };
        if expr_symbol(&outer.callee, scoping) != Some(memo) || function.params.items.len() != 1 {
            continue;
        }
        let callee = expr_symbol(&inner.callee, scoping);
        let name = match callee {
            Some(symbol) if symbol == map => "For",
            Some(symbol) if symbol == index => "Index",
            _ => continue,
        };
        if let Some(rt) = bundled(name) {
            picks.push((rt, symbol));
        }
    }
    picks
        .iter()
        .filter(|(rt, _)| {
            picks.iter().filter(|(other, _)| other == rt).count() == 1 && !found.contains_key(rt)
        })
        .copied()
        .collect()
}

impl Runtime {
    pub fn home(&self, scoping: &Scoping) -> Option<ScopeId> {
        self.symbols
            .iter()
            .find(|(_, rt)| **rt == Rt::Template)
            .map(|(symbol, _)| scoping.symbol_scope_id(*symbol))
    }

    pub fn resolve(&self, expr: &Expression<'_>, scoping: &Scoping) -> Option<Rt> {
        self.symbols.get(&expr_symbol(expr, scoping)?).copied()
    }

    pub fn is(&self, expr: &Expression<'_>, scoping: &Scoping, rt: Rt) -> bool {
        self.resolve(expr, scoping) == Some(rt)
    }
}

pub fn is_function_binding(scoping: &Scoping, nodes: &AstNodes<'_>, symbol: SymbolId) -> bool {
    if scoping.symbol_flags(symbol).is_function() {
        return true;
    }
    match nodes.kind(scoping.symbol_declaration(symbol)) {
        AstKind::VariableDeclarator(declarator) => matches!(
            declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses),
            Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
        ),
        _ => false,
    }
}
