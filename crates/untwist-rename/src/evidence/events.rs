use oxc_ast::ast::*;

use super::values::{global_callee, method_name};

const MAX_EVENT_TYPE: usize = 24;

const KEY_EVENTS: [&str; 3] = ["keydown", "keyup", "keypress"];

const MOUSE_EVENTS: [&str; 9] = [
    "click",
    "dblclick",
    "mousedown",
    "mouseup",
    "mousemove",
    "mouseover",
    "mouseout",
    "mouseenter",
    "mouseleave",
];

const TIMER_CALLBACKS: [(&str, &str); 4] = [
    ("setTimeout", "onTimeout"),
    ("setInterval", "onInterval"),
    ("requestAnimationFrame", "onFrame"),
    ("queueMicrotask", "onMicrotask"),
];

const BODY_READERS: [(&str, &str); 5] = [
    ("json", "data"),
    ("text", "text"),
    ("arrayBuffer", "buffer"),
    ("blob", "blob"),
    ("formData", "formData"),
];

pub fn is_event_type(kind: &str) -> bool {
    !kind.is_empty()
        && kind.len() <= MAX_EVENT_TYPE
        && kind.chars().all(|ch| ch.is_ascii_lowercase())
}

pub fn event_param_name(kind: Option<&str>) -> &'static str {
    let Some(kind) = kind else {
        return "event";
    };
    if KEY_EVENTS.contains(&kind) {
        return "keyEvent";
    }
    if MOUSE_EVENTS.contains(&kind) || kind == "contextmenu" {
        return "mouseEvent";
    }
    if kind == "wheel" {
        return "wheelEvent";
    }
    if kind.starts_with("touch") {
        return "touchEvent";
    }
    if kind.starts_with("pointer") {
        return "pointerEvent";
    }
    "event"
}

pub fn handler_name(kind: &str) -> Option<String> {
    if !is_event_type(kind) {
        return None;
    }
    let mut chars = kind.chars();
    let first = chars.next()?;
    Some(format!(
        "on{}{}",
        first.to_ascii_uppercase(),
        chars.as_str()
    ))
}

pub fn on_property_event(property: &str) -> Option<&str> {
    property
        .strip_prefix("on")
        .filter(|kind| is_event_type(kind))
}

pub fn timer_callback_name(call: &CallExpression<'_>) -> Option<&'static str> {
    let name = match global_callee(&call.callee) {
        Some(name) => std::borrow::Cow::Borrowed(name),
        None => method_name(&call.callee)?,
    };
    TIMER_CALLBACKS
        .iter()
        .find(|(timer, _)| *timer == name)
        .map(|(_, callback)| *callback)
}

fn returned_expression<'k, 'a>(argument: &'k Argument<'a>) -> Option<&'k Expression<'a>> {
    match argument {
        Argument::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => final_return(&body.statements),
            body => body.as_expression(),
        },
        Argument::FunctionExpression(function) => final_return(&function.body.as_ref()?.statements),
        _ => None,
    }
}

fn final_return<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<&'k Expression<'a>> {
    match statements.last()? {
        Statement::ReturnStatement(statement) => statement.argument.as_ref(),
        _ => None,
    }
}

fn body_reader_name(expr: &Expression<'_>) -> Option<&'static str> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let method = method_name(&call.callee)?;
    BODY_READERS
        .iter()
        .find(|(reader, _)| *reader == method)
        .map(|(_, name)| *name)
}

pub fn settled_value_name(receiver: &Expression<'_>) -> Option<&'static str> {
    let Expression::CallExpression(call) = receiver.without_parentheses() else {
        return None;
    };
    if global_callee(&call.callee) == Some("fetch") {
        return Some("response");
    }
    if let Some(name) = body_reader_name(receiver) {
        return Some(name);
    }
    if method_name(&call.callee).is_some_and(|method| method == "then") {
        return call
            .arguments
            .first()
            .and_then(returned_expression)
            .and_then(body_reader_name);
    }
    None
}
