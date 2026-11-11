use oxc_ast::ast::{
    Argument, AssignmentExpression, AssignmentTargetPropertyIdentifier,
    AssignmentTargetPropertyProperty, BinaryExpression, BindingProperty, CallExpression,
    Expression, IdentifierReference, MemberExpression, Program, PropertyKey, Statement,
    ThisExpression, UnaryExpression, WithStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};

const DYNAMIC_CODE: [&str; 2] = ["eval", "Function"];

const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];

const GLOBAL_OBJECTS: [&str; 7] = [
    "window",
    "globalThis",
    "self",
    "global",
    "frames",
    "top",
    "parent",
];

const REFLECTORS: [&str; 3] = ["Object", "Reflect", "constructor"];

const DESCRIPTOR_READS: [&str; 3] = [
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "deleteProperty",
];

const RESHAPES: [&str; 7] = [
    "defineProperty",
    "defineProperties",
    "freeze",
    "seal",
    "preventExtensions",
    "isFrozen",
    "isSealed",
];

pub(super) fn observes_globals(program: &Program<'_>) -> bool {
    let mut scan = Observer::default();
    scan.visit_program(program);
    scan.found
}

fn is_observer(name: &str) -> bool {
    DESCRIPTOR_READS.contains(&name) || RESHAPES.contains(&name)
}

fn reaches_reflection(name: &str) -> bool {
    REFLECTORS.contains(&name) || GLOBAL_OBJECTS.contains(&name)
}

fn runs_code(name: &str) -> bool {
    DYNAMIC_CODE.contains(&name) || TIMERS.contains(&name)
}

fn observing_key(key: &PropertyKey<'_>) -> bool {
    if matches!(key, PropertyKey::PrivateIdentifier(_)) {
        return false;
    }
    key.static_name().is_none_or(|name| {
        name.as_str()
            .is_some_and(|n| is_observer(n) || reaches_reflection(n))
    })
}

fn literal_key<'a>(member: &MemberExpression<'a>) -> Option<Option<&'a str>> {
    match member {
        MemberExpression::StaticMemberExpression(node) => Some(Some(node.property.name.as_str())),
        MemberExpression::ComputedMemberExpression(node) => {
            node.static_property_name().map(|key| key.as_str())
        }
        MemberExpression::PrivateFieldExpression(_) => Some(None),
    }
}

pub(super) fn deletes_a_global(argument: &Expression<'_>) -> bool {
    match argument.without_parentheses() {
        Expression::Identifier(_) | Expression::ChainExpression(_) => true,
        other => other.as_member_expression().is_some(),
    }
}

fn fresh_value(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
    )
}

fn is_function_literal(argument: Option<&Argument<'_>>) -> bool {
    argument
        .and_then(Argument::as_expression)
        .is_some_and(|first| {
            matches!(
                first.without_parentheses(),
                Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
            )
        })
}

#[derive(Default)]
struct Observer {
    found: bool,
}

impl Observer {
    fn visit_object<'a>(&mut self, object: &Expression<'a>) {
        if self.found {
            return;
        }
        match object.without_parentheses() {
            Expression::Identifier(id) => self.found |= runs_code(id.name.as_str()),
            Expression::ThisExpression(_) => {}
            other => match other.as_member_expression() {
                Some(member) => self.visit_object_member(member),
                None => self.visit_expression(other),
            },
        }
    }

    fn visit_object_member<'a>(&mut self, member: &MemberExpression<'a>) {
        match literal_key(member) {
            Some(Some(name)) if is_observer(name) => self.found = true,
            Some(_) => self.visit_object(member.object()),
            None => self.visit_member_expression(member),
        }
    }

    fn visit_member_value<'a>(&mut self, member: &MemberExpression<'a>) {
        match literal_key(member) {
            Some(Some(name)) if is_observer(name) || reaches_reflection(name) => {
                self.found = true;
            }
            Some(_) => self.visit_object(member.object()),
            None => {
                self.visit_expression(member.object());
                if let MemberExpression::ComputedMemberExpression(node) = member {
                    self.visit_expression(&node.expression);
                }
            }
        }
    }

    fn visit_compared(&mut self, operand: &Expression<'_>) {
        match operand.without_parentheses() {
            Expression::Identifier(id) if !runs_code(id.name.as_str()) => {}
            other => match other.as_member_expression() {
                Some(member) if matches!(literal_key(member), Some(Some(_))) => {
                    self.visit_object_member(member);
                }
                _ => self.visit_expression(other),
            },
        }
    }

    fn visit_callee_call<'a>(&mut self, it: &CallExpression<'a>) -> bool {
        let callee = it.callee.without_parentheses();
        if let Expression::Identifier(id) = callee {
            let name = id.name.as_str();
            if TIMERS.contains(&name) && is_function_literal(it.arguments.first()) {
                self.visit_arguments(&it.arguments);
                return true;
            }
            return false;
        }
        let Some(member) = callee.as_member_expression() else {
            return false;
        };
        let Some(Some(name)) = literal_key(member) else {
            return false;
        };
        if DESCRIPTOR_READS.contains(&name) {
            self.found = true;
            return true;
        }
        if !RESHAPES.contains(&name) {
            return false;
        }
        let first = it.arguments.first().and_then(Argument::as_expression);
        if first.is_none_or(|first| !fresh_value(first)) {
            self.found = true;
            return true;
        }
        self.visit_object(member.object());
        self.visit_arguments(&it.arguments);
        true
    }

    fn visit_arguments<'a>(&mut self, arguments: &[Argument<'a>]) {
        for argument in arguments {
            self.visit_argument(argument);
        }
    }
}

impl<'a> Visit<'a> for Observer {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.found {
            walk::walk_expression(self, it);
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.found {
            walk::walk_statement(self, it);
        }
    }

    fn visit_with_statement(&mut self, _it: &WithStatement<'a>) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.found |= runs_code(name) || reaches_reflection(name);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if !self.found {
            self.visit_member_value(it);
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete && deletes_a_global(&it.argument) {
            self.found = true;
            return;
        }
        if it.operator == UnaryOperator::Typeof {
            self.visit_compared(&it.argument);
            return;
        }
        walk::walk_unary_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        match it.operator {
            BinaryOperator::In => {
                self.visit_expression(&it.left);
                self.visit_compared(&it.right);
            }
            BinaryOperator::Instanceof => {
                self.visit_expression(&it.left);
                self.visit_compared(&it.right);
            }
            operator if operator.is_equality() => {
                self.visit_compared(&it.left);
                self.visit_compared(&it.right);
            }
            _ => walk::walk_binary_expression(self, it),
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.found || self.visit_callee_call(it) {
            return;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if self.found {
            return;
        }
        let written = it.left.as_member_expression().filter(|member| {
            it.operator == AssignmentOperator::Assign && literal_key(member).is_some()
        });
        match written {
            Some(member) => {
                self.visit_object(member.object());
                self.visit_expression(&it.right);
            }
            None => walk::walk_assignment_expression(self, it),
        }
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.found |= observing_key(&it.key);
        walk::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.found |= observing_key(&it.name);
        walk::walk_assignment_target_property_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let name = it.binding.name.as_str();
        self.found |= is_observer(name) || reaches_reflection(name);
        walk::walk_assignment_target_property_identifier(self, it);
    }
}
