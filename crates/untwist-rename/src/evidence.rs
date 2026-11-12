use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::prop::member_parts;

use crate::collect::reference_symbol;
use crate::generate::lower_camel;
use crate::human::looks_human_written;

const ELEMENT_CALLBACKS: [&str; 10] = [
    "forEach",
    "map",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "some",
    "every",
    "flatMap",
];

const ELEMENT_PARAMS: [&str; 3] = ["item", "index", "array"];

const REDUCERS: [&str; 2] = ["reduce", "reduceRight"];

const REDUCER_PARAMS: [&str; 4] = ["acc", "item", "index", "array"];

const SORTERS: [&str; 2] = ["sort", "toSorted"];

const SORTER_PARAMS: [&str; 2] = ["a", "b"];

const EXECUTOR_PARAMS: [&str; 2] = ["resolve", "reject"];

const EVENT_SUBSCRIBERS: [&str; 3] = ["addEventListener", "on", "once"];

const NAMED_RESULTS: [(&str, &str); 18] = [
    ("map", "mapped"),
    ("filter", "filtered"),
    ("sort", "sorted"),
    ("reduce", "reduced"),
    ("join", "joined"),
    ("find", "found"),
    ("getContext", "ctx"),
    ("split", "parts"),
    ("keys", "keys"),
    ("values", "values"),
    ("entries", "entries"),
    ("getBoundingClientRect", "rect"),
    ("getAttribute", "attr"),
    ("querySelectorAll", "elements"),
    ("getElementsByClassName", "elements"),
    ("getElementsByTagName", "elements"),
    ("now", "now"),
    ("stringify", "json"),
];

const NAMED_GLOBAL_CALLS: [(&str, &str); 3] = [
    ("fetch", "response"),
    ("setTimeout", "timer"),
    ("setInterval", "timer"),
];

const MAX_TAG_LENGTH: usize = 24;

const MIN_SEGMENT: usize = 2;

const NAMING_HOLDERS: [&str; 5] = ["exports", "window", "self", "globalThis", "global"];

const IGNORED_PROPERTIES: [&str; 4] = ["prototype", "exports", "default", "__proto__"];

#[derive(Default)]
pub struct Evidence {
    names: FxHashMap<SymbolId, String>,
}

impl Evidence {
    pub fn name(&self, symbol: SymbolId) -> Option<&str> {
        self.names.get(&symbol).map(String::as_str)
    }

    fn offer(&mut self, symbol: SymbolId, name: impl Into<String>) {
        self.names.entry(symbol).or_insert_with(|| name.into());
    }
}

pub fn collect_evidence(program: &Program<'_>, scoping: &Scoping) -> Evidence {
    let mut walker = EvidenceWalker {
        scoping,
        out: Evidence::default(),
    };
    walker.visit_program(program);
    walker.out
}

struct EvidenceWalker<'s> {
    scoping: &'s Scoping,
    out: Evidence,
}

