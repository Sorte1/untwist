use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::reference::ReferenceId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::js_text::js_text;
use untwist_core::ast_util::matcher::is_literal_argument;

use super::definer_calls::{applied_definer_patch, bound_definer_patch};
use super::{Refusal, UsedNames, GLOBAL_OBJECTS, STANDARD_GLOBALS};
use crate::notes;
use crate::stamp::decoder_index;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKey<'a> {
    Named(&'a str),
    Pending(Span),
    Dynamic,
    Whole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prototype {
    No,
    Yes,
    Pending(Span),
    ProtoChain,
}

#[derive(Clone, Copy, Debug)]
pub struct PatchSite<'a> {
    pub span: Span,
    pub root: &'a str,
    pub prototype: Prototype,
    pub global_object: bool,
    pub key: PatchKey<'a>,
    pub references: [Option<ReferenceId>; 2],
}

impl PatchSite<'_> {
    pub fn describe(&self) -> String {
        let prototype = match self.prototype {
            Prototype::No => "",
            Prototype::Yes => ".prototype",
            Prototype::Pending(_) => "[a decoded key that may be prototype]",
            Prototype::ProtoChain => ".__proto__",
        };
        let key = match self.key {
            PatchKey::Named(name) => format!(".{name}"),
            PatchKey::Pending(_) => "[a decoded key]".to_string(),
            PatchKey::Dynamic => "[a computed key]".to_string(),
            PatchKey::Whole => String::new(),
        };
        format!("{}{prototype}{key}", self.root)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PendingPrototype<'a> {
    pub root_key: Span,
    pub key: PatchKey<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtoRoot {
    Array,
    Object,
    Function,
    String,
    Number,
    RegExp,
    Either,
}

impl ProtoRoot {
    pub fn named(root: &str) -> Option<Self> {
        match root {
            "Array" => Some(Self::Array),
            "Object" => Some(Self::Object),
            "Function" => Some(Self::Function),
            "String" => Some(Self::String),
            "Number" => Some(Self::Number),
            "RegExp" => Some(Self::RegExp),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProtoWrite<'a> {
    pub span: Span,
    pub root: ProtoRoot,
    pub root_key: Option<Span>,
    pub key: PatchKey<'a>,
}

#[derive(Default)]
pub struct PatchVerdict<'a> {
    pub refusal: Option<Refusal>,
    pub pending_keys: Vec<Span>,
    pub pending_prototypes: Vec<PendingPrototype<'a>>,
    pub proto_writes: Vec<ProtoWrite<'a>>,
}

pub(crate) fn is_pending_key(call: &CallExpression<'_>) -> bool {
    let decoder = matches!(&call.callee, Expression::Identifier(id)
        if decoder_index(id.name.as_str()).is_some());
    decoder
        && call
            .arguments
            .iter()
            .all(|argument| argument.as_expression().is_some_and(is_literal_argument))
}

const WELL_KNOWN_SYMBOLS: &[(&str, &str)] = &[
    ("iterator", "@@iterator"),
    ("asyncIterator", "@@asyncIterator"),
    ("hasInstance", "@@hasInstance"),
    ("isConcatSpreadable", "@@isConcatSpreadable"),
    ("match", "@@match"),
    ("matchAll", "@@matchAll"),
    ("replace", "@@replace"),
    ("search", "@@search"),
    ("species", "@@species"),
    ("split", "@@split"),
    ("toPrimitive", "@@toPrimitive"),
    ("toStringTag", "@@toStringTag"),
    ("unscopables", "@@unscopables"),
];

fn well_known_symbol_key(expr: &Expression<'_>) -> Option<PatchKey<'static>> {
    let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(id) = member.object.without_parentheses() else {
        return None;
    };
    if id.name.as_str() != "Symbol" {
        return None;
    }
    WELL_KNOWN_SYMBOLS
        .iter()
        .find(|(name, _)| *name == member.property.name.as_str())
        .map(|(_, marker)| PatchKey::Named(marker))
}

pub(crate) fn literal_array_index_key<'a>(expr: &Expression<'a>) -> Option<PatchKey<'a>> {
    let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
        return None;
    };
    let Expression::ArrayExpression(array) = member.object.without_parentheses() else {
        return None;
    };
    let Expression::NumericLiteral(index) = member.expression.without_parentheses() else {
        return None;
    };
    let index = index.value;
    if index < 0.0 || index.fract() != 0.0 {
        return None;
    }
    let element = array.elements.get(index as usize)?;
    let Expression::StringLiteral(text) = element.as_expression()? else {
        return None;
    };
    Some(
        text.value
            .as_str()
            .map_or(PatchKey::Dynamic, PatchKey::Named),
    )
}

