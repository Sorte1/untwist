use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::aliased_flows::{is_function_method, is_machinery};
use super::writes::{
    is_getter_definer, is_owned_definer, is_pending_key, key_of, record_definer_patch,
    record_member_patch, PatchKey,
};
use super::{Refusal, UsedNames, GLOBAL_OBJECTS};
use crate::array_rotator::rotator_unit;
use crate::mutants::{builtin_aliases_trusted, function_method_writes_ignored};
use crate::notes;

pub(super) const DANGER: u16 = 1;
pub(super) const FUNCTION: u16 = 2;
pub(super) const CONSOLE: u16 = 4;
pub(super) const DEFINER: u16 = 8;
pub(super) const GETTER_DEFINER: u16 = 16;
pub(super) const LOST: u16 = 32;
pub(super) const OBJECT_CONSTRUCTOR: u16 = 64;
pub(super) const GLOBAL: u16 = 128;
pub(super) const FUNCTION_CONSTRUCTOR: u16 = 256;
pub(super) const CONSTRUCTOR: u16 = 512;
pub(super) const ANY: u16 = 1023;
const FUNCTION_LIKE: u16 = FUNCTION | DEFINER | CONSOLE | FUNCTION_CONSTRUCTOR;
const CONSTRUCTOR_READ: u16 =
    DANGER | OBJECT_CONSTRUCTOR | FUNCTION | FUNCTION_CONSTRUCTOR | CONSTRUCTOR;
const NAMESPACES: &[&str] = &["Math", "JSON", "Intl", "Reflect"];
const CODE_REACHING_BUILTINS: &[&str] = &[
    "return",
    "this",
    "window",
    "self",
    "global",
    "frames",
    "constructor",
    "Function",
    "eval",
    "Object",
    "Reflect",
    "prototype",
    "__proto__",
    "=",
];
const CODE_RESULT: u16 = DANGER | GLOBAL | FUNCTION_CONSTRUCTOR;

const OBJECT_GLOBALS: &[&str] = &[
    "Object",
    "Function",
    "Array",
    "String",
    "Boolean",
    "Number",
    "BigInt",
    "Math",
    "JSON",
    "RegExp",
    "Symbol",
    "Reflect",
    "Proxy",
    "Promise",
    "Date",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "Intl",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "URIError",
    "EvalError",
    "ArrayBuffer",
    "Uint8Array",
    "TextDecoder",
    "TextEncoder",
];

const FUNCTION_GLOBALS: &[&str] = &[
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "unescape",
    "atob",
    "btoa",
    "eval",
];

const PROTOTYPE_GETTERS: &[&str] = &[
    "getPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "__lookupGetter__",
    "__lookupSetter__",
];

const FUNCTION_OWN_KEYS: &[&str] = &["call", "apply", "bind", "toString"];

pub(super) fn member_name<'k>(member: &'k MemberExpression<'_>) -> Option<&'k str> {
    member.static_property_name().and_then(|name| name.as_str())
}

pub(super) fn is_definer(name: &str) -> bool {
    is_owned_definer(name) || is_getter_definer(name)
}

fn global_value(name: &str) -> Option<u16> {
    if name == "console" {
        return Some(CONSOLE);
    }
    if name == "Object" {
        return Some(DANGER | OBJECT_CONSTRUCTOR | CONSTRUCTOR);
    }
    if name == "Reflect" {
        return Some(DANGER | OBJECT_CONSTRUCTOR);
    }
    if name == "Function" {
        return Some(DANGER | FUNCTION_CONSTRUCTOR | CONSTRUCTOR);
    }
    if name == "eval" {
        return Some(FUNCTION | FUNCTION_CONSTRUCTOR);
    }
    if GLOBAL_OBJECTS.contains(&name) {
        return Some(DANGER | GLOBAL);
    }
    if NAMESPACES.contains(&name) {
        return Some(DANGER);
    }
    if OBJECT_GLOBALS.contains(&name) {
        return Some(DANGER | CONSTRUCTOR);
    }
    FUNCTION_GLOBALS.contains(&name).then_some(FUNCTION)
}

