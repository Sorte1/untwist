use oxc_allocator::{Box as ABox, CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_str::Ident;
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::values::into_inner;

pub struct NameGuard<'n> {
    pub name: &'n str,
    pub symbol: SymbolId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Key,
    Value,
    Get,
    Set,
}

const STATIC_REFUSED: [&str; 5] = ["prototype", "name", "length", "caller", "arguments"];

fn slot_of(property: &ObjectProperty<'_>) -> Option<Slot> {
    if property.computed || property.shorthand || property.kind != PropertyKind::Init {
        return None;
    }
    let name = match &property.key {
        PropertyKey::StaticIdentifier(id) => id.name.as_str(),
        PropertyKey::StringLiteral(literal) => literal.value.as_str()?,
        _ => return None,
    };
    match name {
        "key" => Some(Slot::Key),
        "value" => Some(Slot::Value),
        "get" => Some(Slot::Get),
        "set" => Some(Slot::Set),
        _ => None,
    }
}

pub fn static_key_name<'k>(key: &'k Expression<'_>) -> Option<&'k str> {
    match key.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn is_global_symbol_member(key: &Expression<'_>, scoping: &Scoping) -> bool {
    let Expression::StaticMemberExpression(member) = key.without_parentheses() else {
        return false;
    };
    let Expression::Identifier(root) = member.object.without_parentheses() else {
        return false;
    };
    !member.optional
        && root.name.as_str() == "Symbol"
        && root
            .reference_id
            .get()
            .is_some_and(|id| scoping.get_reference(id).symbol_id().is_none())
}

pub fn key_is_pure(key: &Expression<'_>, scoping: &Scoping) -> bool {
    static_key_name(key).is_some()
        || matches!(key.without_parentheses(), Expression::NumericLiteral(_))
        || is_global_symbol_member(key, scoping)
}

pub fn key_allowed(key: &Expression<'_>, is_static: bool) -> bool {
    match static_key_name(key) {
        Some(name) if is_static => !STATIC_REFUSED.contains(&name),
        Some(name) => name != "constructor",
        None => true,
    }
}

struct Capture<'g> {
    guard: &'g NameGuard<'g>,
    scoping: &'g Scoping,
    found: bool,
}

impl<'a> VisitJs<'a> for Capture<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name.as_str() != self.guard.name {
            return;
        }
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        if symbol != Some(self.guard.symbol) {
            self.found = true;
        }
    }
}

fn captures(function: &Function<'_>, guard: &NameGuard<'_>, scoping: &Scoping) -> bool {
    let mut capture = Capture {
        guard,
        scoping,
        found: false,
    };
    if let Some(scope) = function.scope_id.get() {
        capture.visit_function(function, scoping.scope_flags(scope));
    }
    capture.found
}

pub struct Context<'g> {
    pub guard: Option<NameGuard<'g>>,
    pub strict: bool,
}

fn function_ok(
    value: &Expression<'_>,
    slot: Slot,
    context: &Context<'_>,
    scoping: &Scoping,
) -> bool {
    let Expression::FunctionExpression(function) = value.without_parentheses() else {
        return false;
    };
    let guard = context.guard.as_ref();
    let strict = context.strict || function.has_use_strict_directive();
    let own_name_unused = function
        .id
        .as_ref()
        .and_then(|id| id.symbol_id.get())
        .is_none_or(|symbol| scoping.get_resolved_reference_ids(symbol).is_empty());
    let arity = match slot {
        Slot::Get => function.params.items.is_empty() && function.params.rest.is_none(),
        Slot::Set => function.params.items.len() == 1 && function.params.rest.is_none(),
        _ => true,
    };
    let accessor_plain = slot == Slot::Value || (!function.r#async && !function.generator);
    strict
        && own_name_unused
        && arity
        && accessor_plain
        && function.body.is_some()
        && !guard.is_some_and(|guard| captures(function, guard, scoping))
}

fn descriptor_ok(
    object: &ObjectExpression<'_>,
    is_static: bool,
    context: &Context<'_>,
    scoping: &Scoping,
) -> bool {
    let mut seen = [false; 4];
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        let Some(slot) = slot_of(property) else {
            return false;
        };
        let index = slot as usize;
        if seen.get(index).copied().unwrap_or(true) {
            return false;
        }
        if let Some(flag) = seen.get_mut(index) {
            *flag = true;
        }
        let ok = match slot {
            Slot::Key => {
                key_is_pure(&property.value, scoping) && key_allowed(&property.value, is_static)
            }
            _ => function_ok(&property.value, slot, context, scoping),
        };
        if !ok {
            return false;
        }
    }
    let [key, value, get, set] = seen;
    key && (value != (get || set))
}