fn simple_symbol(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn callback_params<'k, 'a>(argument: &'k Argument<'a>) -> Option<&'k FormalParameters<'a>> {
    match argument {
        Argument::FunctionExpression(function) => Some(&function.params),
        Argument::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        _ => None,
    }
}

fn method_name<'k>(callee: &'k Expression<'_>) -> Option<std::borrow::Cow<'k, str>> {
    member_parts(callee.without_parentheses()).map(|(_, name)| name)
}

fn global_callee<'k>(callee: &'k Expression<'_>) -> Option<&'k str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn string_argument<'k>(arguments: &'k [Argument<'_>]) -> Option<&'k str> {
    match arguments.first()? {
        Argument::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn tag_name(tag: &str) -> Option<String> {
    let mut chars = tag.chars();
    let first = chars.next()?;
    let plain = first.is_ascii_lowercase()
        && tag.len() <= MAX_TAG_LENGTH
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit());
    plain.then(|| tag.to_string())
}

fn call_result_name(call: &CallExpression<'_>) -> Option<String> {
    if let Some(name) = global_callee(&call.callee) {
        return NAMED_GLOBAL_CALLS
            .iter()
            .find(|(callee, _)| *callee == name)
            .map(|(_, result)| result.to_string());
    }
    let method = method_name(&call.callee)?;
    if method == "createElement" {
        return string_argument(&call.arguments).and_then(tag_name);
    }
    if method == "parse" && is_json_member(&call.callee) {
        return Some("data".to_string());
    }
    NAMED_RESULTS
        .iter()
        .find(|(name, _)| *name == method)
        .map(|(_, result)| result.to_string())
}

fn is_json_member(callee: &Expression<'_>) -> bool {
    member_parts(callee.without_parentheses()).is_some_and(|(object, _)| {
        matches!(object.without_parentheses(), Expression::Identifier(id) if id.name == "JSON")
    })
}

fn initializer_name(init: &Expression<'_>) -> Option<String> {
    match init.without_parentheses() {
        Expression::CallExpression(call) => call_result_name(call),
        Expression::AwaitExpression(awaited) => initializer_name(&awaited.argument),
        _ => None,
    }
}

fn is_naming_holder(object: &Expression<'_>) -> bool {
    match object.without_parentheses() {
        Expression::ThisExpression(_) => true,
        Expression::Identifier(id) => NAMING_HOLDERS.contains(&id.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            member.property.name == "prototype"
                || (member.property.name == "exports"
                    && matches!(member.object.without_parentheses(), Expression::Identifier(id) if id.name == "module"))
        }
        _ => false,
    }
}

fn property_name(target: &AssignmentTarget<'_>) -> Option<String> {
    let AssignmentTarget::StaticMemberExpression(member) = target else {
        return None;
    };
    let name = member.property.name.as_str();
    let usable = is_naming_holder(&member.object)
        && !IGNORED_PROPERTIES.contains(&name)
        && looks_human_written(name)
        && is_word_like(name);
    usable.then(|| lower_camel(name))
}

pub fn is_word_like(name: &str) -> bool {
    let mut segments: Vec<String> = Vec::new();
    for ch in name.chars() {
        if !ch.is_ascii_alphabetic() {
            return false;
        }
        match segments.last_mut() {
            Some(segment) if !ch.is_ascii_uppercase() => segment.push(ch),
            _ => segments.push(ch.to_string()),
        }
    }
    let Some(first) = segments.first() else {
        return false;
    };
    first.starts_with(|ch: char| ch.is_ascii_lowercase())
        && segments.iter().all(|segment| is_word_segment(segment))
}

fn is_word_segment(segment: &str) -> bool {
    segment.len() >= MIN_SEGMENT
        && segment
            .chars()
            .any(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
}

impl EvidenceWalker<'_> {
    fn name_params(&mut self, params: &FormalParameters<'_>, names: &[&str]) {
        for (param, name) in params.items.iter().zip(names) {
            if let Some(symbol) = simple_symbol(&param.pattern) {
                self.out.offer(symbol, *name);
            }
        }
    }

    fn name_first_param(&mut self, argument: Option<&Argument<'_>>, name: &str) {
        let Some(params) = argument.and_then(callback_params) else {
            return;
        };
        self.name_params(params, &[name]);
    }

    fn method_callback(&mut self, call: &CallExpression<'_>) {
        let Some(method) = method_name(&call.callee) else {
            return;
        };
        let method = method.as_ref();
        if ELEMENT_CALLBACKS.contains(&method) {
            if let Some(params) = call.arguments.first().and_then(callback_params) {
                self.name_params(params, &ELEMENT_PARAMS);
            }
        } else if REDUCERS.contains(&method) {
            if let Some(params) = call.arguments.first().and_then(callback_params) {
                self.name_params(params, &REDUCER_PARAMS);
            }
        } else if SORTERS.contains(&method) {
            if let Some(params) = call.arguments.first().and_then(callback_params) {
                self.name_params(params, &SORTER_PARAMS);
            }
        } else if method == "then" {
            self.name_first_param(call.arguments.first(), "result");
            self.name_first_param(call.arguments.get(1), "err");
        } else if method == "catch" {
            self.name_first_param(call.arguments.first(), "err");
        } else if EVENT_SUBSCRIBERS.contains(&method) {
            self.name_first_param(call.arguments.get(1), "event");
        }
    }
}

impl<'a> VisitJs<'a> for EvidenceWalker<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.method_callback(it);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        let is_promise = matches!(it.callee.without_parentheses(), Expression::Identifier(id) if id.name == "Promise");
        if is_promise {
            if let Some(params) = it.arguments.first().and_then(callback_params) {
                self.name_params(params, &EXECUTOR_PARAMS);
            }
        }
        walk_js::walk_new_expression(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        if let Some(symbol) = it
            .param
            .as_ref()
            .and_then(|param| simple_symbol(&param.pattern))
        {
            self.out.offer(symbol, "err");
        }
        walk_js::walk_catch_clause(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (Some(symbol), Some(init)) = (simple_symbol(&it.id), it.init.as_ref()) {
            if let Some(name) = initializer_name(init) {
                self.out.offer(symbol, name);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let Expression::Identifier(id) = it.right.without_parentheses() {
                let named = reference_symbol(id, self.scoping).zip(property_name(&it.left));
                if let Some((symbol, name)) = named {
                    self.out.offer(symbol, name);
                }
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }
}
