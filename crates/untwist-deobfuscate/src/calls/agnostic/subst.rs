use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_span::Span;

use super::sites::*;
use super::AKey;

#[derive(Default)]
struct ArgumentsUse {
    found: bool,
}

impl<'a> VisitJs<'a> for ArgumentsUse {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {}

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

const MAX_INLINE_DEPTH: usize = 64;

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn proxy_str(
        &self,
        call: &'a CallExpression<'a>,
        depth: u8,
    ) -> Option<(&'a Function<'a>, &'a Expression<'a>)> {
        let value = self.sealed_str_read(&call.callee, depth)?;
        let Expression::FunctionExpression(function) = value.without_parentheses() else {
            return None;
        };
        let body = function.body.as_ref()?;
        let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
            return None;
        };
        Some((function, ret.argument.as_ref()?))
    }

    pub(super) fn substituted(&self, id: &IdentifierReference<'_>) -> Option<&'a Expression<'a>> {
        let symbol = self.fresh.symbol(id)?;
        self.subst
            .borrow()
            .iter()
            .rev()
            .find(|(param, _)| *param == symbol)
            .map(|(_, arg)| *arg)
    }

    pub(super) fn proxy<'k>(
        &self,
        call: &'k CallExpression<'a>,
    ) -> Option<(&'a Function<'a>, &'a Expression<'a>)> {
        let value = self.sealed_read(&call.callee)?;
        let Expression::FunctionExpression(function) = value.without_parentheses() else {
            return None;
        };
        let body = function.body.as_ref()?;
        let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
            return None;
        };
        Some((function, ret.argument.as_ref()?))
    }

    pub(super) fn with_params<R>(
        &self,
        function: &Function<'a>,
        call: &CallExpression<'a>,
        body: impl FnOnce() -> R,
    ) -> Option<R> {
        let depth = self.subst.borrow().len();
        if !self.bind_params(function, call) {
            self.subst.borrow_mut().truncate(depth);
            return None;
        }
        let result = body();
        self.subst.borrow_mut().truncate(depth);
        Some(result)
    }

    fn bind_params(&self, function: &Function<'a>, call: &CallExpression<'a>) -> bool {
        let aliased = self.arguments_alias_params(function);
        for (index, param) in function.params.items.iter().enumerate() {
            let BindingPattern::BindingIdentifier(id) = &param.pattern else {
                return false;
            };
            if param.initializer.is_some() {
                return false;
            }
            let (Some(symbol), Some(arg)) = (
                id.symbol_id.get(),
                call.arguments.get(index).and_then(Argument::as_expression),
            ) else {
                return false;
            };
            if aliased || self.fresh.scoping.symbol_is_mutated(symbol) {
                continue;
            }
            let arg = self.alloc_expr(arg);
            self.subst.borrow_mut().push((symbol, arg));
        }
        true
    }

    fn arguments_alias_params(&self, function: &Function<'a>) -> bool {
        if !self
            .fresh
            .scoping
            .root_unresolved_references()
            .contains_key("arguments")
        {
            return false;
        }
        let mut scan = ArgumentsUse::default();
        if let Some(body) = &function.body {
            for statement in &body.statements {
                scan.visit_statement(statement);
            }
        }
        scan.found
    }

    pub(super) fn alloc_expr(&self, expr: &Expression<'a>) -> &'a Expression<'a> {
        self.alloc(expr)
    }

    pub(super) fn inline_helper(
        &mut self,
        func: &'a Function<'a>,
        symbol: oxc_syntax::symbol::SymbolId,
        call: &CallExpression<'a>,
    ) {
        let depth = self.subst.borrow().len();
        for (index, param) in func.params.items.iter().enumerate() {
            if let BindingPattern::BindingIdentifier(id) = &param.pattern {
                if let (Some(psym), Some(arg)) = (
                    id.symbol_id.get(),
                    call.arguments.get(index).and_then(Argument::as_expression),
                ) {
                    let arg = self.alloc_expr(arg);
                    self.subst.borrow_mut().push((psym, arg));
                }
            }
        }
        self.inlining.push(symbol);
        if let Some(body) = &func.body {
            for statement in &body.statements {
                self.visit_statement(statement);
            }
        }
        self.inlining.pop();
        self.subst.borrow_mut().truncate(depth);
    }

    pub(super) fn afford_inline(&mut self, func: &Function<'a>) -> bool {
        let cost = func.span.size() as usize;
        if self.inlining.len() >= MAX_INLINE_DEPTH || cost > self.inline_budget {
            return false;
        }
        self.inline_budget -= cost;
        true
    }

    pub(super) fn inline_exhausted(&mut self, span: Span) {
        self.out.code_text.get_or_insert(span);
        if !self.text_judged {
            self.out.reach_code.get_or_insert(span);
        }
        self.saturated = true;
    }

    pub(super) fn inline_target(
        &self,
        call: &CallExpression<'a>,
    ) -> Option<(&'a Function<'a>, oxc_syntax::symbol::SymbolId)> {
        if call.optional {
            return None;
        }
        let Expression::Identifier(id) = call.callee.without_parentheses() else {
            return None;
        };
        let symbol = self.fresh.symbol(id)?;
        if self.inlining.contains(&symbol) {
            return None;
        }
        let func = self.fresh.inlinable_helper(id)?;
        Some((func, symbol))
    }

    pub(super) fn recursive_inline(
        &self,
        call: &CallExpression<'a>,
    ) -> Option<(&'a Function<'a>, oxc_syntax::symbol::SymbolId)> {
        if call.optional {
            return None;
        }
        let Expression::Identifier(id) = call.callee.without_parentheses() else {
            return None;
        };
        let symbol = self.fresh.symbol(id)?;
        if !self.inlining.contains(&symbol) {
            return None;
        }
        Some((self.fresh.inlinable_helper(id)?, symbol))
    }

    pub(super) fn walk_unbound(
        &mut self,
        (func, symbol): (&'a Function<'a>, oxc_syntax::symbol::SymbolId),
        call: &CallExpression<'a>,
    ) {
        for argument in &call.arguments {
            self.visit_argument(argument);
        }
        if !self.walked_unbound.insert(symbol) {
            return;
        }
        let outer = std::mem::take(&mut *self.subst.borrow_mut());
        if let Some(body) = &func.body {
            for statement in &body.statements {
                self.visit_statement(statement);
            }
        }
        *self.subst.borrow_mut() = outer;
    }

    pub(super) fn proxy_key(&self, call: &CallExpression<'a>, depth: u8) -> Option<AKey<'a>> {
        let (function, body) = self.proxy(self.alloc(call))?;
        self.with_params(function, call, || self.key_class(body, depth))
    }

    pub(super) fn unsubstituted(&self, expr: &'a Expression<'a>) -> &'a Expression<'a> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.substituted(id).unwrap_or(expr),
            _ => expr,
        }
    }

    pub(super) fn substituted_param(
        &self,
        function: &'a Function<'a>,
        call: &CallExpression<'a>,
        id: &IdentifierReference<'_>,
    ) -> Option<&'a Expression<'a>> {
        let symbol = self.fresh.symbol(id)?;
        let index = function.params.items.iter().position(|param| {
            matches!(&param.pattern, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol))
        })?;
        call.arguments
            .get(index)
            .and_then(Argument::as_expression)
            .map(|arg| self.alloc_expr(arg))
    }
}
