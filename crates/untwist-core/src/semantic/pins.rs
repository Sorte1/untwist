use oxc_semantic::Scoping;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

pub fn script_global_symbols(scoping: &Scoping) -> impl Iterator<Item = SymbolId> + '_ {
    let declared = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
    scoping
        .get_bindings(scoping.root_scope_id())
        .iter()
        .map(|(_, symbol)| *symbol)
        .filter(move |symbol| scoping.symbol_flags(*symbol).intersects(declared))
}
