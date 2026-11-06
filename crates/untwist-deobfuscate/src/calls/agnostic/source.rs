use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};

use super::bind::Bindings;

const STANDARD_NAMES: [&str; 61] = [
    "AggregateError",
    "Array",
    "ArrayBuffer",
    "Atomics",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Boolean",
    "DataView",
    "Date",
    "Error",
    "EvalError",
    "FinalizationRegistry",
    "Float16Array",
    "Float32Array",
    "Float64Array",
    "Infinity",
    "Int16Array",
    "Int32Array",
    "Int8Array",
    "Intl",
    "Iterator",
    "JSON",
    "Map",
    "Math",
    "NaN",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "Reflect",
    "RegExp",
    "Set",
    "SharedArrayBuffer",
    "String",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "URIError",
    "Uint16Array",
    "Uint32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "WeakMap",
    "WeakRef",
    "WeakSet",
    "console",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "undefined",
    "unescape",
    "arguments",
];

pub(super) const SOURCE_KEYS: [&str; 1] = ["prepareStackTrace"];

pub(super) fn is_ambient_intrinsic(name: &str) -> bool {
    name != "arguments" && STANDARD_NAMES.contains(&name)
}

#[derive(Clone, Copy)]
pub(super) struct Frame {
    this_global: bool,
    strict: bool,
    in_function: bool,
}

pub(super) struct Sources {
    frame: Frame,
    found: bool,
    #[cfg(feature = "refusal-spans")]
    first: Option<(Span, &'static str)>,
}

impl Sources {
    pub(super) fn new(program: &Program<'_>) -> Self {
        let frame = Frame {
            this_global: !untwist_core::semantic::has_module_syntax(program),
            strict: untwist_core::semantic::program_is_strict(program),
            in_function: false,
        };
        Self {
            frame,
            found: false,
            #[cfg(feature = "refusal-spans")]
            first: None,
        }
    }

    pub(super) fn enter_function(&mut self, function: &Function<'_>) -> Frame {
        let saved = self.frame;
        let strict = saved.strict || function.has_use_strict_directive();
        self.frame = Frame {
            this_global: !strict,
            strict,
            in_function: true,
        };
        saved
    }

    pub(super) fn enter_bound_method(&mut self, function: &Function<'_>) -> Frame {
        let saved = self.enter_function(function);
        self.frame.this_global = false;
        saved
    }

    pub(super) fn enter_class(&mut self, own_this: bool) -> Frame {
        let saved = self.frame;
        self.frame = Frame {
            this_global: !own_this && saved.this_global,
            strict: true,
            in_function: saved.in_function,
        };
        saved
    }

    pub(super) fn leave(&mut self, saved: Frame) {
        self.frame = saved;
    }

    pub(super) fn found(&self) -> bool {
        self.found
    }

    #[cfg(feature = "refusal-spans")]
    pub(super) fn first(&self) -> Option<(Span, &'static str)> {
        self.first
    }

    pub(super) fn note_this(&mut self, span: Span) {
        if self.frame.this_global {
            self.note(span, "a this that may be the global object");
        }
    }

    pub(super) fn note(&mut self, span: Span, what: &'static str) {
        #[cfg(feature = "refusal-spans")]
        if self.first.is_none() {
            self.first = Some((span, what));
        }
        #[cfg(not(feature = "refusal-spans"))]
        let _ = (span, what);
        self.found = true;
    }

    fn standard(&self, name: &str) -> bool {
        STANDARD_NAMES.contains(&name) && (name != "arguments" || self.frame.in_function)
    }
}

fn source_key(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::StaticIdentifier(name) => SOURCE_KEYS.contains(&name.name.as_str()),
        PropertyKey::StringLiteral(text) => SOURCE_KEYS.iter().any(|key| text.value == *key),
        _ => false,
    }
}

impl<'a> Bindings<'_, 'a> {
    pub(super) fn scan_identifier(&mut self, id: &IdentifierReference<'a>) {
        if !self.sources.found()
            && self.reference_symbol(id).is_none()
            && !self.sources.standard(&id.name)
        {
            self.sources
                .note(id.span, "a free name that may be a host global");
        }
    }

    pub(super) fn scan_member(&mut self, member: &MemberExpression<'a>) {
        if self.sources.found() {
            return;
        }
        let keyed = match member {
            MemberExpression::StaticMemberExpression(m) => {
                SOURCE_KEYS.contains(&m.property.name.as_str())
            }
            MemberExpression::ComputedMemberExpression(m) => matches!(
                m.expression.without_parentheses(),
                Expression::StringLiteral(text) if SOURCE_KEYS.iter().any(|key| text.value == *key)
            ),
            MemberExpression::PrivateFieldExpression(_) => false,
        };
        if keyed {
            self.sources.note(member.span(), "a prepareStackTrace key");
        }
        if let Expression::Identifier(id) = member.object().without_parentheses() {
            self.scan_identifier(id);
        }
    }

    pub(super) fn scan_pattern_key(&mut self, key: &PropertyKey<'a>) {
        if source_key(key) {
            self.sources.note(key.span(), "a prepareStackTrace key");
        }
    }

    pub(super) fn scan_delete(&mut self, unary: &UnaryExpression<'a>) {
        let deleted = matches!(unary.argument.without_parentheses(),
            Expression::Identifier(id) if self.reference_symbol(id).is_none());
        if deleted {
            self.sources.note(unary.span, "a deleted free name");
        }
    }
}