pub fn admissible(
    array: &ArrayExpression<'_>,
    is_static: bool,
    context: &Context<'_>,
    scoping: &Scoping,
) -> bool {
    array.elements.iter().all(|element| match element {
        ArrayExpressionElement::ObjectExpression(object) => {
            descriptor_ok(object, is_static, context, scoping)
        }
        _ => false,
    })
}

pub fn property_key<'a>(key: Expression<'a>, cx: &Cx<'a, '_>) -> (PropertyKey<'a>, bool) {
    let key = into_inner(key);
    if let Expression::StringLiteral(literal) = &key {
        if let Some(name) = identifier_text(literal) {
            let key = PropertyKey::new_static_identifier(literal.span, Ident::from(name), cx);
            return (key, false);
        }
    }
    let computed = !matches!(
        key,
        Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    );
    (PropertyKey::from(key), computed)
}

fn identifier_text<'a>(literal: &StringLiteral<'a>) -> Option<&'a str> {
    literal
        .value
        .as_str()
        .filter(|name| is_identifier_name(name))
}

fn function_of(value: Expression<'_>) -> Option<ABox<'_, Function<'_>>> {
    match into_inner(value) {
        Expression::FunctionExpression(mut function) => {
            function.id = None;
            Some(function)
        }
        _ => None,
    }
}

fn method<'a>(
    key: Expression<'a>,
    function: ABox<'a, Function<'a>>,
    kind: MethodDefinitionKind,
    is_static: bool,
    cx: &Cx<'a, '_>,
) -> ClassElement<'a> {
    let (key, computed) = property_key(key, cx);
    ClassElement::new_method_definition(
        SPAN,
        MethodDefinitionType::MethodDefinition,
        AVec::new_in(cx),
        key,
        function,
        kind,
        computed,
        is_static,
        false,
        false,
        None,
        cx,
    )
}

fn descriptor_parts<'a>(
    object: ObjectExpression<'a>,
) -> (Option<Expression<'a>>, [Option<Expression<'a>>; 3]) {
    let mut key = None;
    let mut parts = [None, None, None];
    for property in object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let slot = slot_of(&property);
        let value = property.unbox().value;
        match slot {
            Some(Slot::Key) => key = Some(value),
            Some(Slot::Value) => parts[0] = Some(value),
            Some(Slot::Get) => parts[1] = Some(value),
            Some(Slot::Set) => parts[2] = Some(value),
            None => {}
        }
    }
    (key, parts)
}

pub fn build<'a>(
    array: ArrayExpression<'a>,
    is_static: bool,
    body: &mut AVec<'a, ClassElement<'a>>,
    cx: &Cx<'a, '_>,
) {
    const KINDS: [MethodDefinitionKind; 3] = [
        MethodDefinitionKind::Method,
        MethodDefinitionKind::Get,
        MethodDefinitionKind::Set,
    ];
    for element in array.elements {
        let ArrayExpressionElement::ObjectExpression(object) = element else {
            continue;
        };
        let (Some(key), parts) = descriptor_parts(object.unbox()) else {
            continue;
        };
        let present = parts.iter().filter(|part| part.is_some()).count();
        let mut key = Some(key);
        for (kind, part) in KINDS.into_iter().zip(parts) {
            let Some(function) = part.and_then(function_of) else {
                continue;
            };
            let this_key = if present > 1 && kind == MethodDefinitionKind::Get {
                key.as_ref().map(|key| key.clone_in(cx.allocator()))
            } else {
                key.take()
            };
            if let Some(this_key) = this_key {
                body.push(method(this_key, function, kind, is_static, cx));
            }
        }
    }
}
