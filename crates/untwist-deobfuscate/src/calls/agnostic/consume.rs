use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::js_text::js_text;

use super::fresh::Bound;
use super::live::{iife_returned, opaque_branch};
use super::sites::Sites;
use untwist_eval::value::JsString;

type Resolve<'s> = Option<&'s FxHashMap<Span, &'s JsString>>;

fn param_symbols(func: &Function<'_>) -> FxHashMap<SymbolId, usize> {
    let mut map = FxHashMap::default();
    for (index, param) in func.params.items.iter().enumerate() {
        if let BindingPattern::BindingIdentifier(id) = &param.pattern {
            if let Some(symbol) = id.symbol_id.get() {
                map.insert(symbol, index);
            }
        }
    }
    map
}

fn nth_param_symbol(func: &Function<'_>, index: usize) -> Option<SymbolId> {
    match &func.params.items.get(index)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

struct ParamUse<'s> {
    scoping: &'s Scoping,
    resolve: Resolve<'s>,
    target: SymbolId,
    params: &'s FxHashMap<SymbolId, usize>,
    applied: Vec<usize>,
    bad: bool,
}

impl ParamUse<'_> {
    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn is_target(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if self.symbol_of(id) == Some(self.target))
    }

    fn resolved_key(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::StringLiteral(text) = expr.without_parentheses() {
            return Some(js_text(text.value).into_owned());
        }
        self.resolve?
            .get(&expr.span())
            .and_then(|value| value.as_string())
    }

    fn receiver_apply<'a>(&self, call: &CallExpression<'a>) -> Option<SymbolId> {
        let member = call.callee.without_parentheses().as_member_expression()?;
        let key = match member {
            MemberExpression::StaticMemberExpression(m) => m.property.name.to_string(),
            MemberExpression::ComputedMemberExpression(m) => self.resolved_key(&m.expression)?,
            MemberExpression::PrivateFieldExpression(_) => return None,
        };
        if key != "apply" && key != "call" {
            return None;
        }
        let Expression::Identifier(base) = member.object().without_parentheses() else {
            return None;
        };
        let receiver = call.arguments.first().and_then(Argument::as_expression)?;
        self.is_target(receiver).then_some(())?;
        self.symbol_of(base)
    }
}

impl<'a> VisitJs<'a> for ParamUse<'_> {
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match opaque_branch(it) {
            Some(taken) => {
                if let Some(branch) = taken {
                    self.visit_statement(branch);
                }
            }
            None => walk_js::walk_if_statement(self, it),
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.bad {
            return;
        }
        if let Some(base) = self.receiver_apply(it) {
            match self.params.get(&base) {
                Some(index) => self.applied.push(*index),
                None => {
                    self.bad = true;
                    return;
                }
            }
            for argument in it.arguments.iter().skip(1) {
                if let Some(expr) = argument.as_expression() {
                    self.visit_expression(expr);
                }
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.symbol_of(it) == Some(self.target) {
            self.bad = true;
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            if self.symbol_of(id) == Some(self.target) {
                self.bad = true;
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }
}

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn resolve_called_function<'k>(
        &self,
        callee: &'k Expression<'a>,
    ) -> Option<&'k Function<'a>> {
        if let Some(func) = self.fresh.callee_function(callee) {
            return Some(func);
        }
        let Expression::Identifier(id) = callee.without_parentheses() else {
            return None;
        };
        iife_returned(self.fresh.literal_init(id)?)
    }

    pub(super) fn arg_consumed_safely(
        &self,
        callee: &Expression<'a>,
        arguments: &[Argument<'a>],
        index: usize,
    ) -> bool {
        let Some(func) = self.resolve_called_function(callee) else {
            return false;
        };
        let Some(body) = func.body.as_ref() else {
            return false;
        };
        let Some(target) = nth_param_symbol(func, index) else {
            return false;
        };
        let params = param_symbols(func);
        let mut scan = ParamUse {
            scoping: self.fresh.scoping,
            resolve: self.resolve,
            target,
            params: &params,
            applied: Vec::new(),
            bad: false,
        };
        for statement in &body.statements {
            scan.visit_statement(statement);
        }
        if scan.bad || arguments.iter().any(|arg| arg.as_expression().is_none()) {
            return false;
        }
        scan.applied.iter().all(|index| {
            arguments
                .get(*index)
                .and_then(Argument::as_expression)
                .is_some_and(|expr| self.function_value(expr))
        })
    }
}

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn function_value(&self, expr: &Expression<'a>) -> bool {
        match expr.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(id) if self.substituted(id).is_none() => {
                matches!(self.fresh.bound_value(id), Some(Bound::Function))
            }
            _ => false,
        }
    }

    pub(super) fn applies_user_function(&self, callee: &Expression<'a>) -> Option<&'static str> {
        let member = callee.without_parentheses().as_member_expression()?;
        let key = match member {
            MemberExpression::StaticMemberExpression(m) => m.property.name.as_str(),
            MemberExpression::ComputedMemberExpression(m) => {
                match m.expression.without_parentheses() {
                    Expression::StringLiteral(text) => text.value.as_str()?,
                    _ => return None,
                }
            }
            MemberExpression::PrivateFieldExpression(_) => return None,
        };
        let key = match key {
            "apply" => "apply",
            "call" => "call",
            _ => return None,
        };
        let target = member.object().without_parentheses();
        let user = self.function_value(target)
            || matches!(target, Expression::Identifier(id) if self.param_holds_functions(id));
        user.then_some(key)
    }

    fn param_holds_functions(&self, id: &IdentifierReference<'a>) -> bool {
        if self.substituted(id).is_some() {
            return false;
        }
        let Some(symbol) = self.fresh.symbol(id) else {
            return false;
        };
        let bindings = self.fresh.bindings;
        let Some((func, index)) = bindings.params.get(&symbol).copied() else {
            return false;
        };
        let primitive_writes = bindings.defs.get(symbol).is_some_and(|defs| {
            !defs.fields_written
                && defs.writes.iter().all(|value| {
                    value.is_some_and(|value| {
                        matches!(
                            value.without_parentheses(),
                            Expression::NullLiteral(_)
                                | Expression::BooleanLiteral(_)
                                | Expression::NumericLiteral(_)
                                | Expression::StringLiteral(_)
                        )
                    })
                })
        });
        primitive_writes
            && bindings.does_not_escape(func)
            && bindings
                .call_sites(func)
                .all(|call| match call.arguments.get(index) {
                    None => true,
                    Some(argument) => argument
                        .as_expression()
                        .is_some_and(|value| self.function_value(value)),
                })
    }
}
