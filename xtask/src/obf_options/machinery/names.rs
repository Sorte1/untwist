use std::collections::{BTreeMap, BTreeSet};

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;

use super::shapes::{function_of, ident_name};

#[derive(Default)]
struct FnFacts {
    params: usize,
    self_assigns: bool,
    holds_strings: bool,
    calls: BTreeSet<String>,
    returns_call_to: Option<String>,
}

#[derive(Default)]
pub(super) struct Names {
    functions: BTreeMap<String, FnFacts>,
    aliases: Vec<(String, String)>,
    pub(super) array_fns: BTreeSet<String>,
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
                facts: &mut facts,
            };
            inner.visit_function_body(body);
        }
        if facts.params == 0 && facts.self_assigns && facts.holds_strings {
            self.array_fns.insert(name.to_string());
        }
        self.functions.insert(name.to_string(), facts);
    }

    pub(super) fn decoders(&self) -> Vec<&str> {
        self.functions
            .iter()
            .filter(|(name, facts)| {
                !self.array_fns.contains(*name)
                    && facts.calls.iter().any(|c| self.array_fns.contains(c))
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
            if let Some(func) = function_of(init) {
                self.record(id.name.as_str(), func);
            } else if let Some(target) = ident_name(init) {
                self.aliases.push((id.name.to_string(), target.to_string()));
            }
        }
        walk::walk_variable_declarator(self, decl);
    }
}

struct FnBody<'n> {
    name: &'n str,
    facts: &'n mut FnFacts,
}

impl<'a> Visit<'a> for FnBody<'_> {
    fn visit_assignment_expression(&mut self, assign: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left {
            if id.name.as_str() == self.name {
                self.facts.self_assigns = true;
            }
        }
        walk::walk_assignment_expression(self, assign);
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
