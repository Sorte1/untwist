use std::collections::BTreeSet;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};

const STORAGE_KEY_LEN: usize = 5;
const MIN_CHAR_CODE_ARGS: usize = 4;

pub(super) fn ident_name<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

pub(super) fn member_name<'b, 'a>(
    expr: &'b Expression<'a>,
) -> Option<(&'b Expression<'a>, &'b str)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => {
            Some((&member.object, member.property.name.as_str()))
        }
        Expression::ComputedMemberExpression(member) => match &member.expression {
            Expression::StringLiteral(key) => Some((&member.object, key.value.as_str()?)),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn is_rotation(call: &CallExpression) -> bool {
    let Some((target, "push")) = member_name(&call.callee) else {
        return false;
    };
    let [Argument::CallExpression(inner)] = call.arguments.as_slice() else {
        return false;
    };
    let Some((source, "shift")) = member_name(&inner.callee) else {
        return false;
    };
    ident_name(target).is_some() && ident_name(target) == ident_name(source)
}

pub(super) fn is_numeric(expr: &Expression) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) && is_numeric(&unary.argument)
        }
        Expression::BinaryExpression(binary) => is_number_expression(binary),
        _ => false,
    }
}

pub(super) fn is_number_expression(binary: &BinaryExpression) -> bool {
    matches!(
        binary.operator,
        BinaryOperator::Addition | BinaryOperator::Subtraction | BinaryOperator::Multiplication
    ) && is_numeric(&binary.left)
        && is_numeric(&binary.right)
}

pub(super) fn is_string_concat(expr: &Expression) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_) => true,
        Expression::BinaryExpression(binary) => is_split_string(binary),
        _ => false,
    }
}

pub(super) fn is_split_string(binary: &BinaryExpression) -> bool {
    binary.operator == BinaryOperator::Addition
        && is_string_concat(&binary.left)
        && is_string_concat(&binary.right)
}

pub(super) fn is_identifier_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

pub(super) fn is_storage_key(text: &str) -> bool {
    text.len() == STORAGE_KEY_LEN
        && text.chars().all(|c| c.is_ascii_alphabetic())
        && text.chars().any(|c| c.is_ascii_uppercase())
        && text.chars().any(|c| c.is_ascii_lowercase())
}

pub(super) fn param_names<'b>(func: &'b Function) -> Option<Vec<&'b str>> {
    func.params
        .items
        .iter()
        .map(|p| match &p.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn is_param(expr: &Expression, params: &[&str]) -> bool {
    ident_name(expr).is_some_and(|n| params.contains(&n))
}

pub(super) fn is_proxy_function(expr: &Expression) -> bool {
    let Some(func) = function_of(expr) else {
        return false;
    };
    let Some(params) = param_names(func) else {
        return false;
    };
    if params.is_empty() {
        return false;
    }
    let Some(body) = &func.body else {
        return false;
    };
    let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return false;
    };
    match ret.argument.as_ref().map(|a| a.without_parentheses()) {
        Some(Expression::BinaryExpression(b)) => {
            is_param(&b.left, &params) && is_param(&b.right, &params)
        }
        Some(Expression::LogicalExpression(l)) => {
            is_param(&l.left, &params) && is_param(&l.right, &params)
        }
        Some(Expression::CallExpression(c)) => {
            is_param(&c.callee, &params)
                && c.arguments
                    .iter()
                    .all(|a| a.as_expression().is_some_and(|e| is_param(e, &params)))
        }
        _ => false,
    }
}

pub(super) fn property_key_text<'b>(key: &'b PropertyKey) -> Option<&'b str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(s) => s.value.as_str(),
        _ => None,
    }
}

pub(super) fn is_storage_object(object: &ObjectExpression) -> bool {
    let mut any = false;
    for prop in &object.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            return false;
        };
        if is_proxy_function(&prop.value) {
            return true;
        }
        let keyed = property_key_text(&prop.key).is_some_and(is_storage_key);
        let literal = matches!(
            prop.value.without_parentheses(),
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        );
        if keyed && literal {
            any = true;
        }
    }
    any
}

pub(super) fn printable_escapes(raw: &str) -> usize {
    let bytes = raw.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let hex = match bytes[i + 1] {
            b'x' => raw.get(i + 2..i + 4),
            b'u' if bytes.get(i + 2) == Some(&b'{') => raw[i + 3..].split('}').next(),
            b'u' => raw.get(i + 2..i + 6),
            _ => {
                i += 2;
                continue;
            }
        };
        let code = hex.and_then(|h| u32::from_str_radix(h, 16).ok());
        if code.is_some_and(|c| (0x20..0x7f).contains(&c)) {
            count += 1;
        }
        i += 2;
    }
    count
}

pub(super) fn is_hidden_string_replace(call: &CallExpression) -> bool {
    let Some((object, "replace")) = member_name(&call.callee) else {
        return false;
    };
    let second_empty = matches!(
        call.arguments.get(1).and_then(Argument::as_expression).map(|e| e.without_parentheses()),
        Some(Expression::StringLiteral(s)) if s.value.is_empty()
    );
    matches!(object.without_parentheses(), Expression::StringLiteral(_)) && second_empty
}

pub(super) fn has_char_code_array(call: &CallExpression) -> bool {
    call.arguments.iter().any(|arg| match arg {
        Argument::ArrayExpression(array) => {
            array.elements.len() >= MIN_CHAR_CODE_ARGS
                && array
                    .elements
                    .iter()
                    .all(|e| matches!(e, ArrayExpressionElement::NumericLiteral(_)))
        }
        _ => false,
    })
}

pub(super) struct ForInChecks {
    pub(super) found: bool,
}

impl<'a> Visit<'a> for ForInChecks {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if has_char_code_array(call) {
            self.found = true;
        }
        walk::walk_call_expression(self, call);
    }
}

#[derive(Default)]
pub(super) struct Controller {
    applied: BTreeSet<String>,
    nulled: BTreeSet<String>,
}

impl<'a> Visit<'a> for Controller {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some((object, "apply")) = member_name(&call.callee) {
            let forwards = call
                .arguments
                .get(1)
                .and_then(Argument::as_expression)
                .and_then(ident_name)
                == Some("arguments");
            if let (true, Some(name)) = (forwards, ident_name(object)) {
                self.applied.insert(name.to_string());
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_assignment_expression(&mut self, assign: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left {
            if matches!(
                assign.right.without_parentheses(),
                Expression::NullLiteral(_)
            ) {
                self.nulled.insert(id.name.to_string());
            }
        }
        walk::walk_assignment_expression(self, assign);
    }
}

pub(super) fn is_call_controller(func: &Function) -> bool {
    let Some(body) = &func.body else {
        return false;
    };
    let mut controller = Controller::default();
    for statement in &body.statements {
        controller.visit_statement(statement);
    }
    controller
        .applied
        .intersection(&controller.nulled)
        .next()
        .is_some()
}

pub(super) fn is_order_string(text: &str) -> bool {
    let parts: Vec<&str> = text.split('|').collect();
    parts.len() > 1
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

pub(super) fn is_dispatcher(switch: &SwitchStatement) -> bool {
    match switch.discriminant.without_parentheses() {
        Expression::ComputedMemberExpression(member) => {
            matches!(member.expression, Expression::UpdateExpression(_))
        }
        _ => false,
    }
}

pub(super) fn function_of<'b, 'a>(expr: &'b Expression<'a>) -> Option<&'b Function<'a>> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(func) => Some(&**func),
        _ => None,
    }
}