const DATA_PROPERTIES: &[&str] = &[
    "length",
    "name",
    "PI",
    "E",
    "LN2",
    "LN10",
    "LOG2E",
    "LOG10E",
    "SQRT2",
    "SQRT1_2",
    "MAX_SAFE_INTEGER",
    "MIN_SAFE_INTEGER",
    "MAX_VALUE",
    "MIN_VALUE",
    "EPSILON",
    "NaN",
    "POSITIVE_INFINITY",
    "NEGATIVE_INFINITY",
    "iterator",
    "asyncIterator",
    "hasInstance",
    "isConcatSpreadable",
    "species",
    "toPrimitive",
    "toStringTag",
    "unscopables",
];

fn named_on_builtin(name: &str) -> u16 {
    if DATA_PROPERTIES.contains(&name) {
        0
    } else if name == "console" {
        CONSOLE
    } else if matches!(name, "__defineGetter__" | "__defineSetter__") {
        DEFINER | GETTER_DEFINER
    } else if is_definer(name) {
        DEFINER | FUNCTION
    } else if global_value(name).is_some() || name == "prototype" {
        global_value(name).map_or(DANGER, |value| value | DANGER)
    } else {
        FUNCTION
    }
}

pub struct BuiltinAliases<'s> {
    pub(super) scoping: &'s Scoping,
    pub(super) values: FxHashMap<SymbolId, u16>,
    pub(super) returns: FxHashMap<Span, u16>,
    pub(super) functions: FxHashMap<SymbolId, Span>,
    pub(super) sloppy_this: FxHashSet<Span>,
    pub(super) this_bits: u16,
    pub(super) lost: u16,
    pub(super) decoded: FxHashMap<Span, String>,
    pub(super) object_functions: FxHashMap<SymbolId, FxHashMap<String, Span>>,
    pub(super) function_methods_written: bool,
    pub(super) strings: FxHashMap<SymbolId, Vec<String>>,
    pub(super) unknown_strings: FxHashSet<SymbolId>,
    pub(super) string_arrays: FxHashMap<SymbolId, Vec<String>>,
    pub(super) string_objects: FxHashMap<SymbolId, FxHashMap<String, Vec<String>>>,
}

const KEY_SET_LIMIT: usize = 64;

fn merged(mut left: Vec<String>, right: Vec<String>) -> Option<Vec<String>> {
    for name in right {
        if !left.contains(&name) {
            left.push(name);
        }
    }
    (left.len() <= KEY_SET_LIMIT).then_some(left)
}

impl<'s> BuiltinAliases<'s> {
    pub(super) fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    pub(super) fn expand(&self, bits: u16) -> u16 {
        if bits & LOST != 0 {
            (bits & !LOST) | self.lost
        } else {
            bits
        }
    }

    fn identifier(&self, id: &IdentifierReference<'_>) -> u16 {
        match self.symbol_of(id) {
            Some(symbol) => self.expand(self.values.get(&symbol).copied().unwrap_or(0)),
            None => global_value(id.name.as_str()).unwrap_or(self.lost),
        }
    }

    pub(super) fn function_span(&self, expr: &Expression<'_>) -> Option<Span> {
        match expr.without_parentheses() {
            Expression::FunctionExpression(function) => Some(function.span),
            Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
            Expression::Identifier(id) => self
                .symbol_of(id)
                .and_then(|s| self.functions.get(&s).copied()),
            other => {
                let member = other.as_member_expression()?;
                let Expression::Identifier(id) = member.object().without_parentheses() else {
                    return None;
                };
                let key = match member {
                    MemberExpression::ComputedMemberExpression(m) => {
                        match m.expression.without_parentheses() {
                            Expression::StringLiteral(text) => {
                                text.value.as_str().map(str::to_string)
                            }
                            Expression::CallExpression(call) => {
                                self.decoded.get(&call.span).cloned()
                            }
                            _ => None,
                        }
                    }
                    other => member_name(other).map(str::to_string),
                }?;
                self.object_functions
                    .get(&self.symbol_of(id)?)?
                    .get(&key)
                    .copied()
            }
        }
    }

    pub(super) fn returned(&self, function: Span) -> u16 {
        self.expand(self.returns.get(&function).copied().unwrap_or(0))
    }

