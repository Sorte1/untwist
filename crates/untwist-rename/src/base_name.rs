use crate::collect::{BindingInfo, BindingRole, Collected, InitKind, InitShape};
use crate::generate::{lower_camel, strip_accessor_prefix, to_identifier};
use crate::human::looks_human_written;
use crate::import_path::import_base;
use crate::simple::is_renameable;
use crate::usage::Usage;
use crate::RenameConfig;
use oxc_syntax::symbol::SymbolId;

pub const FALLBACK_BASE: &str = "v";

const ALIAS_DEPTH: usize = 4;

const COUNTER_NAMES: [&str; 3] = ["i", "j", "k"];

const COMPONENT: &str = "Component";

const DOM_LOOKUPS: [&str; 6] = [
    "getElementById",
    "querySelector",
    "querySelectorAll",
    "getElementsByClassName",
    "getElementsByTagName",
    "createElement",
];

const RESPONSE_PROPS: [&str; 8] = [
    "send",
    "json",
    "status",
    "redirect",
    "render",
    "end",
    "sendFile",
    "setHeader",
];

const REQUEST_PROPS: [&str; 8] = [
    "body",
    "params",
    "query",
    "headers",
    "cookies",
    "originalUrl",
    "baseUrl",
    "method",
];

const ELEMENT_PROPS: [&str; 8] = [
    "classList",
    "innerHTML",
    "textContent",
    "appendChild",
    "addEventListener",
    "style",
    "dataset",
    "removeChild",
];

const ERROR_PROPS: [&str; 3] = ["message", "stack", "code"];

pub trait BaseNamer {
    fn base_name(&self, binding: &BindingInfo, usage: &Usage) -> Option<String>;
}

pub struct Context<'c> {
    pub collected: &'c Collected,
    pub usage: &'c Usage,
    pub namer: Option<&'c dyn BaseNamer>,
    pub config: RenameConfig,
}

pub fn base_name(info: &BindingInfo, cx: &Context<'_>) -> String {
    derive(info, cx, &mut Vec::new())
}

fn derive(info: &BindingInfo, cx: &Context<'_>, reading: &mut Vec<SymbolId>) -> String {
    reading.push(info.symbol);
    let candidate = cx
        .namer
        .and_then(|namer| namer.base_name(info, cx.usage))
        .or_else(|| counter_base(info, cx))
        .or_else(|| component_base(info, cx))
        .or_else(|| class_hint_base(info, cx))
        .or_else(|| usage_base(info, cx, reading))
        .or_else(|| structural_base(info));
    reading.pop();
    sanitize(candidate)
}

fn counter_base(info: &BindingInfo, cx: &Context<'_>) -> Option<String> {
    let depth = *cx.collected.loop_counters.get(&info.symbol)?;
    Some(COUNTER_NAMES[depth % COUNTER_NAMES.len()].to_string())
}

fn component_base(info: &BindingInfo, cx: &Context<'_>) -> Option<String> {
    if !cx.collected.jsx_tags.contains(&info.symbol) {
        return None;
    }
    let rest = info.name.strip_prefix('_').unwrap_or(&info.name);
    let suffix = rest.strip_prefix(COMPONENT)?;
    let digits = suffix
        .strip_prefix('_')
        .and_then(|inner| inner.strip_suffix("__"))
        .unwrap_or(suffix);
    digits
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| COMPONENT.to_string())
}

fn class_hint_base(info: &BindingInfo, cx: &Context<'_>) -> Option<String> {
    let inner = match &info.role {
        BindingRole::Class | BindingRole::Function => None,
        BindingRole::Variable(shape)
            if matches!(shape.kind, InitKind::Class | InitKind::Function) =>
        {
            shape.symbol
        }
        _ => return None,
    };
    let hint = cx
        .usage
        .class_hint(info.symbol)
        .or_else(|| inner.and_then(|symbol| cx.usage.class_hint(symbol)))?;
    let applies = !looks_human_written(&info.name) || is_numbered(&info.name, hint);
    applies.then(|| hint.to_string())
}

fn is_numbered(name: &str, base: &str) -> bool {
    name.strip_prefix(base)
        .is_some_and(|digits| digits.chars().all(|ch| ch.is_ascii_digit()))
}

fn sanitize(candidate: Option<String>) -> String {
    let Some(candidate) = candidate else {
        return FALLBACK_BASE.to_string();
    };
    let identifier = to_identifier(&candidate);
    if identifier.is_empty() || identifier == "_" {
        return FALLBACK_BASE.to_string();
    }
    identifier
}

fn usage_base(info: &BindingInfo, cx: &Context<'_>, reading: &mut Vec<SymbolId>) -> Option<String> {
    if info.role == BindingRole::CatchParam {
        return None;
    }
    if let Some(base) = module_base(info) {
        return Some(base);
    }
    if let Some(key) = cx.usage.object_literal_key(info.symbol) {
        return Some(key.to_string());
    }
    if let BindingRole::PatternProperty(key) = &info.role {
        return Some(key.clone());
    }
    if let BindingRole::Variable(shape) = &info.role {
        if let Some(base) = variable_base(info, shape, cx, reading) {
            return Some(base);
        }
    }
    role_word(info, cx).map(str::to_string)
}

