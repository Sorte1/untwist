mod annex_b;
mod code_modules;
mod decoded_name;
mod decoder;
mod definer_calls;
mod definers;
mod escape;
mod escape_scan;
mod foreign;
mod global_aliases;
mod intrinsics;
mod names;
mod numeric_keys;
mod pending;
mod property_reads;
mod proxy_objects;
mod rotator;
mod seal;
mod storage;
mod text_writes;
mod unsafe_bindings;
mod watch;
mod writes;
mod written;

pub use annex_b::refuse_annex_b_bindings;
pub use decoded_name::{encode_names, names_machinery, EncodedName};
pub use decoder::{decoder_refusal, used_names, UsedNames};
pub use escape::{
    build_escape_context, scan_escaping_roots_resolved, scan_pending, EscapeContext, EscapeScan,
};
pub(crate) use escape_scan::{
    literal_text_binds, literal_text_reaches, literal_text_runs, literal_text_writes,
    with_parsed_text,
};
pub use foreign::{ForeignUnits, Reliance};
pub use names::{
    names_a_token, redeclared, regex_name_text, scan_names, spelled_at, writes_outside,
    written_outside, NameScan, Write,
};
pub use pending::{resolve_pending_sites, PendingSite, PreScan};
pub use rotator::{rotator_refusal_at, LiteralBindings, RotatorSite};
pub use seal::ROTATOR_GLOBALS;
pub(crate) use watch::watch_texts;
pub use watch::{declared_text, reachable_by_name, TextWatch, Watched};
pub(crate) use writes::{key_of, object_operand_is_intrinsic};
pub use writes::{
    key_relevant, patch_refusal, prototype_root_ref, record_definer_patch, record_identifier_patch,
    record_member_patch, PatchKey, PatchSite, PatchVerdict, PendingPrototype, ProtoRoot,
    ProtoWrite, Prototype,
};
pub use written::WrittenKey;

use oxc_ast::ast::{Program, Statement, WithStatement};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub note: &'static str,
    pub span: Span,
    pub detail: Option<String>,
}

impl Refusal {
    pub fn at(note: &'static str, span: Span) -> Self {
        Self {
            note,
            span,
            detail: None,
        }
    }

    pub fn naming(note: &'static str, span: Span, detail: String) -> Self {
        Self {
            note,
            span,
            detail: Some(detail),
        }
    }

    pub fn message(&self) -> String {
        match &self.detail {
            Some(detail) => format!("{}: {detail}", self.note),
            None => self.note.to_string(),
        }
    }
}

pub const STANDARD_GLOBALS: &[&str] = &[
    "Object",
    "Function",
    "Array",
    "String",
    "Boolean",
    "Number",
    "Math",
    "JSON",
    "RegExp",
    "Symbol",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "URIError",
    "EvalError",
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
    "undefined",
    "NaN",
    "Infinity",
    "arguments",
];

pub const GLOBAL_OBJECTS: &[&str] = &["window", "self", "globalThis", "global", "frames"];

pub fn builtins_assumed_pristine() -> bool {
    !untwist_core::semantic::with_observes()
}

pub fn has_with_statement(scoping: &Scoping) -> bool {
    untwist_core::semantic::with_observes()
        && scoping
            .scope_descendants_from_root()
            .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
}

#[derive(Default)]
struct FirstWith {
    span: Option<Span>,
}

impl<'a> VisitJs<'a> for FirstWith {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.span.is_none() {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.span.get_or_insert(it.span);
    }
}

pub fn with_statement(program: &Program<'_>, scoping: &Scoping) -> Option<Span> {
    if !has_with_statement(scoping) {
        return None;
    }
    let mut first = FirstWith::default();
    first.visit_program(program);
    Some(first.span.unwrap_or_default())
}
