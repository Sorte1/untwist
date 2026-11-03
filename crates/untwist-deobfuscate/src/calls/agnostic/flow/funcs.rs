use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

pub(super) struct Func {
    pub(super) span_start: u32,
    pub(super) params: Vec<Option<SymbolId>>,
}

pub(super) fn param_symbol(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn func_of(span_start: u32, params: &FormalParameters<'_>) -> Func {
    Func {
        span_start,
        params: params.items.iter().map(param_symbol).collect(),
    }
}

pub(super) struct FuncScan<'s> {
    pub(super) funcs: &'s mut FxHashMap<SymbolId, Func>,
}

impl<'a> VisitJs<'a> for FuncScan<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            self.funcs
                .insert(symbol, func_of(it.span.start, &it.params));
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            let init = it.init.as_ref().map(Expression::without_parentheses);
            let func = match init {
                Some(Expression::FunctionExpression(func)) => {
                    Some(func_of(func.span.start, &func.params))
                }
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    Some(func_of(arrow.span.start, &arrow.params))
                }
                _ => None,
            };
            if let (Some(symbol), Some(func)) = (id.symbol_id.get(), func) {
                self.funcs.insert(symbol, func);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}