fn module_base(info: &BindingInfo) -> Option<String> {
    match &info.role {
        BindingRole::ImportDefault(source) | BindingRole::ImportNamespace(source) => {
            import_base(source)
        }
        BindingRole::ImportNamed { imported, .. } => Some(imported.clone()),
        BindingRole::Variable(shape) if shape.kind == InitKind::Require => {
            import_base(shape.name.as_deref()?)
        }
        _ => None,
    }
}

fn binding_of<'c>(cx: &'c Context<'_>, symbol: SymbolId) -> Option<&'c BindingInfo> {
    let index = *cx.collected.by_symbol.get(&symbol)?;
    cx.collected.bindings.get(index)
}

fn alias_base(shape: &InitShape, cx: &Context<'_>, reading: &mut Vec<SymbolId>) -> Option<String> {
    let mut current = shape.symbol?;
    for _ in 0..ALIAS_DEPTH {
        let info = binding_of(cx, current)?;
        if let Some(base) = module_base(info) {
            return Some(base);
        }
        let BindingRole::Variable(next) = &info.role else {
            return structural_base(info);
        };
        if next.kind != InitKind::Identifier {
            return variable_base(info, next, cx, reading).or_else(|| init_base(next.kind));
        }
        current = next.symbol?;
    }
    None
}

fn variable_base(
    info: &BindingInfo,
    shape: &InitShape,
    cx: &Context<'_>,
    reading: &mut Vec<SymbolId>,
) -> Option<String> {
    let make: fn(&str) -> Option<String> = match shape.kind {
        InitKind::New => construction_base,
        InitKind::Call => call_base,
        _ => return initializer_base(shape, cx, reading),
    };
    match renamed_callee(shape, cx) {
        Some(callee) => callee_base(info, callee, make, cx, reading),
        None => make(shape.name.as_deref()?),
    }
}

fn renamed_callee<'c>(shape: &InitShape, cx: &'c Context<'_>) -> Option<&'c BindingInfo> {
    let callee = binding_of(cx, shape.symbol?)?;
    is_renameable(callee, cx.collected, cx.config).then_some(callee)
}

fn callee_base(
    info: &BindingInfo,
    callee: &BindingInfo,
    make: fn(&str) -> Option<String>,
    cx: &Context<'_>,
    reading: &mut Vec<SymbolId>,
) -> Option<String> {
    if is_informative(&callee.name) {
        return make(&callee.name);
    }
    if reading.contains(&callee.symbol) || reading.len() > ALIAS_DEPTH {
        return None;
    }
    let received = make(&derive(callee, cx, reading))?;
    if is_numbered(&info.name, &received) {
        return Some(received);
    }
    Some(numbered_stem(&info.name).unwrap_or(received))
}

fn is_informative(name: &str) -> bool {
    looks_human_written(name) && !name.ends_with(|ch: char| ch.is_ascii_digit())
}

fn numbered_stem(name: &str) -> Option<String> {
    let stem = name.trim_end_matches(|ch: char| ch.is_ascii_digit());
    let numbered = stem.len() < name.len();
    (numbered && looks_human_written(name) && is_informative(stem)).then(|| stem.to_string())
}

fn initializer_base(
    shape: &InitShape,
    cx: &Context<'_>,
    reading: &mut Vec<SymbolId>,
) -> Option<String> {
    match shape.kind {
        InitKind::Member => shape.name.clone(),
        InitKind::Identifier => alias_base(shape, cx, reading),
        _ => None,
    }
}

fn construction_base(name: &str) -> Option<String> {
    if name.ends_with("Error") {
        return Some("err".to_string());
    }
    Some(lower_camel(name))
}

fn call_base(name: &str) -> Option<String> {
    if DOM_LOOKUPS.contains(&name) {
        return Some("el".to_string());
    }
    strip_accessor_prefix(name).or_else(|| Some(lower_camel(name)))
}

fn role_word(info: &BindingInfo, cx: &Context<'_>) -> Option<&'static str> {
    let usage = cx.usage;
    if usage.reads_any(info.symbol, &RESPONSE_PROPS) {
        return Some("res");
    }
    if usage.reads_any(info.symbol, &REQUEST_PROPS) {
        return Some("req");
    }
    if usage.reads_any(info.symbol, &ELEMENT_PROPS) {
        return Some("el");
    }
    if usage.reads_any(info.symbol, &ERROR_PROPS) {
        return Some("err");
    }
    if !usage.called.contains(&info.symbol) {
        return None;
    }
    if info.role == BindingRole::Param {
        return Some("cb");
    }
    Some("fn")
}

fn structural_base(info: &BindingInfo) -> Option<String> {
    match &info.role {
        BindingRole::Function => Some("fn".to_string()),
        BindingRole::Class => Some("cls".to_string()),
        BindingRole::CatchParam => Some("e".to_string()),
        BindingRole::Variable(shape) => init_base(shape.kind),
        _ => None,
    }
}

fn init_base(kind: InitKind) -> Option<String> {
    let word = match kind {
        InitKind::Function => "fn",
        InitKind::Class => "cls",
        InitKind::Array => "arr",
        InitKind::String => "str",
        InitKind::Number => "num",
        InitKind::Boolean => "bool",
        InitKind::Object => "obj",
        InitKind::Regex => "re",
        _ => return None,
    };
    Some(word.to_string())
}