pub(crate) fn key_of<'a>(expr: &Expression<'a>) -> PatchKey<'a> {
    match expr.without_parentheses() {
        Expression::StringLiteral(key) => key
            .value
            .as_str()
            .map_or(PatchKey::Dynamic, PatchKey::Named),
        Expression::CallExpression(call) if is_pending_key(call) => PatchKey::Pending(call.span),
        other => well_known_symbol_key(other)
            .or_else(|| literal_array_index_key(other))
            .unwrap_or(PatchKey::Dynamic),
    }
}

fn step_kind(key: PatchKey<'_>) -> Option<Prototype> {
    match key {
        PatchKey::Named("prototype") => Some(Prototype::Yes),
        PatchKey::Named("__proto__") => Some(Prototype::ProtoChain),
        PatchKey::Pending(span) => Some(Prototype::Pending(span)),
        _ => None,
    }
}

fn root_of<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(Option<&'k IdentifierReference<'a>>, Prototype)> {
    let (object, kind) = match expr.without_parentheses() {
        Expression::Identifier(id) => return Some((Some(id), Prototype::No)),
        Expression::StaticMemberExpression(m) => (
            &m.object,
            step_kind(PatchKey::Named(m.property.name.as_str()))?,
        ),
        Expression::ComputedMemberExpression(m) => (&m.object, step_kind(key_of(&m.expression))?),
        _ => return None,
    };
    match (object.without_parentheses(), kind) {
        (_, Prototype::ProtoChain) => Some((None, kind)),
        (Expression::Identifier(id), _) => Some((Some(id), kind)),
        _ => None,
    }
}

fn is_root_name(name: &str) -> bool {
    STANDARD_GLOBALS.contains(&name) || GLOBAL_OBJECTS.contains(&name)
}

fn root_recognized(root: Option<&IdentifierReference<'_>>) -> bool {
    match root {
        None => true,
        Some(id) => is_root_name(id.name.as_str()),
    }
}

fn site<'a>(
    root: Option<&IdentifierReference<'a>>,
    prototype: Prototype,
    key: PatchKey<'a>,
    span: Span,
) -> Option<PatchSite<'a>> {
    if !root_recognized(root) {
        return None;
    }
    let name = root.map_or("__proto__", |id| id.name.as_str());
    Some(PatchSite {
        span,
        root: name,
        prototype,
        global_object: GLOBAL_OBJECTS.contains(&name) && prototype == Prototype::No,
        key,
        references: [root.and_then(|id| id.reference_id.get()), None],
    })
}

pub fn prototype_root_ref<'k, 'a>(
    obj: &'k Expression<'a>,
) -> Option<(&'k IdentifierReference<'a>, ProtoRoot)> {
    match root_of(obj) {
        Some((Some(id), Prototype::Yes)) if is_root_name(id.name.as_str()) => {
            Some((id, named_proto_root(id.name.as_str())))
        }
        _ => None,
    }
}

fn named_proto_root(name: &str) -> ProtoRoot {
    match name {
        "String" => ProtoRoot::String,
        "Array" => ProtoRoot::Array,
        "Object" => ProtoRoot::Object,
        "Function" => ProtoRoot::Function,
        "Number" => ProtoRoot::Number,
        "RegExp" => ProtoRoot::RegExp,
        _ => ProtoRoot::Either,
    }
}

pub fn record_member_patch<'a>(member: &MemberExpression<'a>) -> Option<PatchSite<'a>> {
    let (root, prototype) = root_of(member.object())?;
    if !root_recognized(root) {
        return None;
    }
    let key = match member {
        MemberExpression::StaticMemberExpression(m) => PatchKey::Named(m.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(m) => key_of(&m.expression),
        MemberExpression::PrivateFieldExpression(_) => return None,
    };
    site(root, prototype, key, member.span())
}

