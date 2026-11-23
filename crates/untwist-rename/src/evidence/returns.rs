use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::prop::member_parts;

use super::functions::title;
use super::values::{assigned_value_name, key_name, method_name};
use crate::collect::reference_symbol;
use crate::generate::strip_accessor_prefix;

const INVOKERS: [&str; 2] = ["call", "apply"];

pub const DESCRIPTOR: &str = "getOwnPropertyDescriptor";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Returned {
    Named(String),
    Forward(SymbolId),
    Local(SymbolId),
}

pub fn returned_value(returns: &[&Expression<'_>], scoping: &Scoping) -> Option<Returned> {
    let mut found: Option<Returned> = None;
    for expr in returns {
        if is_empty_value(expr) {
            continue;
        }
        let next = return_of(expr, scoping)?;
        match &found {
            Some(held) if *held != next => return None,
            _ => found = Some(next),
        }
    }
    found
}

fn is_empty_value(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NullLiteral(_) => true,
        Expression::Identifier(id) => id.name == "undefined",
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::Void,
        _ => false,
    }
}

fn prefixed(prefix: &str, value: &str) -> Returned {
    Returned::Named(format!("{prefix}{}", title(value)))
}

fn return_of(expr: &Expression<'_>, scoping: &Scoping) -> Option<Returned> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => reference_symbol(id, scoping).map(Returned::Local),
        Expression::NewExpression(_) => Some(prefixed("create", &assigned_value_name(expr)?)),
        Expression::StaticMemberExpression(_) => Some(prefixed("get", &assigned_value_name(expr)?)),
        Expression::CallExpression(call) => call_return(call, expr, scoping),
        _ => None,
    }
}

fn invoked(callee: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let target = match member_parts(callee) {
        Some((object, method)) if INVOKERS.contains(&method.as_ref()) => object,
        Some(_) => return None,
        None => callee,
    };
    match target.without_parentheses() {
        Expression::Identifier(id) => reference_symbol(id, scoping),
        _ => None,
    }
}

fn call_return(
    call: &CallExpression<'_>,
    expr: &Expression<'_>,
    scoping: &Scoping,
) -> Option<Returned> {
    let callee = call.callee.without_parentheses();
    if let Some(symbol) = invoked(callee, scoping) {
        return Some(Returned::Forward(symbol));
    }
    let value = assigned_value_name(expr)?;
    let method = method_name(callee);
    match method {
        Some(method) if strip_accessor_prefix(&method).as_deref() == Some(value.as_str()) => {
            Some(Returned::Named(method.into_owned()))
        }
        Some(method) if method == value => None,
        _ => Some(prefixed("get", &value)),
    }
}

pub struct Getter {
    pub key: String,
    pub through: Option<SymbolId>,
}

pub fn descriptor_getter(value: &Expression<'_>, scoping: &Scoping) -> Option<Getter> {
    let Expression::StaticMemberExpression(member) = value.without_parentheses() else {
        return None;
    };
    let Expression::CallExpression(call) = member.object.without_parentheses() else {
        return None;
    };
    let through = match call.callee.without_parentheses() {
        Expression::Identifier(id) if id.name == DESCRIPTOR => None,
        Expression::Identifier(id) => Some(reference_symbol(id, scoping)?),
        callee => (method_name(callee)? == DESCRIPTOR).then_some(None)?,
    };
    let Some(Argument::StringLiteral(key)) = call.arguments.get(1) else {
        return None;
    };
    let key = key_name(key.value.as_str()?)?;
    (member.property.name == "get").then_some(Getter { key, through })
}
