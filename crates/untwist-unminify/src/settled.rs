use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::semantic::{has_module_syntax, is_constant_binding};

pub fn script_root(program: &Program<'_>, scoping: &Scoping) -> Option<ScopeId> {
    (!has_module_syntax(program)).then(|| scoping.root_scope_id())
}

fn function_of(scoping: &Scoping, scope: ScopeId) -> Option<ScopeId> {
    scoping.scope_ancestors(scope).find(|ancestor| {
        let flags = scoping.scope_flags(*ancestor);
        flags.contains(ScopeFlags::Function) && !flags.contains(ScopeFlags::Arrow)
    })
}

pub fn arguments_can_rebind(scoping: &Scoping, symbol: SymbolId) -> bool {
    if !scoping
        .symbol_flags(symbol)
        .contains(SymbolFlags::FunctionScopedVariable)
    {
        return false;
    }
    let scope = scoping.symbol_scope_id(symbol);
    scoping
        .root_unresolved_references()
        .get("arguments")
        .into_iter()
        .flatten()
        .any(|reference| {
            function_of(scoping, scoping.get_reference(*reference).scope_id()) == Some(scope)
        })
}

fn shared_with_other_scripts(
    scoping: &Scoping,
    symbol: SymbolId,
    script_root: Option<ScopeId>,
) -> bool {
    Some(scoping.symbol_scope_id(symbol)) == script_root
        && !scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::ConstVariable)
}

pub fn is_unwritten_here(
    scoping: &Scoping,
    symbol: SymbolId,
    script_root: Option<ScopeId>,
) -> bool {
    is_constant_binding(scoping, symbol)
        && !scoping.symbol_flags(symbol).contains(SymbolFlags::Import)
        && !shared_with_other_scripts(scoping, symbol, script_root)
}

pub fn is_settled_binding(
    scoping: &Scoping,
    symbol: SymbolId,
    script_root: Option<ScopeId>,
) -> bool {
    is_unwritten_here(scoping, symbol, script_root) && !arguments_can_rebind(scoping, symbol)
}
