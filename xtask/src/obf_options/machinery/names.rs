use std::collections::{BTreeMap, BTreeSet};

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;

use super::shapes::{function_of, ident_name, is_numeric, param_names};

#[derive(Default)]
struct FnFacts {
    params: usize,
    self_assigns: bool,
    holds_strings: bool,
    calls: BTreeSet<String>,
    returns_call_to: Option<String>,
    shifts_param: bool,
    indexed: BTreeSet<String>,
}

#[derive(Default)]
pub(super) struct Names {
    functions: BTreeMap<String, FnFacts>,
    aliases: Vec<(String, String)>,
    pub(super) array_fns: BTreeSet<String>,
    string_vars: BTreeSet<String>,
    pub(super) bare_declarations: Vec<String>,
    pub(super) referenced: BTreeSet<String>,
}

impl Names {
    fn record(&mut self, name: &str, func: &Function) {
        let mut facts = FnFacts {
            params: func.params.items.len(),
            ..FnFacts::default()
        };
        if let Some(body) = &func.body {
            facts.returns_call_to = single_returned_call(&body.statements);
            let mut inner = FnBody {
                name,
                params: owned_params(func),
                facts: &mut facts,
            };
            inner.visit_function_body(body);
        }
        if facts.params == 0 && facts.self_assigns && facts.holds_strings {
            self.array_fns.insert(name.to_string());
        }
        self.functions.insert(name.to_string(), facts);
    }

    fn legacy_arrays_of<'s>(&'s self, facts: &'s FnFacts) -> impl Iterator<Item = &'s String> {
        facts
            .indexed
            .iter()
            .filter(move |array| facts.shifts_param && self.string_vars.contains(*array))
    }

    pub(super) fn legacy_arrays(&self) -> BTreeSet<&str> {
        self.functions
            .values()
            .flat_map(|facts| self.legacy_arrays_of(facts))
            .map(String::as_str)
            .collect()
    }

    pub(super) fn decoders(&self) -> Vec<&str> {
        self.functions
            .iter()
            .filter(|(name, facts)| {
                !self.array_fns.contains(*name)
                    && (facts.calls.iter().any(|c| self.array_fns.contains(c))
                        || self.legacy_arrays_of(facts).next().is_some())
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    pub(super) fn decoder_closure(&self) -> BTreeSet<String> {
        let mut closure: BTreeSet<String> = self.decoders().into_iter().map(String::from).collect();
        loop {
            let before = closure.len();
            for (alias, target) in &self.aliases {
                if closure.contains(target) {
                    closure.insert(alias.clone());
                }
            }
            for (name, facts) in &self.functions {
                if facts
                    .returns_call_to
                    .as_ref()
                    .is_some_and(|c| closure.contains(c))
                {
                    closure.insert(name.clone());
                }
            }
            if closure.len() == before {
                return closure;
            }
        }
    }
}

fn single_returned_call(statements: &[Statement]) -> Option<String> {
    let [Statement::ReturnStatement(ret)] = statements else {
        return None;
    };
    let Some(Expression::CallExpression(call)) =
        ret.argument.as_ref().map(|a| a.without_parentheses())
    else {
        return None;
    };
    ident_name(&call.callee).map(String::from)
}

impl<'a> Visit<'a> for Names {
    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &func.id {
            if matches!(func.r#type, FunctionType::FunctionDeclaration) {
                self.record(id.name.as_str(), func);
            }
        }
        walk::walk_function(self, func, flags);
    }

    fn visit_identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        self.referenced.insert(id.name.to_string());
    }

    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), None) = (&decl.id, &decl.init) {
            self.bare_declarations.push(id.name.to_string());
        }
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&decl.id, &decl.init) {
            if is_string_array(init) {
                self.string_vars.insert(id.name.to_string());
            }
            if let Some(func) = function_of(init) {
                self.record(id.name.as_str(), func);
            } else if let Some(target) = ident_name(init) {
                self.aliases.push((id.name.to_string(), target.to_string()));
            }
        }
        walk::walk_variable_declarator(self, decl);
    }
}

fn is_string_array(init: &Expression) -> bool {
    let Expression::ArrayExpression(array) = init.without_parentheses() else {
        return false;
    };
    !array.elements.is_empty()
        && array
            .elements
            .iter()
            .all(|e| matches!(e, ArrayExpressionElement::StringLiteral(_)))
}

fn owned_params(func: &Function) -> Vec<String> {
    param_names(func)
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect()
}

struct FnBody<'n> {
    name: &'n str,
    params: Vec<String>,
    facts: &'n mut FnFacts,
}

impl FnBody<'_> {
    fn is_param(&self, expr: &Expression) -> bool {
        ident_name(expr).is_some_and(|n| self.params.iter().any(|p| p == n))
    }

    fn shifts_param(&self, assign: &AssignmentExpression) -> bool {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
            return false;
        };
        let name = id.name.as_str();
        if !self.params.iter().any(|p| p == name) {
            return false;
        }
        match (assign.operator, assign.right.without_parentheses()) {
            (AssignmentOperator::Subtraction, right) => is_numeric(right),
            (AssignmentOperator::Assign, Expression::UnaryExpression(unary)) => {
                unary.operator == UnaryOperator::UnaryPlus
                    && ident_name(&unary.argument) == Some(name)
            }
            (AssignmentOperator::Assign, Expression::BinaryExpression(binary)) => {
                binary.operator == BinaryOperator::Subtraction
                    && ident_name(&binary.left) == Some(name)
                    && is_numeric(&binary.right)
            }
            _ => false,
        }
    }

    fn is_param_index(&self, expr: &Expression) -> bool {
        match expr.without_parentheses() {
            Expression::AssignmentExpression(assign) => self.shifts_param(assign),
            other => self.is_param(other),
        }
    }
}

impl<'a> Visit<'a> for FnBody<'_> {
    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        let outer = self.params.len();
        self.params.extend(owned_params(func));
        walk::walk_function(self, func, flags);
        self.params.drain(outer..);
    }

    fn visit_assignment_expression(&mut self, assign: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left {
            if id.name.as_str() == self.name {
                self.facts.self_assigns = true;
            }
        }
        if self.shifts_param(assign) {
            self.facts.shifts_param = true;
        }
        walk::walk_assignment_expression(self, assign);
    }

    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        if let Some(array) = ident_name(&member.object) {
            if self.is_param_index(&member.expression) {
                self.facts.indexed.insert(array.to_string());
            }
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_array_expression(&mut self, array: &ArrayExpression<'a>) {
        if array
            .elements
            .iter()
            .any(|e| matches!(e, ArrayExpressionElement::StringLiteral(_)))
        {
            self.facts.holds_strings = true;
        }
        walk::walk_array_expression(self, array);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(name) = ident_name(&call.callee) {
            self.facts.calls.insert(name.to_string());
        }
        walk::walk_call_expression(self, call);
    }
}
