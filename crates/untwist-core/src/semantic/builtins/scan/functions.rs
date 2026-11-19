use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) type FunctionBindings = FxHashSet<SymbolId>;

#[derive(Default)]
struct Count {
    functions: usize,
    bindings: usize,
}

#[derive(Default)]
struct Bindings {
    counts: FxHashMap<SymbolId, Count>,
}

pub(super) fn function_bindings(program: &Program<'_>) -> FunctionBindings {
    let mut found = Bindings::default();
    found.visit_program(program);
    found
        .counts
        .into_iter()
        .filter(|(_, count)| count.functions == count.bindings)
        .map(|(symbol, _)| symbol)
        .collect()
}

fn is_function(init: Option<&Expression<'_>>) -> bool {
    matches!(
        init.map(Expression::without_parentheses),
        Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_))
    )
}

impl Bindings {
    fn binds_a_function(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.counts.entry(symbol).or_default().functions += 1;
        }
    }
}

impl Bindings {
    fn rebinds_each_turn(&mut self, left: &ForStatementLeft<'_>) {
        let ForStatementLeft::VariableDeclaration(declaration) = left else {
            return;
        };
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                if let Some(symbol) = id.symbol_id.get() {
                    self.counts.entry(symbol).or_default().bindings += 1;
                }
            }
        }
    }
}

impl<'a> VisitJs<'a> for Bindings {
    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.rebinds_each_turn(&it.left);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.rebinds_each_turn(&it.left);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.counts.entry(symbol).or_default().bindings += 1;
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.binds_a_function(id);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            if is_function(it.init.as_ref()) {
                self.binds_a_function(id);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}