    pub(super) fn key_set(&self, expr: &Expression<'_>) -> Option<Vec<String>> {
        match expr.without_parentheses() {
            Expression::StringLiteral(text) => Some(vec![text.value.as_str()?.to_string()]),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                let cooked = template.quasis.first()?.value.cooked.as_ref()?;
                Some(vec![cooked.as_str()?.to_string()])
            }
            Expression::CallExpression(call) => Some(vec![self.decoded.get(&call.span)?.clone()]),
            Expression::BinaryExpression(binary)
                if binary.operator == oxc_syntax::operator::BinaryOperator::Addition =>
            {
                let (left, right) = (self.key_set(&binary.left)?, self.key_set(&binary.right)?);
                let joined: Vec<String> = left
                    .iter()
                    .flat_map(|l| right.iter().map(move |r| format!("{l}{r}")))
                    .collect();
                (joined.len() <= KEY_SET_LIMIT).then_some(joined)
            }
            Expression::ConditionalExpression(conditional) => merged(
                self.key_set(&conditional.consequent)?,
                self.key_set(&conditional.alternate)?,
            ),
            Expression::LogicalExpression(logical) => {
                merged(self.key_set(&logical.left)?, self.key_set(&logical.right)?)
            }
            Expression::Identifier(id) => {
                let symbol = self.symbol_of(id)?;
                if self.unknown_strings.contains(&symbol) {
                    return None;
                }
                Some(self.strings.get(&symbol).cloned().unwrap_or_default())
            }
            Expression::ComputedMemberExpression(member) => {
                let Expression::Identifier(id) = member.object.without_parentheses() else {
                    return None;
                };
                let symbol = self.symbol_of(id)?;
                if let Some(names) = self.string_arrays.get(&symbol) {
                    return Some(names.clone());
                }
                let keys = self.key_set(&member.expression)?;
                self.object_strings(symbol, &keys)
            }
            Expression::StaticMemberExpression(member) => {
                let Expression::Identifier(id) = member.object.without_parentheses() else {
                    return None;
                };
                let key = vec![member.property.name.as_str().to_string()];
                self.object_strings(self.symbol_of(id)?, &key)
            }
            _ => None,
        }
    }

    fn code_result(&self, call: &CallExpression<'_>) -> u16 {
        let texts = call
            .arguments
            .iter()
            .try_fold(Vec::new(), |mut found, argument| {
                found.extend(self.key_set(argument.as_expression()?)?);
                Some(found)
            });
        let quiet = texts.is_some_and(|texts| {
            texts.iter().all(|text| {
                !CODE_REACHING_BUILTINS
                    .iter()
                    .any(|word| text.contains(word))
            })
        });
        if quiet {
            FUNCTION
        } else {
            CODE_RESULT
        }
    }

    fn object_strings(&self, symbol: SymbolId, keys: &[String]) -> Option<Vec<String>> {
        let properties = self.string_objects.get(&symbol)?;
        keys.iter().try_fold(Vec::new(), |found, key| {
            merged(found, properties.get(key)?.clone())
        })
    }

    fn calls_own_method(&self, object: u16) -> bool {
        (function_method_writes_ignored() || !self.function_methods_written)
            && object & (DEFINER | GETTER_DEFINER | LOST) == 0
    }

    fn single_key(&self, member: &MemberExpression<'_>) -> Option<String> {
        let MemberExpression::ComputedMemberExpression(computed) = member else {
            return None;
        };
        match self.key_set(&computed.expression)?.as_slice() {
            [name] => Some(name.clone()),
            _ => None,
        }
    }

    fn decoded_key_read(&self, member: &MemberExpression<'_>, object: u16) -> Option<u16> {
        let MemberExpression::ComputedMemberExpression(computed) = member else {
            return None;
        };
        let names = self.key_set(&computed.expression)?;
        Some(names.iter().fold(0, |bits, name| {
            let read = match name.as_str() {
                "constructor" => self.constructor_of(&computed.object),
                other => self.named_read(other, object),
            };
            bits | read
        }))
    }

    fn constructor_of(&self, object: &Expression<'_>) -> u16 {
        match object.without_parentheses() {
            Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => DANGER | CONSTRUCTOR | FUNCTION_CONSTRUCTOR,
            Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::ArrayExpression(_)
            | Expression::RegExpLiteral(_) => DANGER | CONSTRUCTOR,
            _ => CONSTRUCTOR_READ,
        }
    }

    fn named_read(&self, name: &str, object: u16) -> u16 {
        match name {
            "__proto__" => return DANGER,
            "constructor" => return CONSTRUCTOR_READ,
            _ => {}
        }
        let mut bits = 0;
        if object & DANGER != 0 {
            bits |= named_on_builtin(name);
        }
        if object & FUNCTION_LIKE != 0 {
            bits |= match name {
                "bind" | "call" | "apply" => {
                    object & (FUNCTION | DEFINER | GETTER_DEFINER | FUNCTION_CONSTRUCTOR)
                }
                _ => FUNCTION,
            };
        }
        bits
    }

    fn computed_on(&self, object: u16) -> u16 {
        let mut bits = 0;
        if object & DANGER != 0 {
            bits |= DANGER | FUNCTION | CONSOLE | CONSTRUCTOR;
        }
        if object & CONSTRUCTOR != 0 {
            bits |= FUNCTION_CONSTRUCTOR;
        }
        if object & GLOBAL != 0 {
            bits |= OBJECT_CONSTRUCTOR | GLOBAL | FUNCTION_CONSTRUCTOR;
        }
        if object & OBJECT_CONSTRUCTOR != 0 {
            bits |= DEFINER | GETTER_DEFINER;
        }
        if object & FUNCTION_LIKE != 0 {
            bits |= FUNCTION | FUNCTION_CONSTRUCTOR;
        }
        bits
    }

    fn member(&self, member: &MemberExpression<'_>) -> u16 {
        let name = member_name(member);
        match name {
            Some("__proto__") => return DANGER | self.lost,
            Some("constructor") => return self.constructor_of(member.object()) | self.lost,
            _ => {}
        }
        let object = self.value(member.object());
        let Some(name) = name else {
            if let Some(bits) = self.decoded_key_read(member, object) {
                return bits | self.lost;
            }
            return self.computed_on(object) | self.lost;
        };
        self.named_read(name, object) | self.lost
    }

    fn call(&self, call: &CallExpression<'_>) -> u16 {
        if self.decoded.contains_key(&call.span) {
            return 0;
        }
        let callee = call.callee.without_parentheses();
        if let Some(member) = callee.as_member_expression() {
            let name = member_name(member);
            if name.is_some_and(|name| PROTOTYPE_GETTERS.contains(&name)) {
                return DANGER | self.lost;
            }
            if name == Some("get") && self.value(member.object()) & DANGER != 0 {
                let target = call.arguments.first().and_then(Argument::as_expression);
                let target = target.map_or(ANY & !LOST, |t| self.value(t));
                return self.computed_on(target) | self.lost;
            }
            if self.value(callee) & FUNCTION_CONSTRUCTOR != 0 {
                return self.code_result(call) | self.lost;
            }
            if name == Some("bind") {
                return self.value(member.object());
            }
            if matches!(name, Some("call" | "apply")) {
                if let Some(function) = self.function_span(member.object()) {
                    return self.returned(function);
                }
            }
            return self.lost;
        }
        if let Some(function) = self.function_span(callee) {
            return self.returned(function);
        }
        if self.value(callee) & FUNCTION_CONSTRUCTOR != 0 {
            return self.code_result(call) | self.lost;
        }
        self.lost
    }

    pub(super) fn value(&self, expr: &Expression<'_>) -> u16 {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.identifier(id),
            Expression::ThisExpression(this) => {
                let sloppy = if self.sloppy_this.contains(&this.span) {
                    DANGER | GLOBAL
                } else {
                    0
                };
                sloppy | self.this_bits
            }
            Expression::CallExpression(call) => self.call(call),
            Expression::NewExpression(new) => match self.function_span(&new.callee) {
                Some(function) => self.returned(function),
                None if self.value(&new.callee) & FUNCTION_CONSTRUCTOR != 0 => {
                    CODE_RESULT | self.lost
                }
                None => self.lost,
            },
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => self.call(call),
                other => other
                    .as_member_expression()
                    .map_or(self.lost, |m| self.member(m)),
            },
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .map_or(0, |last| self.value(last)),
            Expression::ConditionalExpression(conditional) => {
                self.value(&conditional.consequent) | self.value(&conditional.alternate)
            }
            Expression::LogicalExpression(logical) => {
                self.value(&logical.left) | self.value(&logical.right)
            }
            Expression::AssignmentExpression(assignment) => self.value(&assignment.right),
            Expression::AwaitExpression(_)
            | Expression::YieldExpression(_)
            | Expression::TaggedTemplateExpression(_)
            | Expression::ImportExpression(_) => self.lost,
            other => other
                .as_member_expression()
                .map_or(0, |member| self.member(member)),
        }
    }

    fn site_key(&self, key: PatchKey<'_>, expr: Option<&Expression<'_>>) -> AliasKey {
        match key {
            PatchKey::Named(name) => AliasKey::Names(vec![name.to_string()]),
            PatchKey::Pending(span) => match self.decoded.get(&span) {
                Some(name) => AliasKey::Names(vec![name.clone()]),
                None => AliasKey::Unknown,
            },
            PatchKey::Dynamic => match expr.and_then(|expr| self.key_set(expr)) {
                Some(names) => AliasKey::Names(names),
                None => AliasKey::Unknown,
            },
            PatchKey::Whole => AliasKey::Unknown,
        }
    }

    fn member_site(&self, member: &MemberExpression<'_>) -> Option<AliasSite> {
        let (key, expr) = match member {
            MemberExpression::StaticMemberExpression(m) => {
                (PatchKey::Named(m.property.name.as_str()), None)
            }
            MemberExpression::ComputedMemberExpression(m) => {
                (key_of(&m.expression), Some(&m.expression))
            }
            MemberExpression::PrivateFieldExpression(_) => return None,
        };
        let functions_only = self.functions_only(self.value(member.object()))?;
        Some(AliasSite {
            span: member.span(),
            key: self.site_key(key, expr),
            functions_only,
        })
    }

    fn definer_sites(&self, call: &CallExpression<'_>) -> Vec<AliasSite> {
        let callee = call.callee.without_parentheses();
        let arguments: Vec<&Expression<'_>> = call
            .arguments
            .iter()
            .map_while(Argument::as_expression)
            .collect();
        let value_at = |index: usize| arguments.get(index).map_or(0, |e| self.value(e));
        let key_at = |index: usize| match arguments.get(index) {
            Some(e) => self.site_key(key_of(e), Some(e)),
            None => AliasKey::Names(vec!["undefined".to_string()]),
        };
        let mut sites: Vec<(u16, AliasKey)> = Vec::new();
        if let Some(member) = callee.as_member_expression() {
            let object = self.value(member.object());
            let resolved = self.single_key(member);
            let name = member_name(member).or(resolved.as_deref());
            if name.is_some_and(is_function_method) && self.calls_own_method(object) {
                return Vec::new();
            }
            match name {
                Some(name) if is_getter_definer(name) => sites.push((object, key_at(0))),
                Some(name) if is_owned_definer(name) => {
                    let key = match name {
                        "defineProperty" | "set" | "deleteProperty" => key_at(1),
                        "assign" => literal_keys(arguments.get(1..).unwrap_or_default()),
                        "defineProperties" => literal_keys(arguments.get(1..2).unwrap_or_default()),
                        _ => AliasKey::Unknown,
                    };
                    sites.push((value_at(0), key));
                }
                Some("call") if object & DEFINER != 0 => {
                    sites.push((value_at(1), key_at(2)));
                    if object & GETTER_DEFINER != 0 {
                        sites.push((value_at(0) | GLOBAL | DANGER, key_at(1)));
                    }
                }
                Some("apply") if object & DEFINER != 0 => {
                    sites.push((value_at(1) | self.lost | DANGER, AliasKey::Unknown));
                }
                _ => {
                    let value = self.member(member);
                    if value & DEFINER != 0 {
                        sites.push((value_at(0), key_at(1)));
                    }
                    if value & GETTER_DEFINER != 0 {
                        sites.push((object, key_at(0)));
                    }
                }
            }
        } else {
            let value = self.value(callee);
            if value & DEFINER != 0 {
                sites.push((value_at(0), key_at(1)));
            }
            if value & GETTER_DEFINER != 0 {
                sites.push((GLOBAL | DANGER, key_at(0)));
            }
        }
        sites
            .into_iter()
            .filter_map(|(target, key)| {
                Some(AliasSite {
                    span: call.span,
                    key,
                    functions_only: self.functions_only(target)?,
                })
            })
            .collect()
    }

    fn functions_only(&self, bits: u16) -> Option<bool> {
        if builtin_aliases_trusted() || bits & !LOST == 0 {
            return None;
        }
        Some(bits & (DANGER | GETTER_DEFINER | OBJECT_CONSTRUCTOR | GLOBAL) == 0)
    }
}

