use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::readonly::{scan_object_uses, ObjectUses};
use untwist_core::semantic::builtins::{intact, Reliance};
use untwist_core::semantic::is_constant_binding;
use untwist_core::semantic::{has_module_syntax, AnnexBWrites};

use std::sync::OnceLock;

const PROTO_KEY: &str = "__proto__";

pub(super) const OBJECT_PROTOTYPE_KEYS: [&str; 12] = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
    PROTO_KEY,
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

const FOLDED_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];

fn is_plain_own_key(key: &str) -> bool {
    !OBJECT_PROTOTYPE_KEYS.contains(&key)
}

fn static_key<'p>(property: &'p ObjectProperty<'p>) -> Option<&'p str> {
    if property.method || property.computed || property.kind != PropertyKind::Init {
        return None;
    }
    let key = match &property.key {
        PropertyKey::StaticIdentifier(id) => id.name.as_str(),
        PropertyKey::StringLiteral(text) => text.value.as_str()?,
        _ => return None,
    };
    is_plain_own_key(key).then_some(key)
}

pub(super) fn static_properties<'p>(
    object: &'p ObjectExpression<'p>,
) -> Option<Vec<(&'p str, &'p Expression<'p>)>> {
    object
        .properties
        .iter()
        .map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => {
                Some((static_key(property)?, &property.value))
            }
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
        .collect()
}

pub(super) fn static_property<'p>(
    object: &'p ObjectExpression<'p>,
    name: &str,
) -> Option<&'p Expression<'p>> {
    static_properties(object)?
        .into_iter()
        .rev()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

struct ReceiverUses {
    found: bool,
}

impl<'a> VisitJs<'a> for ReceiverUses {
    fn visit_this_expression(&mut self, _: &ThisExpression) {
        self.found = true;
    }

    fn visit_super(&mut self, _: &Super) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if matches!(it.name.as_str(), "arguments" | "eval") {
            self.found = true;
        }
    }
}

pub(super) fn reaches_its_receiver(object: &ObjectExpression<'_>) -> bool {
    let mut scan = ReceiverUses { found: false };
    scan.visit_object_expression(object);
    scan.found
}

fn is_written_function(value: &Expression<'_>) -> bool {
    matches!(
        value.without_parentheses(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

struct Callees<'s> {
    scoping: &'s Scoping,
    receiver: SymbolId,
    spans: FxHashSet<Span>,
}

impl Callees<'_> {
    fn record(&mut self, callee: &Expression<'_>) {
        let callee = callee.without_parentheses();
        let Some(member) = callee.as_member_expression() else {
            return;
        };
        let Expression::Identifier(object) = member.object().without_parentheses() else {
            return;
        };
        let symbol = object
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol == Some(self.receiver) {
            self.spans.insert(callee.span());
        }
    }
}

impl<'a> VisitJs<'a> for Callees<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.record(&it.callee);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.record(&it.callee);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.record(&it.tag);
        walk_js::walk_tagged_template_expression(self, it);
    }
}

fn callee_spans(program: &Program<'_>, scoping: &Scoping, receiver: SymbolId) -> FxHashSet<Span> {
    let mut scan = Callees {
        scoping,
        receiver,
        spans: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.spans
}

fn reads_are_proven(
    object: &ObjectExpression<'_>,
    uses: &ObjectUses,
    callees: &FxHashSet<Span>,
    builtins_unpatched: bool,
) -> bool {
    let Some(keys) = static_properties(object) else {
        return false;
    };
    let value_of = |name: &str| {
        keys.iter()
            .rev()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
    };
    uses.reads.iter().all(|read| {
        let name = read.name.as_str();
        match value_of(name) {
            Some(value) => !callees.contains(&read.member_span) || is_written_function(value),
            None => builtins_unpatched && is_plain_own_key(name),
        }
    })
}

pub(super) fn proven_readonly_uses(
    program: &Program<'_>,
    object: &ObjectExpression<'_>,
    scoping: &Scoping,
    receiver: SymbolId,
    builtins_unpatched: bool,
) -> Option<ObjectUses> {
    if reaches_its_receiver(object) || !is_constant_binding(scoping, receiver) {
        return None;
    }
    let uses = scan_object_uses(program, scoping, receiver);
    if uses.reads.is_empty() {
        return None;
    }
    let callees = callee_spans(program, scoping, receiver);
    reads_are_proven(object, &uses, &callees, builtins_unpatched).then_some(uses)
}

pub(super) fn annex_b_written(program: &Program<'_>, scoping: &Scoping) -> Option<AnnexBWrites> {
    if crate::mutants::annex_b_writes_trusted() {
        return None;
    }
    let writes = AnnexBWrites::of(program, scoping);
    (!writes.is_empty()).then_some(writes)
}

struct BuiltinTouch<'s> {
    scoping: &'s Scoping,
    found: bool,
}

impl<'a> VisitJs<'a> for BuiltinTouch<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        match it {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::Identifier(_)
            | Expression::ParenthesizedExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::LogicalExpression(_)
            | Expression::ConditionalExpression(_)
            | Expression::SequenceExpression(_) => walk_js::walk_expression(self, it),
            _ => self.found = true,
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let unresolved = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_none();
        if unresolved && !FOLDED_GLOBALS.contains(&it.name.as_str()) {
            self.found = true;
        }
    }
}

pub(super) fn touches_builtins(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    let mut scan = BuiltinTouch {
        scoping,
        found: false,
    };
    scan.visit_expression(expr);
    scan.found
}

fn every_builtin() -> &'static Reliance {
    static RELIANCE: OnceLock<Reliance> = OnceLock::new();
    RELIANCE.get_or_init(|| Reliance {
        roots: untwist_eval::builtins::all_root_names(),
        members: &[],
    })
}

pub(super) fn builtins_proven_intact(program: &Program<'_>, scoping: &Scoping) -> bool {
    intact(
        program,
        scoping,
        has_module_syntax(program),
        every_builtin(),
    )
}