pub fn record_identifier_patch<'a>(id: &IdentifierReference<'a>) -> Option<PatchSite<'a>> {
    site(Some(id), Prototype::No, PatchKey::Whole, id.span)
}

const KEYED_DEFINERS: &[&str] = &["defineProperty", "set", "deleteProperty"];
const WHOLE_DEFINERS: &[&str] = &["defineProperties", "assign", "setPrototypeOf"];
const GETTER_DEFINERS: &[&str] = &["__defineGetter__", "__defineSetter__"];

pub(super) fn is_getter_definer(method: &str) -> bool {
    GETTER_DEFINERS.contains(&method)
}

pub(super) fn is_owned_definer(method: &str) -> bool {
    KEYED_DEFINERS.contains(&method) || WHOLE_DEFINERS.contains(&method)
}

pub(super) fn definer_args_site<'a>(
    method: &str,
    arguments: &[&Expression<'a>],
    span: Span,
) -> Option<PatchSite<'a>> {
    let keyed = KEYED_DEFINERS.contains(&method) || is_getter_definer(method);
    if !keyed && !WHOLE_DEFINERS.contains(&method) {
        return None;
    }
    let (root, prototype) = root_of(arguments.first()?)?;
    if !root_recognized(root) {
        return None;
    }
    let key = match arguments.get(1) {
        Some(expr) if keyed => key_of(expr),
        _ => PatchKey::Dynamic,
    };
    site(root, prototype, key, span)
}

pub fn record_definer_patch<'a>(call: &CallExpression<'a>) -> Option<PatchSite<'a>> {
    let callee = call.callee.without_parentheses();
    let Some(member) = callee.as_member_expression() else {
        return bound_definer_patch(callee, call);
    };
    let method = member
        .static_property_name()
        .and_then(|name| name.as_str())?;
    if matches!(method, "call" | "apply") {
        return applied_definer_patch(member, method, call);
    }
    if is_getter_definer(method) {
        let mut arguments = vec![member.object()];
        arguments.extend(call.arguments.iter().map_while(Argument::as_expression));
        return definer_args_site(method, &arguments, call.span);
    }
    let Expression::Identifier(owner) = member.object().without_parentheses() else {
        return None;
    };
    if !matches!(owner.name.as_str(), "Object" | "Reflect") || !is_owned_definer(method) {
        return None;
    }
    let arguments: Vec<&Expression<'a>> = call
        .arguments
        .iter()
        .map_while(Argument::as_expression)
        .collect();
    let mut found = definer_args_site(method, &arguments, call.span)?;
    found.references[1] = owner.reference_id.get();
    Some(found)
}

const SPECIES: &str = "@@species";

pub(super) fn unshadowed(site: &PatchSite<'_>, scoping: &Scoping) -> bool {
    site.references
        .iter()
        .all(|reference| reference.is_none_or(|r| scoping.get_reference(r).symbol_id().is_none()))
}

fn protected_root(site: &PatchSite<'_>) -> Option<ProtoRoot> {
    ProtoRoot::named(site.root)
}

fn protected_write<'a>(
    site: &PatchSite<'a>,
    root: ProtoRoot,
    root_key: Option<Span>,
) -> Decision<'a> {
    Decision::Protected(ProtoWrite {
        span: site.span,
        root,
        root_key,
        key: site.key,
    })
}

pub fn key_relevant(key: &PatchKey<'_>, used: &UsedNames<'_>) -> bool {
    match key {
        PatchKey::Named(name) => used.uses(name),
        PatchKey::Pending(_) => false,
        PatchKey::Dynamic | PatchKey::Whole => true,
    }
}

fn relevant(site: &PatchSite<'_>, used: &UsedNames<'_>) -> bool {
    match &site.key {
        PatchKey::Whole => site.global_object || used.uses(site.root),
        PatchKey::Named(key) if site.global_object => used.uses(key),
        PatchKey::Named(key) if site.prototype == Prototype::No && *key == "prototype" => true,
        other => key_relevant(other, used),
    }
}

