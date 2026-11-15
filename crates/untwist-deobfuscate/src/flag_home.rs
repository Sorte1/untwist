use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;

pub const DIRECT_EVAL: &str = "eval";
pub const ARGUMENTS: &str = "arguments";

#[derive(Default)]
pub struct Homes {
    stack: Vec<(ScopeId, bool)>,
    params: Vec<(ScopeId, Span)>,
    hazards: Vec<ScopeId>,
}

impl Homes {
    pub fn enter(&mut self, scope: Option<ScopeId>, params: Span, arrow: bool) {
        if let Some(scope) = scope {
            self.params.push((scope, params));
            self.stack.push((scope, arrow));
        }
    }

    pub fn leave(&mut self, scope: Option<ScopeId>) {
        if scope.is_some() {
            self.stack.pop();
        }
    }

    pub fn reaches_every_frame(&mut self) {
        self.hazards
            .extend(self.stack.iter().map(|(scope, _)| *scope));
    }

    pub fn reads_arguments(&mut self) {
        for (scope, arrow) in self.stack.iter().rev() {
            self.hazards.push(*scope);
            if !arrow {
                break;
            }
        }
    }
}

fn owning_function(scoping: &Scoping, symbol: SymbolId) -> Option<ScopeId> {
    let mut scope = scoping.symbol_scope_id(symbol);
    loop {
        let flags = scoping.scope_flags(scope);
        if flags.contains(ScopeFlags::Function) {
            return Some(scope);
        }
        if flags.intersects(ScopeFlags::Top | ScopeFlags::ClassStaticBlock) {
            return None;
        }
        scope = scoping.scope_parent_id(scope)?;
    }
}

pub fn is_private_local(homes: &Homes, scoping: &Scoping, symbol: SymbolId) -> bool {
    if !scoping.symbol_flags(symbol).is_variable() {
        return false;
    }
    let Some(scope) = owning_function(scoping, symbol) else {
        return false;
    };
    let Some((_, params)) = homes.params.iter().find(|(home, _)| *home == scope) else {
        return false;
    };
    let declared = scoping.symbol_span(symbol);
    let in_params = params.start <= declared.start && declared.end <= params.end;
    !in_params && !homes.hazards.contains(&scope)
}
