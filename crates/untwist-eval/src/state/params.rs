use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentOperator, AssignmentTarget, BindingPattern,
    ComputedMemberExpression, Declaration, Expression, FormalParameters, Function, FunctionBody,
};
use oxc_ast::ast::{IdentifierReference, Statement};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;

use super::recipe::{fold_constant, Prim};
use crate::stack;

struct ParamUsage<'a> {
    names: Vec<Option<&'a str>>,
    used: Vec<bool>,
    shadowed: Vec<Vec<&'a str>>,
}

fn declared_names<'a>(statements: &'a [Statement<'a>]) -> Vec<&'a str> {
    let mut names = Vec::new();
    for statement in statements {
        let Some(declaration) = statement.as_declaration() else {
            continue;
        };
        match declaration {
            Declaration::VariableDeclaration(declaration) => names.extend(
                declaration
                    .declarations
                    .iter()
                    .filter_map(|declarator| param_name(&declarator.id)),
            ),
            Declaration::FunctionDeclaration(function) => {
                names.extend(function.id.as_ref().map(|id| id.name.as_str()));
            }
            Declaration::ClassDeclaration(class) => {
                names.extend(class.id.as_ref().map(|id| id.name.as_str()));
            }
            _ => {}
        }
    }
    names
}

impl<'a> VisitJs<'a> for ParamUsage<'a> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let own_name = it
            .id
            .as_ref()
            .filter(|_| it.is_expression())
            .map(|id| id.name.as_str());
        self.shadowed.push(own_name.into_iter().collect());
        walk_js::walk_function(self, it, flags);
        self.shadowed.pop();
        self.shadowed.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        walk_js::walk_arrow_function_expression(self, it);
        self.shadowed.pop();
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        let params: &'a FormalParameters<'a> = self.alloc(it);
        let names = params
            .items
            .iter()
            .filter_map(|param| param_name(&param.pattern))
            .collect();
        self.shadowed.push(names);
        walk_js::walk_formal_parameters(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        let body: &'a FunctionBody<'a> = self.alloc(it);
        self.shadowed.push(declared_names(&body.statements));
        walk_js::walk_function_body(self, it);
        self.shadowed.pop();
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name.as_str() == "arguments" {
            self.used.iter_mut().for_each(|used| *used = true);
            return;
        }
        let name = it.name.as_str();
        let nested_skipped = crate::mutants::nested_names_rebound() && !self.shadowed.is_empty();
        if nested_skipped || self.shadowed.iter().flatten().any(|shadow| *shadow == name) {
            return;
        }
        if let Some(index) = self.names.iter().position(|slot| *slot == Some(name)) {
            self.used[index] = true;
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if is_arguments(&it.object) {
            match literal_index(&it.expression) {
                Some(index) if index < self.used.len() => self.used[index] = true,
                _ => self.used.iter_mut().for_each(|used| *used = true),
            }
            self.visit_expression(&it.expression);
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

fn is_arguments(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name.as_str() == "arguments")
}

fn literal_index(expr: &Expression<'_>) -> Option<usize> {
    match expr {
        Expression::NumericLiteral(literal)
            if literal.value >= 0.0 && literal.value.fract() == 0.0 =>
        {
            Some(literal.value as usize)
        }
        _ => None,
    }
}

fn param_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn visit_body<'a>(usage: &mut ParamUsage<'a>, statements: &'a [Statement<'a>]) {
    for statement in statements {
        usage.visit_statement(statement);
    }
}

pub(super) fn used_params(function: &Function<'_>) -> Vec<bool> {
    let names: Vec<Option<&str>> = function
        .params
        .items
        .iter()
        .map(|param| param_name(&param.pattern))
        .collect();
    let mut usage = ParamUsage {
        used: vec![false; names.len()],
        names,
        shadowed: Vec::new(),
    };
    if let Some(body) = &function.body {
        visit_body(&mut usage, &body.statements);
    }
    usage.used
}

struct ArgumentsUse {
    found: bool,
}

impl<'a> VisitJs<'a> for ArgumentsUse {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name.as_str() == "arguments";
    }
}

fn reads_arguments(function: &Function<'_>) -> bool {
    let mut usage = ArgumentsUse { found: false };
    usage.visit_function(function, ScopeFlags::Function);
    usage.found
}

fn shifted_param<'a>(statement: &Statement<'a>) -> Option<&'a str> {
    let Statement::ExpressionStatement(node) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = node.expression.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let name = target.name.as_str();
    if assignment.operator == AssignmentOperator::Subtraction {
        let by_a_number = matches!(fold_constant(&assignment.right, &[]), Some(Prim::Num(_)));
        return by_a_number.then_some(name);
    }
    let Expression::BinaryExpression(shift) = assignment.right.without_parentheses() else {
        return None;
    };
    let reads_itself = matches!(shift.left.without_parentheses(),
        Expression::Identifier(id) if id.name.as_str() == name);
    let by_a_number = matches!(fold_constant(&shift.right, &[]), Some(Prim::Num(_)));
    let shaped = assignment.operator == AssignmentOperator::Assign
        && shift.operator == BinaryOperator::Subtraction
        && reads_itself
        && by_a_number;
    shaped.then_some(name)
}

pub(super) fn numeric_params(function: &Function<'_>) -> Vec<bool> {
    let params = &function.params;
    let mut numeric = vec![false; params.items.len()];
    let plain = params.rest.is_none()
        && params
            .items
            .iter()
            .all(|param| param.initializer.is_none() && param_name(&param.pattern).is_some());
    if !plain || reads_arguments(function) {
        return numeric;
    }
    let first = function
        .body
        .as_ref()
        .and_then(|body| body.statements.first());
    let Some(name) = first.and_then(shifted_param) else {
        return numeric;
    };
    let names: Vec<Option<&str>> = params
        .items
        .iter()
        .map(|param| param_name(&param.pattern))
        .collect();
    if let [index] = positions_of(&names, name)[..] {
        numeric[index] = true;
    }
    numeric
}

fn positions_of(names: &[Option<&str>], name: &str) -> Vec<usize> {
    names
        .iter()
        .enumerate()
        .filter(|(_, slot)| **slot == Some(name))
        .map(|(index, _)| index)
        .collect()
}
