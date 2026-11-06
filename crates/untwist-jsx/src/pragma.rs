use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::{identifier_name, member_parts};
use untwist_core::pass::Cx;

pub const REACT: &str = "React";
const MODULE: &str = "react";
const REQUIRE: &str = "require";
const DEFAULT: &str = "default";
const GLOBALS: [&str; 4] = ["window", "globalThis", "global", "self"];

#[derive(Default)]
pub struct ReactBindings {
    symbols: FxHashSet<SymbolId>,
    shadowing: FxHashSet<SymbolId>,
}

impl ReactBindings {
    pub fn is_react(&self, symbol: SymbolId) -> bool {
        self.symbols.contains(&symbol)
    }

    pub fn is_shadowing(&self, symbol: SymbolId) -> bool {
        self.shadowing.contains(&symbol)
    }

    pub(crate) fn names_member(
        &self,
        expr: &Expression<'_>,
        property: &str,
        cx: &Cx<'_, '_>,
    ) -> bool {
        is_react_member(expr, property, self, !cx.opts.extended, cx.scoping())
    }
}

fn resolve(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

pub(crate) fn is_react_member(
    expr: &Expression<'_>,
    property: &str,
    bindings: &ReactBindings,
    conservative: bool,
    scoping: &Scoping,
) -> bool {
    let Some((object, name)) = member_parts(expr) else {
        return false;
    };
    if name != property {
        return false;
    }
    let Expression::Identifier(id) = object else {
        return false;
    };
    if conservative {
        return id.name.as_str() == REACT;
    }
    match resolve(id, scoping) {
        Some(symbol) if bindings.is_shadowing(symbol) => false,
        Some(symbol) => id.name.as_str() == REACT || bindings.is_react(symbol),
        None => id.name.as_str() == REACT,
    }
}

fn is_react_module(literal: &StringLiteral<'_>) -> bool {
    literal.value == MODULE
}

fn is_react_module_expr(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::StringLiteral(literal) if is_react_module(literal))
}

fn is_direct_require(call: &CallExpression<'_>) -> bool {
    identifier_name(&call.callee) == Some(REQUIRE)
        && call.arguments.len() == 1
        && call.arguments[0]
            .as_expression()
            .is_some_and(is_react_module_expr)
}

fn is_react_require(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::CallExpression(call) if !call.optional => {
            is_direct_require(call)
                || (call.arguments.len() == 1
                    && call.arguments[0]
                        .as_expression()
                        .is_some_and(is_react_require))
        }
        _ => match member_parts(expr) {
            Some((object, name)) if name == DEFAULT => is_react_require(object),
            _ => false,
        },
    }
}

fn is_react_global(expr: &Expression<'_>) -> bool {
    let Some((object, name)) = member_parts(expr) else {
        return false;
    };
    name == REACT && identifier_name(object).is_some_and(|root| GLOBALS.contains(&root))
}

fn is_react_initialiser(expr: &Expression<'_>) -> bool {
    is_react_require(expr) || is_react_global(expr)
}

fn is_default_import(specifier: &ImportSpecifier<'_>) -> bool {
    specifier.imported.name() == DEFAULT
}

#[derive(Default)]
struct Scan {
    symbols: FxHashSet<SymbolId>,
    shadowing: FxHashSet<SymbolId>,
}

impl Scan {
    fn bind(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.symbols.insert(symbol);
        }
    }

    fn shadow(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.shadowing.insert(symbol);
        }
    }
}

impl<'a> VisitJs<'a> for Scan {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if is_react_initialiser(init) {
                self.bind(id);
            } else if id.name.as_str() == REACT {
                self.shadow(id);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        if !is_react_module(&it.source) {
            return;
        }
        for specifier in it.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => self.bind(&spec.local),
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
                    self.bind(&spec.local)
                }
                ImportDeclarationSpecifier::ImportSpecifier(spec) if is_default_import(spec) => {
                    self.bind(&spec.local)
                }
                ImportDeclarationSpecifier::ImportSpecifier(_) => {}
            }
        }
    }
}

pub fn react_bindings(program: &Program<'_>) -> ReactBindings {
    let mut scan = Scan::default();
    scan.visit_program(program);
    ReactBindings {
        symbols: scan.symbols,
        shadowing: scan.shadowing,
    }
}
