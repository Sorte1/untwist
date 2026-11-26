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

#[derive(Default)]
pub struct Runtime {
    pub symbols: FxHashMap<SymbolId, Rt>,
    pub ambiguous: Vec<SymbolId>,
    pub esm: bool,
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
        let home = scoping.symbol_scope_id(*template);
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
