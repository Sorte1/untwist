use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, AssignmentTarget, CallExpression, Expression, NewExpression, Program,
    SimpleAssignmentTarget, UnaryExpression, UpdateExpression,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::operator::UnaryOperator;
use std::collections::HashSet;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;

const GLOBAL_OBJECTS: [&str; 7] = [
    "globalThis",
    "window",
    "self",
    "global",
    "top",
    "parent",
    "frames",
];
const TEXT_RUNNERS: [&str; 5] = [
    "eval",
    "Function",
    "execScript",
    "setTimeout",
    "setInterval",
];
const DEFINERS: [&str; 4] = [
    "defineProperty",
    "defineProperties",
    "assign",
    "__defineGetter__",
];
const TEXT_DEPTH_CAP: u32 = 4;

#[derive(Default)]
pub(super) struct GlobalWrites {
    pub names: HashSet<String>,
    pub every: bool,
}

struct Walk<'w> {
    out: &'w mut GlobalWrites,
    script: bool,
    text_depth: u32,
}

pub(super) fn global_writes(program: &Program<'_>) -> GlobalWrites {
    let mut out = GlobalWrites::default();
    let mut walk = Walk {
        out: &mut out,
        script: !program.source_type.is_module(),
        text_depth: 0,
    };
    walk.visit_program(program);
    out
}

fn static_key(expression: &Expression<'_>) -> Option<String> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Expression::NumericLiteral(number) => Some(number.value.to_string()),
        _ => None,
    }
}

fn names_runner(callee: &Expression<'_>) -> bool {
    match callee.without_parentheses() {
        Expression::Identifier(id) => TEXT_RUNNERS.contains(&id.name.as_str()),
        Expression::SequenceExpression(sequence) => {
            sequence.expressions.last().is_some_and(names_runner)
        }
        other => other
            .as_member_expression()
            .and_then(|member| member.static_property_name())
            .and_then(|name| name.as_str())
            .is_some_and(|name| TEXT_RUNNERS.contains(&name)),
    }
}

impl Walk<'_> {
    fn is_global_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => GLOBAL_OBJECTS.contains(&id.name.as_str()),
            Expression::ThisExpression(_) => self.script,
            _ => false,
        }
    }

    fn member_written(&mut self, object: &Expression<'_>, key: Option<String>) {
        if !self.is_global_object(object) {
            return;
        }
        match key {
            Some(name) => {
                self.out.names.insert(name);
            }
            None => self.out.every = true,
        }
    }

    fn target_written(&mut self, target: &SimpleAssignmentTarget<'_>) {
        match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.member_written(&member.object, Some(member.property.name.to_string()));
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.member_written(&member.object, static_key(&member.expression));
            }
            _ => {}
        }
    }

    fn runs_text(&mut self, arguments: &[Argument<'_>]) {
        for argument in arguments {
            let Some(expression) = argument.as_expression() else {
                self.out.every = true;
                continue;
            };
            match expression.without_parentheses() {
                Expression::StringLiteral(literal) => match literal.value.as_str() {
                    Some(text) => self.read_text(text),
                    None => self.out.every = true,
                },
                Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {}
                Expression::NumericLiteral(_) => {}
                _ => self.out.every = true,
            }
        }
    }

    fn read_text(&mut self, text: &str) {
        if self.text_depth >= TEXT_DEPTH_CAP {
            self.out.every = true;
            return;
        }
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, text);
        if has_error(&parsed.diagnostics) {
            return;
        }
        let scoping = build_scoping(&parsed.program).scoping;
        self.out.names.extend(
            scoping.symbol_names().map(str::to_string).chain(
                scoping
                    .root_unresolved_references()
                    .keys()
                    .map(|name| name.to_string()),
            ),
        );
        let mut inner = Walk {
            out: &mut *self.out,
            script: true,
            text_depth: self.text_depth + 1,
        };
        inner.visit_program(&parsed.program);
    }

    fn defines(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) {
        let Some(member) = callee.without_parentheses().as_member_expression() else {
            return;
        };
        let definer = member
            .static_property_name()
            .and_then(|name| name.as_str())
            .is_some_and(|name| DEFINERS.contains(&name));
        let target_is_global = arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|target| self.is_global_object(target))
            || self.is_global_object(member.object());
        if definer && target_is_global {
            self.out.every = true;
        }
    }
}

impl<'a> Visit<'a> for Walk<'_> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        if let Some(target) = it.as_simple_assignment_target() {
            self.target_written(target);
        }
        walk::walk_assignment_target(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        self.target_written(&it.argument);
        walk::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                let key = member
                    .static_property_name()
                    .and_then(|name| name.as_str())
                    .map(str::to_string);
                self.member_written(member.object(), key);
            }
        }
        walk::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if names_runner(&it.callee) {
            self.runs_text(&it.arguments);
        }
        self.defines(&it.callee, &it.arguments);
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if names_runner(&it.callee) {
            self.runs_text(&it.arguments);
        }
        walk::walk_new_expression(self, it);
    }
}
