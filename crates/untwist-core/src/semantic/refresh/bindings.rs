use oxc_ast::ast::BindingIdentifier;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

use super::{Context, Region, Walk};

struct Expected {
    includes: SymbolFlags,
    scope: ScopeId,
    deferred: bool,
    early: bool,
}

impl Walk<'_, '_> {
    pub(super) fn bind(&mut self, it: &BindingIdentifier<'_>) {
        if self.failed {
            return;
        }
        let symbol = it
            .symbol_id
            .get()
            .filter(|symbol| symbol.index() < self.scoping.symbols_len());
        let (Some(context), Some(symbol)) = (self.context, symbol) else {
            return self.fail();
        };
        let Some(expected) = self.expected(context) else {
            return self.fail();
        };
        let same = if self.symbols.claim(symbol.index()) {
            self.first_declaration(symbol, it, &expected)
        } else {
            self.redeclaration(symbol, it, &expected)
        };
        if !same {
            return self.fail();
        }
        if expected.early {
            self.early.push(symbol);
        }
    }

    fn expected(&self, context: Context) -> Option<Expected> {
        let current = self.frames.last()?;
        let at = |scope: ScopeId, includes: SymbolFlags| Expected {
            includes,
            scope,
            deferred: false,
            early: false,
        };
        match context {
            Context::Var => {
                let target = self.frames.iter().rev().find(|f| f.expected.is_var())?;
                Some(at(target.scope, SymbolFlags::FunctionScopedVariable))
            }
            Context::Lexical(includes) => Some(at(current.scope, includes)),
            Context::Param => (current.region == Region::Function && current.in_params)
                .then(|| at(current.scope, SymbolFlags::FunctionScopedVariable)),
            Context::FunctionName {
                declaration,
                includes,
            } => self.function_name(declaration, includes),
            Context::ClassName {
                declaration: true, ..
            } => Some(at(current.scope, SymbolFlags::Class)),
            Context::ClassName {
                declaration: false,
                scope,
            } => scope
                .filter(|scope| scope.index() < self.scoping.scopes_len())
                .map(|scope| at(scope, SymbolFlags::Class)),
            Context::CatchParam(includes) => {
                (current.region == Region::Catch).then_some(Expected {
                    includes,
                    scope: current.scope,
                    deferred: true,
                    early: false,
                })
            }
        }
    }

    fn function_name(&self, declaration: bool, includes: SymbolFlags) -> Option<Expected> {
        let own = self.frames.last()?;
        if !declaration {
            return Some(Expected {
                includes,
                scope: own.scope,
                deferred: false,
                early: true,
            });
        }
        let parent = self.frames.iter().rev().nth(1)?;
        let plain = !includes.contains(SymbolFlags::AsyncOrGeneratorFunction);
        let annex_b = plain && !parent.expected.is_var() && !parent.expected.is_strict_mode();
        (!annex_b).then_some(Expected {
            includes,
            scope: parent.scope,
            deferred: false,
            early: false,
        })
    }

    fn first_declaration(
        &mut self,
        symbol: SymbolId,
        it: &BindingIdentifier<'_>,
        expected: &Expected,
    ) -> bool {
        let scoping = self.scoping;
        let named = scoping.symbol_ident(symbol) == it.name;
        let placed = expected.deferred
            || (scoping.symbol_scope_id(symbol) == expected.scope
                && scoping.get_binding(expected.scope, it.name) == Some(symbol));
        let redeclarations = scoping.symbol_redeclarations(symbol);
        let first = redeclarations
            .first()
            .is_none_or(|first| first.flags == expected.includes && first.span == it.span);
        if !redeclarations.is_empty() {
            self.redeclared.insert(symbol, 1);
        }
        named
            && placed
            && first
            && scoping.symbol_flags(symbol) == expected.includes
            && scoping.symbol_span(symbol) == it.span
    }

    fn redeclaration(
        &mut self,
        symbol: SymbolId,
        it: &BindingIdentifier<'_>,
        expected: &Expected,
    ) -> bool {
        let scoping = self.scoping;
        let Some(seen) = self.redeclared.get_mut(&symbol) else {
            return false;
        };
        let Some(entry) = scoping.symbol_redeclarations(symbol).get(*seen) else {
            return false;
        };
        *seen += 1;
        entry.flags == expected.includes
            && entry.span == it.span
            && scoping.symbol_ident(symbol) == it.name
            && scoping.symbol_flags(symbol) == expected.includes
            && (expected.deferred || scoping.symbol_scope_id(symbol) == expected.scope)
    }
}
