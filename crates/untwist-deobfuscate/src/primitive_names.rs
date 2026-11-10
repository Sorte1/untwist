use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;
use untwist_core::semantic::is_constant_binding;

const FUNCTION_BOUNDARY: ScopeFlags = ScopeFlags::Top
    .union(ScopeFlags::Function)
    .union(ScopeFlags::Arrow)
    .union(ScopeFlags::ClassStaticBlock);

fn is_primitive_literal(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::UnaryNegation
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

struct Collect<'s> {
    scoping: &'s Scoping,
    found: FxHashMap<SymbolId, u32>,
}

impl<'a> VisitJs<'a> for Collect<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let symbol = id.symbol_id.get();
            if let Some(symbol) = symbol.filter(|symbol| {
                is_primitive_literal(init) && is_constant_binding(self.scoping, *symbol)
            }) {
                self.found.insert(symbol, it.span.end);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }
}

#[derive(Default)]
pub struct PrimitiveNames {
    declared_until: FxHashMap<SymbolId, u32>,
}

impl PrimitiveNames {
    pub fn collect(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut collect = Collect {
            scoping,
            found: FxHashMap::default(),
        };
        collect.visit_program(program);
        PrimitiveNames {
            declared_until: collect.found,
        }
    }

    pub fn holds(&self, scoping: &Scoping, symbol: SymbolId, scope: ScopeId, at: u32) -> bool {
        let Some(end) = self.declared_until.get(&symbol) else {
            return false;
        };
        if sees_a_direct_eval(scoping, scope) {
            return false;
        }
        if scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::FunctionScopedVariable)
        {
            return true;
        }
        *end <= at
            && runs_in_the_declaring_function(scoping, scoping.symbol_scope_id(symbol), scope)
    }
}

fn sees_a_direct_eval(scoping: &Scoping, scope: ScopeId) -> bool {
    scoping.scope_ancestors(scope).any(|ancestor| {
        scoping
            .scope_flags(ancestor)
            .contains(ScopeFlags::DirectEval)
    })
}

fn runs_in_the_declaring_function(
    scoping: &Scoping,
    declared: ScopeId,
    reference: ScopeId,
) -> bool {
    if !scoping.scope_flags(declared).intersects(FUNCTION_BOUNDARY) {
        return false;
    }
    scoping
        .scope_ancestors(reference)
        .find(|ancestor| scoping.scope_flags(*ancestor).intersects(FUNCTION_BOUNDARY))
        == Some(declared)
}
