use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, BindingIdentifier, BindingPattern, CallExpression,
    Expression, Function, IdentifierReference, MemberExpression, SimpleAssignmentTarget, Statement,
    UnaryExpression, UpdateExpression, VariableDeclarator, WithStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use super::names::{FORBIDDEN_NAMES, INTRINSICS};
use crate::stack;

const DEFINERS: [&str; 9] = [
    "defineProperty",
    "defineProperties",
    "setPrototypeOf",
    "getPrototypeOf",
    "assign",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "__defineGetter__",
    "__defineSetter__",
];

const GLOBAL_HANDLES: [&str; 6] = ["Reflect", "Proxy", "globalThis", "window", "self", "global"];

#[derive(Default)]
struct Declarations<'a> {
    names: FxHashSet<&'a str>,
    functions: FxHashSet<&'a str>,
    assigned: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Declarations<'a> {
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

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.as_str());
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (true, Some(id)) = (it.is_declaration(), &it.id) {
            self.functions.insert(id.name.as_str());
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(Expression::FunctionExpression(_))) =
            (&it.id, &it.init)
        {
            self.functions.insert(id.name.as_str());
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.assigned.insert(id.name.as_str());
        }
        walk_js::walk_simple_assignment_target(self, it);
    }
}

struct Guard<'s, 'a> {
    declared: &'s Declarations<'a>,
    machinery: &'s FxHashSet<&'a str>,
    failed: bool,
}

fn chain_root<'n, 'a>(member: &'n MemberExpression<'a>) -> (&'n Expression<'a>, Option<&'n str>) {
    let mut first = member;
    while let Some(inner) = first.object().without_parentheses().as_member_expression() {
        first = inner;
    }
    (
        first.object().without_parentheses(),
        first.static_property_name(),
    )
}

impl<'a> Guard<'_, 'a> {
    fn function_bound(&self, expr: &Expression<'a>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if self.declared.functions.contains(id.name.as_str())
                && !self.declared.assigned.contains(id.name.as_str()))
    }

    fn declared(&self, name: &str) -> bool {
        self.declared.names.contains(name) || self.machinery.contains(name)
    }

    fn member_write(&mut self, member: &MemberExpression<'a>) {
        let (root, first) = chain_root(member);
        let rooted = match root {
            Expression::ThisExpression(_) => true,
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                self.declared(name) && !INTRINSICS.contains(&name)
            }
            _ => false,
        };
        let shadows =
            first.is_some_and(|key| INTRINSICS.contains(&key) || self.machinery.contains(key));
        if !rooted || shadows {
            self.failed = true;
        }
    }

    fn target(&mut self, target: &SimpleAssignmentTarget<'a>) {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                if !self.declared(id.name.as_str()) {
                    self.failed = true;
                }
            }
            other => match other.as_member_expression() {
                Some(member) => self.member_write(member),
                None => self.failed = true,
            },
        }
    }
}

impl<'a> VisitJs<'a> for Guard<'_, 'a> {
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

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        match it.static_property_name() {
            Some("__proto__" | "constructor") => self.failed = true,
            Some("prototype") if !self.function_bound(it.object()) => self.failed = true,
            _ => {}
        }
        walk_js::walk_member_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        match it.left.as_simple_assignment_target() {
            Some(target) => self.target(target),
            None => self.failed = true,
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        self.target(&it.argument);
        walk_js::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            match it.argument.without_parentheses().as_member_expression() {
                Some(member) => self.member_write(member),
                None => self.failed = true,
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let definer = it
            .callee
            .as_member_expression()
            .and_then(MemberExpression::static_property_name)
            .is_some_and(|key| DEFINERS.contains(&key));
        if definer {
            self.failed = true;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        let handle = GLOBAL_HANDLES.contains(&name) || FORBIDDEN_NAMES[1..].contains(&name);
        if handle && !self.declared(name) {
            self.failed = true;
        }
    }

    fn visit_with_statement(&mut self, _it: &WithStatement<'a>) {
        self.failed = true;
    }
}

pub(super) fn clean<'a>(functions: &[&Function<'a>], machinery: &FxHashSet<&'a str>) -> bool {
    let mut declared = Declarations::default();
    for function in functions {
        declared.visit_function(function, ScopeFlags::Function);
    }
    if INTRINSICS.iter().any(|name| declared.names.contains(name)) {
        return false;
    }
    let mut guard = Guard {
        declared: &declared,
        machinery,
        failed: false,
    };
    for function in functions {
        guard.visit_function(function, ScopeFlags::Function);
    }
    !guard.failed
}
