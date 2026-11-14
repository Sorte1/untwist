use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use super::keys::{classify, Class};
use super::{Scan, This, GLOBAL_OBJECTS};
use crate::source_map::decode_base64;

pub(super) const REACHING_KEYS: [&str; 4] = [
    "__proto__",
    "constructor",
    "__lookupGetter__",
    "__lookupSetter__",
];

const FORWARDING_KEYS: [&str; 3] = ["call", "apply", "bind"];

const PROTOTYPE_READERS: [(&str, &str); 3] = [
    ("Object", "getPrototypeOf"),
    ("Reflect", "getPrototypeOf"),
    ("Reflect", "get"),
];

pub(super) const CODE_RUNNERS: [&str; 3] = ["eval", "Function", "execScript"];

const CONTEXT_RUNNERS: [&str; 4] = [
    "runInThisContext",
    "runInNewContext",
    "runInContext",
    "compileFunction",
];

const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];

#[derive(Clone, Copy)]
pub(super) enum Key<'k> {
    Name(&'k str),
    Other,
    Unknown(&'k Expression<'k>),
}

pub(super) fn key_of<'k>(expression: &'k Expression<'k>) -> Key<'k> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal
            .value
            .as_str()
            .map_or(Key::Unknown(expression), Key::Name),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked)
            .and_then(|cooked| cooked.as_str())
            .map_or(Key::Unknown(expression), Key::Name),
        Expression::NumericLiteral(_) | Expression::BigIntLiteral(_) => Key::Other,
        Expression::UpdateExpression(_) => Key::Other,
        Expression::UnaryExpression(unary) if is_numeric_unary(unary.operator) => Key::Other,
        Expression::BinaryExpression(binary) if is_numeric_binary(binary.operator) => Key::Other,
        _ => Key::Unknown(expression),
    }
}

fn is_numeric_unary(operator: UnaryOperator) -> bool {
    matches!(
        operator,
        UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
    )
}

fn is_numeric_binary(operator: BinaryOperator) -> bool {
    (operator.is_arithmetic() && operator != BinaryOperator::Addition) || operator.is_bitwise()
}

pub(super) fn member_of<'e, 'a>(
    expression: &'e Expression<'a>,
) -> Option<(&'e Expression<'a>, Key<'e>)> {
    match expression.without_parentheses() {
        Expression::StaticMemberExpression(member) => {
            Some((&member.object, Key::Name(member.property.name.as_str())))
        }
        Expression::ComputedMemberExpression(member) => {
            Some((&member.object, key_of(&member.expression)))
        }
        Expression::ChainExpression(chain) => match chain.expression.as_member_expression()? {
            MemberExpression::StaticMemberExpression(member) => {
                Some((&member.object, Key::Name(member.property.name.as_str())))
            }
            MemberExpression::ComputedMemberExpression(member) => {
                Some((&member.object, key_of(&member.expression)))
            }
            MemberExpression::PrivateFieldExpression(_) => None,
        },
        _ => None,
    }
}

pub(super) fn call_of<'e, 'a>(expression: &'e Expression<'a>) -> Option<&'e CallExpression<'a>> {
    match expression.without_parentheses() {
        Expression::CallExpression(call) => Some(call),
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => Some(call),
            _ => None,
        },
        _ => None,
    }
}