enum Decision<'a> {
    Refuse,
    Accept,
    Key(Span),
    Prototype(PendingPrototype<'a>),
    Protected(ProtoWrite<'a>),
}

fn key_named_used(key: PatchKey<'_>, used: &UsedNames<'_>) -> bool {
    matches!(key, PatchKey::Named(name) if used.uses(name))
}

fn decide<'a>(
    site: &PatchSite<'a>,
    used: &UsedNames<'_>,
    own_calls: &FxHashSet<Span>,
) -> Decision<'a> {
    let protected = protected_root(site);
    match (site.prototype, protected) {
        (Prototype::ProtoChain, _) if key_named_used(site.key, used) => return Decision::Refuse,
        (Prototype::ProtoChain, _) => return protected_write(site, ProtoRoot::Either, None),
        (Prototype::No, Some(root)) if site.key == PatchKey::Named(SPECIES) => {
            return protected_write(site, root, None)
        }
        (Prototype::Pending(span), _) if !own_calls.contains(&span) => return Decision::Refuse,
        (Prototype::Yes, Some(root)) => {
            if let PatchKey::Named(name) = site.key {
                if used.uses(name) {
                    return Decision::Refuse;
                }
            }
            return protected_write(site, root, None);
        }
        (Prototype::Pending(span), Some(root)) => return protected_write(site, root, Some(span)),
        (Prototype::Pending(span), None) => {
            return Decision::Prototype(PendingPrototype {
                root_key: span,
                key: site.key,
            })
        }
        (Prototype::Yes | Prototype::No, _) => {}
    }
    match site.key {
        PatchKey::Pending(span) if own_calls.contains(&span) => Decision::Key(span),
        PatchKey::Pending(_) => Decision::Refuse,
        _ if relevant(site, used) => Decision::Refuse,
        _ => Decision::Accept,
    }
}

pub fn patch_refusal<'a>(
    sites: &[PatchSite<'a>],
    scoping: &Scoping,
    used: &UsedNames<'_>,
    own_calls: &FxHashSet<Span>,
) -> PatchVerdict<'a> {
    let mut verdict = PatchVerdict::default();
    if super::builtins_assumed_pristine() {
        return verdict;
    }
    for site in sites.iter().filter(|site| unshadowed(site, scoping)) {
        match decide(site, used, own_calls) {
            Decision::Refuse => {
                verdict.refusal = Some(Refusal::naming(
                    notes::BUILTIN_PATCHED,
                    site.span,
                    site.describe(),
                ));
                return verdict;
            }
            Decision::Accept => {}
            Decision::Key(span) => verdict.pending_keys.push(span),
            Decision::Prototype(pending) => verdict.pending_prototypes.push(pending),
            Decision::Protected(write) => verdict.proto_writes.push(write),
        }
    }
    verdict
}

fn reaches_beyond(key: PatchKey<'_>, harmless: impl Fn(&str) -> bool) -> bool {
    !matches!(key, PatchKey::Named(name) if harmless(name))
}

fn rebinds_object(site: &PatchSite<'_>) -> bool {
    match (site.root, site.prototype) {
        ("Object", Prototype::No) => reaches_beyond(site.key, |name| {
            !matches!(name, "prototype" | "@@hasInstance")
        }),
        ("Object", _) | ("__proto__", _) => true,
        _ => site.global_object && reaches_beyond(site.key, |name| name != "Object"),
    }
}

pub(crate) fn object_operand_is_intrinsic(record: &[PatchSite<'_>]) -> bool {
    !record.iter().any(rebinds_object)
}

fn concat_string(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::StringLiteral(name) => Some(js_text(name.value).into_owned()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            Some(js_text(template.quasis.first()?.value.cooked?).into_owned())
        }
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            Some(concat_string(&binary.left)? + &concat_string(&binary.right)?)
        }
        _ => None,
    }
}

pub fn member_method_name(member: &MemberExpression<'_>) -> Option<String> {
    match member {
        MemberExpression::StaticMemberExpression(m) => Some(m.property.name.as_str().to_string()),
        MemberExpression::ComputedMemberExpression(m) => concat_string(&m.expression),
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}