fn literal_key(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(text) => text.value.as_str().map(str::to_string),
        PropertyKey::NumericLiteral(number) => Some(number.value.to_string()),
        _ => None,
    }
}

fn object_keys(expr: &Expression<'_>) -> Option<Vec<String>> {
    let Expression::ObjectExpression(object) = expr.without_parentheses() else {
        return None;
    };
    object
        .properties
        .iter()
        .map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => literal_key(&property.key),
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
        .collect()
}

fn literal_keys(sources: &[&Expression<'_>]) -> AliasKey {
    let keys: Option<Vec<Vec<String>>> = sources.iter().map(|source| object_keys(source)).collect();
    match keys {
        Some(keys) if !sources.is_empty() => AliasKey::Names(keys.concat()),
        _ => AliasKey::Unknown,
    }
}

enum AliasKey {
    Names(Vec<String>),
    Unknown,
}

pub struct AliasSite {
    span: Span,
    key: AliasKey,
    functions_only: bool,
}

fn position_of(source: &str, span: Span) -> Option<(usize, usize)> {
    let before = source.get(..span.start as usize)?;
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    Some((line, before[line_start..].chars().count() + 1))
}

impl AliasSite {
    fn refuses(&self, used: &UsedNames<'_>) -> bool {
        match &self.key {
            AliasKey::Names(names) => names.iter().any(|name| used.uses(name)),
            AliasKey::Unknown if self.functions_only => {
                FUNCTION_OWN_KEYS.iter().any(|key| used.uses(key))
            }
            AliasKey::Unknown => true,
        }
    }
}

struct SiteScan<'s, 'r> {
    aliases: &'r BuiltinAliases<'s>,
    skip: &'r [Span],
    sites: Vec<AliasSite>,
}

impl SiteScan<'_, '_> {
    fn written(&mut self, member: Option<&MemberExpression<'_>>) {
        let Some(member) = member else {
            return;
        };
        if record_member_patch(member).is_none() {
            self.sites.extend(self.aliases.member_site(member));
        }
    }
}

impl<'a> VisitJs<'a> for SiteScan<'_, '_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let skipped = self.skip.contains(&it.span()) || rotator_unit(it).is_some();
        if !is_machinery(it) && !skipped {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.written(it.as_member_expression());
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            self.written(it.argument.without_parentheses().as_member_expression());
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if record_definer_patch(it).is_none() {
            self.sites.extend(self.aliases.definer_sites(it));
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub fn alias_sites(
    program: &Program<'_>,
    aliases: &BuiltinAliases<'_>,
    skip: &[Span],
) -> Vec<AliasSite> {
    let mut scan = SiteScan {
        aliases,
        skip,
        sites: Vec::new(),
    };
    scan.visit_program(program);
    scan.sites
}

pub fn alias_refusal(sites: &[AliasSite], used: &UsedNames<'_>, source: &str) -> Option<Refusal> {
    let site = sites.iter().find(|site| site.refuses(used))?;
    let key = match &site.key {
        AliasKey::Names(names) => format!("key {}", names.join(" or ")),
        AliasKey::Unknown => "computed key".to_string(),
    };
    let detail = match position_of(source, site.span) {
        Some((line, column)) => format!("the write at line {line}:{column}, {key}"),
        None => format!("a write in code untwist built, {key}"),
    };
    Some(Refusal::naming(
        notes::BUILTIN_ALIAS_PATCHED,
        site.span,
        detail,
    ))
}

#[derive(Default)]
struct PendingKeys {
    found: Vec<Span>,
}

impl<'a> VisitJs<'a> for PendingKeys {
    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Expression::CallExpression(call) = it.expression.without_parentheses() {
            if is_pending_key(call) {
                self.found.push(call.span);
            }
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

pub fn pending_member_keys(program: &Program<'_>) -> Vec<Span> {
    let mut keys = PendingKeys::default();
    keys.visit_program(program);
    keys.found
}