impl Scan<'_> {
    pub(super) fn reads_prototypes(&self, expression: &Expression<'_>) -> bool {
        let Some((object, Key::Name(name))) = member_of(expression) else {
            return false;
        };
        let Expression::Identifier(owner) = object.without_parentheses() else {
            return false;
        };
        PROTOTYPE_READERS
            .iter()
            .any(|(root, method)| *method == name && self.global_named(owner, &[root]))
    }

    pub(super) fn is_reached(&self, expression: &Expression<'_>) -> bool {
        if matches!(
            expression.without_parentheses(),
            Expression::ThisExpression(_)
        ) {
            return self.this_value() == This::MaybeGlobal;
        }
        if let Some((object, key)) = member_of(expression) {
            return self.reached_member(object, key);
        }
        call_of(expression).is_some_and(|call| {
            self.reads_prototypes(&call.callee) || self.is_reached(&call.callee)
        })
    }

    fn reached_member(&self, object: &Expression<'_>, key: Key<'_>) -> bool {
        match key {
            Key::Name(name) if REACHING_KEYS.contains(&name) => true,
            Key::Name(name) if self.names_a_root(name) => true,
            Key::Name(name) if GLOBAL_OBJECTS.contains(&name) => {
                self.is_reached(object) || self.is_root(object)
            }
            Key::Name("prototype") => self.is_reached(object),
            Key::Name(name) if FORWARDING_KEYS.contains(&name) => {
                self.is_reached(object) || self.reads_prototypes(object)
            }
            Key::Name(_) | Key::Other => false,
            Key::Unknown(key) => {
                self.is_reached(object)
                    || self.is_root(object)
                    || self.spells(key, |name| self.reached_member(object, Key::Name(name)))
            }
        }
    }

    fn folded_text(&self, expression: &Expression<'_>) -> Option<String> {
        match expression.without_parentheses() {
            Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
            Expression::BooleanLiteral(literal) => Some(literal.value.to_string()),
            Expression::NullLiteral(_) => Some("null".to_string()),
            Expression::NumericLiteral(literal)
                if literal.value.fract() == 0.0 && literal.value.abs() < 1e15 =>
            {
                Some(format!("{}", literal.value as i64))
            }
            Expression::ArrayExpression(array) if array.elements.is_empty() => Some(String::new()),
            Expression::TemplateLiteral(template) => {
                let mut text = String::new();
                for (index, quasi) in template.quasis.iter().enumerate() {
                    text.push_str(quasi.value.cooked.as_ref()?.as_str()?);
                    if let Some(inner) = template.expressions.get(index) {
                        text.push_str(&self.folded_text(inner)?);
                    }
                }
                Some(text)
            }
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                Some(self.folded_text(&binary.left)? + &self.folded_text(&binary.right)?)
            }
            Expression::CallExpression(call) => self.folded_call(call),
            _ => None,
        }
    }

    fn folded_call(&self, call: &CallExpression<'_>) -> Option<String> {
        let literals: Option<Vec<&Expression<'_>>> = call
            .arguments
            .iter()
            .map(|argument| argument.as_expression())
            .collect();
        let literals = literals?;
        match call.callee.without_parentheses() {
            Expression::Identifier(id) if self.global_named(id, &["atob"]) => {
                let [Expression::StringLiteral(text)] = literals.as_slice() else {
                    return None;
                };
                let bytes = decode_base64(text.value.as_str()?)?;
                Some(bytes.into_iter().map(char::from).collect())
            }
            Expression::StaticMemberExpression(member)
                if member.property.name == "fromCharCode"
                    && matches!(member.object.without_parentheses(),
                        Expression::Identifier(id) if self.global_named(id, &["String"])) =>
            {
                literals
                    .iter()
                    .map(|argument| match argument.without_parentheses() {
                        Expression::NumericLiteral(number)
                            if number.value.fract() == 0.0
                                && (0.0..65536.0).contains(&number.value) =>
                        {
                            char::from_u32(number.value as u32)
                        }
                        _ => None,
                    })
                    .collect()
            }
            _ => None,
        }
    }

    pub(super) fn spells(&self, key: &Expression<'_>, named: impl Fn(&str) -> bool) -> bool {
        if let Some(text) = self.folded_text(key) {
            return named(&text);
        }
        let mut deps = Vec::new();
        let class = classify(self.scoping, key, &mut deps);
        let spelled = class == Class::Spelling
            || deps.iter().any(|symbol| self.spelling_symbol(*symbol))
            || ((class == Class::Opaque || !deps.is_empty()) && self.names_a_reaching_key());
        if spelled {
            return true;
        }
        let plain = class == Class::Plain && deps.iter().all(|symbol| self.plain_symbol(*symbol));
        if plain {
            return false;
        }
        self.opaque_key.set(true);
        self.strict
    }

    pub(super) fn names_a_root(&self, name: &str) -> bool {
        self.reliance.roots.contains(&name)
    }

    pub(super) fn relies_on_key(&self, key: Option<&str>) -> bool {
        let Some(key) = key else {
            return true;
        };
        let unlisted = self.reliance.roots.iter().any(|root| {
            !self
                .reliance
                .members
                .iter()
                .any(|(owner, _, _)| owner == root)
        });
        unlisted
            || key == "__proto__"
            || self.names_a_root(key)
            || self
                .reliance
                .members
                .iter()
                .any(|(_, _, keys)| keys.contains(&key))
    }

    pub(super) fn is_code_runner_name(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, &CODE_RUNNERS),
            _ => false,
        }
    }

    pub(super) fn runs_code(&self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        match callee.without_parentheses() {
            Expression::SequenceExpression(sequence) => {
                return sequence
                    .expressions
                    .last()
                    .is_some_and(|last| self.runs_code(last, arguments));
            }
            Expression::LogicalExpression(logical) => {
                return self.runs_code(&logical.left, arguments)
                    || self.runs_code(&logical.right, arguments);
            }
            Expression::ConditionalExpression(conditional) => {
                return self.runs_code(&conditional.consequent, arguments)
                    || self.runs_code(&conditional.alternate, arguments);
            }
            _ => {}
        }
        if self.is_code_runner_name(callee) || self.is_reached(callee) {
            return true;
        }
        let named = |names: &[&str]| match callee.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, names),
            other => member_of(other).is_some_and(|(object, key)| {
                matches!(key, Key::Name(name) if names.contains(&name))
                    && self.is_global_object(object)
            }),
        };
        if named(&CODE_RUNNERS) {
            return true;
        }
        let context = member_of(callee).is_some_and(
            |(_, key)| matches!(key, Key::Name(name) if CONTEXT_RUNNERS.contains(&name)),
        );
        context || (named(&TIMERS) && !self.calls_back(arguments.first()))
    }

    fn calls_back(&self, first: Option<&Argument<'_>>) -> bool {
        let Some(expression) = first.and_then(Argument::as_expression) else {
            return false;
        };
        match expression.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(id) => id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .is_some_and(|symbol| self.scoping.symbol_flags(symbol).is_function()),
            other => call_of(other).is_some_and(|call| {
                matches!(member_of(&call.callee), Some((_, Key::Name("bind"))))
            }),
        }
    }
}
