use std::cell::RefCell;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::js_text::js_text;

const CALLER: &str = "caller";
const ARGUMENTS: &str = "arguments";
const EVAL: &str = "eval";

pub struct Sealed {
    scopes: FxHashSet<ScopeId>,
    memo: RefCell<FxHashMap<ScopeId, bool>>,
}

fn is_equality(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
    )
}

fn is_caller_text(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => js_text(literal.value) == CALLER,
        Expression::TemplateLiteral(template) => {
            template.expressions.is_empty()
                && template
                    .quasis
                    .first()
                    .and_then(|quasi| quasi.value.cooked)
                    .is_some_and(|cooked| js_text(cooked) == CALLER)
        }
        _ => false,
    }
}

fn discarded<'k, 'a>(expression: &'k Expression<'a>) -> &'k Expression<'a> {
    match expression.without_parentheses() {
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::LogicalNot | UnaryOperator::Void
            ) =>
        {
            discarded(&unary.argument)
        }
        other => other,
    }
}

fn function_scope(expression: &Expression<'_>) -> Option<ScopeId> {
    match expression.without_parentheses() {
        Expression::FunctionExpression(function) => function.scope_id.get(),
        Expression::ArrowFunctionExpression(arrow) => arrow.scope_id.get(),
        _ => None,
    }
}

#[derive(Default)]
struct Walker {
    hazard: bool,
    callees: FxHashSet<ReferenceId>,
    unheld: FxHashSet<ScopeId>,
    held: Vec<(ScopeId, SymbolId)>,
    callers: Vec<Span>,
    compared: FxHashSet<Span>,
}

impl<'a> VisitJs<'a> for Walker {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        match it.callee.without_parentheses() {
            Expression::Identifier(callee) if callee.name == EVAL => self.hazard = true,
            Expression::Identifier(callee) if !it.optional => {
                self.callees.extend(callee.reference_id.get());
            }
            other => self.unheld.extend(function_scope(other)),
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.hazard = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.unheld
            .extend(function_scope(discarded(&it.expression)));
        walk_js::walk_expression_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.is_declaration() {
            if let (Some(scope), Some(symbol)) = (
                it.scope_id.get(),
                it.id.as_ref().and_then(|id| id.symbol_id.get()),
            ) {
                self.held.push((scope, symbol));
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let named = matches!(init.without_parentheses(), Expression::FunctionExpression(function) if function.id.is_some());
            if let (Some(scope), Some(symbol), false) =
                (function_scope(init), id.symbol_id.get(), named)
            {
                self.held.push((scope, symbol));
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if is_equality(it.operator) {
            self.compared.insert(it.left.without_parentheses().span());
            self.compared.insert(it.right.without_parentheses().span());
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == CALLER {
            self.callers.push(it.span);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if is_caller_text(&it.expression) {
            self.callers.push(it.span);
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

fn called_only(scoping: &Scoping, callees: &FxHashSet<ReferenceId>, symbol: SymbolId) -> bool {
    let references = scoping.get_resolved_reference_ids(symbol);
    !references.is_empty()
        && scoping.symbol_redeclarations(symbol).is_empty()
        && references.iter().all(|id| callees.contains(id))
}

fn nearest_function(scoping: &Scoping, scope: ScopeId) -> Option<ScopeId> {
    scoping.scope_ancestors(scope).find(|&ancestor| {
        let flags = scoping.scope_flags(ancestor);
        flags.is_function() && !flags.is_arrow()
    })
}

fn own_arguments(scoping: &Scoping) -> FxHashSet<ScopeId> {
    scoping
        .root_unresolved_references()
        .get(ARGUMENTS)
        .into_iter()
        .flatten()
        .filter_map(|&id| nearest_function(scoping, scoping.get_reference(id).scope_id()))
        .collect()
}

fn has_scope_hazard(scoping: &Scoping) -> bool {
    scoping.scope_descendants_from_root().any(|scope| {
        scoping
            .scope_flags(scope)
            .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
    })
}

pub fn sealed(program: &Program<'_>, scoping: &Scoping, callbacks: &[ScopeId]) -> Sealed {
    let empty = || Sealed {
        scopes: FxHashSet::default(),
        memo: RefCell::new(FxHashMap::default()),
    };
    if has_scope_hazard(scoping) {
        return empty();
    }
    let mut walker = Walker::default();
    walker.visit_program(program);
    if walker.hazard {
        return empty();
    }
    let caller_read = walker
        .callers
        .iter()
        .any(|span| !walker.compared.contains(span));
    let script = !program.source_type.is_module();
    let root = scoping.root_scope_id();
    let held = walker.held.iter().filter(|(_, symbol)| {
        let global = script && scoping.symbol_scope_id(*symbol) == root;
        !global && called_only(scoping, &walker.callees, *symbol)
    });
    let arguments = own_arguments(scoping);
    let scopes = walker
        .unheld
        .iter()
        .copied()
        .chain(held.map(|(scope, _)| *scope))
        .chain(callbacks.iter().copied())
        .filter(|scope| !arguments.contains(scope))
        .filter(|&scope| !caller_read || scoping.scope_flags(scope).is_strict_mode())
        .collect();
    Sealed {
        scopes,
        memo: RefCell::new(FxHashMap::default()),
    }
}

impl Sealed {
    pub fn contains(&self, scope: ScopeId) -> bool {
        self.scopes.contains(&scope)
    }

    fn enclosed(&self, scoping: &Scoping, scope: ScopeId) -> bool {
        let mut chain = Vec::new();
        let mut known = true;
        for ancestor in scoping.scope_ancestors(scope) {
            if let Some(&memo) = self.memo.borrow().get(&ancestor) {
                known = memo;
                break;
            }
            chain.push(ancestor);
        }
        let mut memo = self.memo.borrow_mut();
        for ancestor in chain.into_iter().rev() {
            let own =
                !scoping.scope_flags(ancestor).is_function() || self.scopes.contains(&ancestor);
            known = known && own;
            memo.insert(ancestor, known);
        }
        known
    }

    pub fn hides(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        self.enclosed(scoping, scoping.symbol_scope_id(symbol))
            && scoping
                .get_resolved_references(symbol)
                .all(|reference| self.enclosed(scoping, reference.scope_id()))
    }
}
