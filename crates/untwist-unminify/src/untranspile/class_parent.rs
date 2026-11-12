use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::symbol::SymbolId;

struct ParentCalls<'s> {
    scoping: &'s Scoping,
    builtin_parents: Vec<SymbolId>,
    found: bool,
}

fn forwarding(property: &str) -> bool {
    property == "call" || property == "apply"
}

fn prototype_lookup(call: &CallExpression<'_>) -> bool {
    matches!(
        call.arguments.as_slice(),
        [Argument::Identifier(_) | Argument::StaticMemberExpression(_)]
    )
}

fn proto_chain(logical: &LogicalExpression<'_>) -> bool {
    logical.operator == LogicalOperator::Or
        && matches!(logical.left.without_parentheses(),
            Expression::StaticMemberExpression(member) if member.property.name.as_str() == "__proto__")
}

fn computed_parent(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::CallExpression(call) => prototype_lookup(call),
        Expression::LogicalExpression(logical) => proto_chain(logical),
        Expression::AssignmentExpression(assignment) => computed_parent(&assignment.right),
        Expression::StaticMemberExpression(member) => {
            forwarding(member.property.name.as_str()) && computed_parent(&member.object)
        }
        _ => false,
    }
}

fn this_first(call: &CallExpression<'_>) -> bool {
    matches!(
        call.arguments.first().and_then(Argument::as_expression),
        Some(Expression::ThisExpression(_))
    )
}

fn forwarded<'k, 'a>(expression: &'k Expression<'a>) -> Option<&'k CallExpression<'a>> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    forwarding(member.property.name.as_str()).then_some(call)
}

fn parent_forwarding(call: &CallExpression<'_>) -> bool {
    if call.optional {
        return false;
    }
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return false;
    };
    let nested = matches!(member.object.without_parentheses(),
        Expression::StaticMemberExpression(inner) if forwarding(inner.property.name.as_str()));
    !member.optional
        && forwarding(member.property.name.as_str())
        && (this_first(call) || nested)
        && computed_parent(&member.object)
}

fn wrapped_parent_call(call: &CallExpression<'_>) -> bool {
    let [first, second, ..] = call.arguments.as_slice() else {
        return false;
    };
    matches!(first, Argument::ThisExpression(_))
        && second
            .as_expression()
            .and_then(forwarded)
            .is_some_and(parent_forwarding)
}

fn forwarded_object<'k, 'a>(call: &'k CallExpression<'a>) -> Option<&'k Expression<'a>> {
    match call.callee.without_parentheses() {
        Expression::StaticMemberExpression(member) => Some(&member.object),
        _ => None,
    }
}

impl ParentCalls<'_> {
    fn global_argument(&self, argument: &Argument<'_>) -> bool {
        let Argument::Identifier(id) = argument else {
            return false;
        };
        id.reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).symbol_id().is_none())
    }

    fn builtin_parent(&self, expression: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expression.without_parentheses() else {
            return false;
        };
        id.reference_id.get().is_some_and(|reference| {
            self.scoping
                .get_reference(reference)
                .symbol_id()
                .is_some_and(|symbol| self.builtin_parents.contains(&symbol))
        })
    }

    fn global_parameters(&self, call: &CallExpression<'_>) -> Vec<SymbolId> {
        let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
            return Vec::new();
        };
        function
            .params
            .items
            .iter()
            .zip(call.arguments.iter())
            .filter(|(_, argument)| self.global_argument(argument))
            .filter_map(|(param, _)| match &param.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            })
            .collect()
    }
}

impl<'a> VisitJs<'a> for ParentCalls<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.found |= wrapped_parent_call(it);
        if self.found {
            return;
        }
        let parameters = self.global_parameters(it);
        let pushed = parameters.len();
        self.builtin_parents.extend(parameters);
        walk_js::walk_call_expression(self, it);
        let kept = self.builtin_parents.len() - pushed;
        self.builtin_parents.truncate(kept);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        let returns_this = it.operator == LogicalOperator::Or
            && matches!(
                it.right.without_parentheses(),
                Expression::ThisExpression(_)
            );
        if returns_this {
            let left = match it.left.without_parentheses() {
                Expression::LogicalExpression(inner) if inner.operator == LogicalOperator::And => {
                    &inner.right
                }
                other => other,
            };
            self.found |= forwarded(left).is_some_and(|call| {
                this_first(call)
                    && !forwarded_object(call).is_some_and(|object| self.builtin_parent(object))
            });
        }
        if !self.found {
            walk_js::walk_logical_expression(self, it);
        }
    }
}

pub fn calls_parent_as_function(program: &Program<'_>, scoping: &Scoping) -> bool {
    let mut calls = ParentCalls {
        scoping,
        builtin_parents: Vec::new(),
        found: false,
    };
    calls.visit_program(program);
    calls.found
}
