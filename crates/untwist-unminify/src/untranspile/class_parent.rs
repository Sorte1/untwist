use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::LogicalOperator;

#[derive(Default)]
struct ParentCalls {
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

impl<'a> VisitJs<'a> for ParentCalls {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(member) = it.callee.without_parentheses() {
            let property = member.property.name.as_str();
            let nested = matches!(member.object.without_parentheses(),
                Expression::StaticMemberExpression(inner) if forwarding(inner.property.name.as_str()));
            self.found |= forwarding(property)
                && (this_first(it) || nested)
                && computed_parent(&member.object);
        }
        if !self.found {
            walk_js::walk_call_expression(self, it);
        }
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
            self.found |= forwarded(left).is_some_and(this_first);
        }
        if !self.found {
            walk_js::walk_logical_expression(self, it);
        }
    }
}

pub fn calls_parent_as_function(program: &Program<'_>) -> bool {
    let mut calls = ParentCalls::default();
    calls.visit_program(program);
    calls.found
}
