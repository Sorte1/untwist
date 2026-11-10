use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use untwist_core::ast_util::js_text::js_text;

const WRITER_KEYS: &[&str] = &[
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "setPrototypeOf",
    "defineProperty",
    "defineProperties",
    "deleteProperty",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "assign",
    "set",
];

fn writer_key(key: &Expression<'_>) -> bool {
    match key.without_parentheses() {
        Expression::StringLiteral(name) => WRITER_KEYS.iter().any(|key| name.value == *key),
        Expression::NumericLiteral(_) => false,
        _ => true,
    }
}

#[derive(Default)]
struct TextWrites {
    found: bool,
}

impl<'a> VisitJs<'a> for TextWrites {
    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.found |= it.as_member_expression().is_some()
            || matches!(it, SimpleAssignmentTarget::AssignmentTargetIdentifier(_));
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.found = true;
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        self.found |= it.operator == UnaryOperator::Delete;
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.found |= match it {
            MemberExpression::StaticMemberExpression(member) => {
                WRITER_KEYS.contains(&member.property.name.as_str())
            }
            MemberExpression::ComputedMemberExpression(member) => writer_key(&member.expression),
            MemberExpression::PrivateFieldExpression(_) => false,
        };
        walk_js::walk_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "Reflect";
    }
}

const TEXT_RUNNERS: &[&str] = &[
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "execScript",
    "require",
    "runInThisContext",
    "runInContext",
    "runInNewContext",
    "compileFunction",
    "Script",
    "SourceTextModule",
    "constructor",
    "write",
    "writeln",
    "insertAdjacentHTML",
    "createContextualFragment",
];

fn reach_key(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::StaticIdentifier(name) => {
            WRITER_KEYS.contains(&name.name.as_str()) || TEXT_RUNNERS.contains(&name.name.as_str())
        }
        PropertyKey::PrivateIdentifier(_) => false,
        other => other
            .as_expression()
            .is_none_or(|expr| match expr.without_parentheses() {
                Expression::StringLiteral(name) => {
                    WRITER_KEYS.iter().any(|key| name.value == *key)
                        || TEXT_RUNNERS.iter().any(|key| name.value == *key)
                }
                Expression::NumericLiteral(_) => false,
                _ => true,
            }),
    }
}

fn runner_member(it: &MemberExpression<'_>) -> bool {
    match it {
        MemberExpression::StaticMemberExpression(member) => {
            TEXT_RUNNERS.contains(&member.property.name.as_str())
        }
        MemberExpression::ComputedMemberExpression(member) => {
            match member.expression.without_parentheses() {
                Expression::StringLiteral(name) => {
                    TEXT_RUNNERS.iter().any(|key| name.value == *key)
                }
                _ => false,
            }
        }
        MemberExpression::PrivateFieldExpression(_) => false,
    }
}

fn runner_callee(callee: &Expression<'_>) -> bool {
    match callee.without_parentheses() {
        Expression::Identifier(id) => TEXT_RUNNERS.contains(&id.name.as_str()),
        other => other.as_member_expression().is_some_and(runner_member),
    }
}

#[derive(Default)]
struct TextRuns {
    found: bool,
    callees: Vec<u32>,
}

impl TextRuns {
    fn runner_call(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) {
        if !runner_callee(callee) {
            return;
        }
        self.callees.push(callee.without_parentheses().span().start);
        for argument in arguments {
            self.found |= match argument
                .as_expression()
                .map(Expression::without_parentheses)
            {
                Some(Expression::StringLiteral(text)) => {
                    super::escape_scan::literal_text_reaches_in(&js_text(text.value))
                }
                Some(Expression::TemplateLiteral(t)) if t.expressions.is_empty() => t
                    .quasis
                    .first()
                    .and_then(|quasi| quasi.value.cooked)
                    .is_none_or(|text| super::escape_scan::literal_text_reaches_in(&js_text(text))),
                Some(Expression::NumericLiteral(_) | Expression::BooleanLiteral(_)) => false,
                _ => true,
            };
        }
    }

    fn escaped(&self, span: Span) -> bool {
        !self.callees.contains(&span.start)
    }
}

impl<'a> VisitJs<'a> for TextRuns {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.runner_call(&it.callee, &it.arguments);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.runner_call(&it.callee, &it.arguments);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= TEXT_RUNNERS.contains(&it.name.as_str()) && self.escaped(it.span);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.found |= self.escaped(it.span())
            && match it {
                MemberExpression::StaticMemberExpression(member) => {
                    TEXT_RUNNERS.contains(&member.property.name.as_str())
                }
                MemberExpression::ComputedMemberExpression(member) => {
                    match member.expression.without_parentheses() {
                        Expression::StringLiteral(name) => {
                            TEXT_RUNNERS.iter().any(|key| name.value == *key)
                        }
                        _ => false,
                    }
                }
                MemberExpression::PrivateFieldExpression(_) => false,
            };
        walk_js::walk_member_expression(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.found |= reach_key(&it.key);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let name = it.binding.name.as_str();
        self.found |= WRITER_KEYS.contains(&name) || TEXT_RUNNERS.contains(&name);
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.found |= reach_key(&it.name);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.found = true;
        walk_js::walk_import_expression(self, it);
    }
}

#[derive(Default)]
struct TextBinds {
    found: bool,
}

impl<'a> VisitJs<'a> for TextBinds {
    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.found |= matches!(it, SimpleAssignmentTarget::AssignmentTargetIdentifier(_));
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.found = true;
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        self.found |= it.kind == VariableDeclarationKind::Var;
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        self.found |= it.is_declaration();
        walk_js::walk_function(self, it, flags);
    }
}

pub(crate) fn binds_in_text(program: &Program<'_>) -> bool {
    let mut scan = TextBinds::default();
    scan.visit_program(program);
    scan.found
}

pub(crate) fn runs_in_text(program: &Program<'_>) -> bool {
    let mut scan = TextRuns::default();
    scan.visit_program(program);
    scan.found
}

pub(crate) fn writes_in_text(program: &Program<'_>) -> bool {
    let mut scan = TextWrites::default();
    scan.visit_program(program);
    scan.found
}
