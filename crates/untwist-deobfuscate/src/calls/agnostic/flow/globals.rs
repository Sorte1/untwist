use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::js_text::js_text;

const GLOBAL_NAMES: [&str; 7] = [
    "window",
    "self",
    "globalThis",
    "global",
    "frames",
    "top",
    "parent",
];

const GLOBAL_KEYS: [&str; 10] = [
    "window",
    "self",
    "globalThis",
    "global",
    "frames",
    "top",
    "parent",
    "defaultView",
    "contentWindow",
    "opener",
];

const RUNNERS: [&str; 2] = ["eval", "Function"];

pub(super) const NATIVE_HOLDERS: [&str; 2] = ["console", "JSON"];

#[derive(Default)]
pub(super) struct Globals {
    pub(super) writes: FxHashSet<String>,
    pub(super) keys: FxHashSet<String>,
    pub(super) unknown_key: bool,
    pub(super) unknown_write: bool,
    pub(super) written_keys: FxHashSet<String>,
    pub(super) value: bool,
    pub(super) holder_values: FxHashSet<String>,
    pub(super) with: bool,
    pub(super) open_eval: bool,
    base_next: Option<u32>,
    closed_evals: FxHashSet<u32>,
}

fn static_key<'k>(member: &MemberExpression<'k>) -> Option<Cow<'k, str>> {
    match member {
        MemberExpression::StaticMemberExpression(m) => {
            Some(Cow::Borrowed(m.property.name.as_str()))
        }
        MemberExpression::ComputedMemberExpression(m) => match m.expression.without_parentheses() {
            Expression::StringLiteral(text) => Some(js_text(text.value)),
            Expression::NumericLiteral(num) => Some(Cow::Owned(num.value.to_string())),
            _ => None,
        },
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

impl Globals {
    pub(super) fn unresolved(scoping: &Scoping, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| scoping.get_reference(reference).symbol_id())
            .is_none()
    }

    fn global_object(scoping: &Scoping, object: &Expression<'_>, this_global: bool) -> bool {
        match object.without_parentheses() {
            Expression::Identifier(id) => {
                GLOBAL_NAMES.contains(&id.name.as_str()) && Self::unresolved(scoping, id)
            }
            Expression::ThisExpression(_) => this_global,
            other => other
                .as_member_expression()
                .and_then(static_key)
                .is_some_and(|key| GLOBAL_KEYS.contains(&key.as_ref())),
        }
    }

    fn global_base(&self, scoping: &Scoping, object: &Expression<'_>, this_global: bool) -> bool {
        match object.without_parentheses() {
            Expression::Identifier(id) => Self::unresolved(scoping, id),
            Expression::ThisExpression(_) => this_global,
            other => other
                .as_member_expression()
                .and_then(static_key)
                .is_some_and(|key| GLOBAL_KEYS.contains(&key.as_ref())),
        }
    }

    fn looked_up(scoping: &Scoping, object: &Expression<'_>) -> bool {
        match object {
            Expression::Identifier(id) => Self::unresolved(scoping, id),
            Expression::ThisExpression(_) => true,
            other => other
                .as_member_expression()
                .and_then(static_key)
                .is_some_and(|key| {
                    GLOBAL_KEYS.contains(&key.as_ref()) || NATIVE_HOLDERS.contains(&key.as_ref())
                }),
        }
    }

    pub(super) fn note_member(
        &mut self,
        scoping: &Scoping,
        member: &MemberExpression<'_>,
        this_global: bool,
    ) {
        let object = member.object().without_parentheses();
        self.base_next = Self::looked_up(scoping, object).then_some(object.span().start);
        if !Self::global_object(scoping, object, this_global) {
            return;
        }
        match static_key(member) {
            Some(key) => {
                if !self.keys.contains(key.as_ref()) {
                    self.keys.insert(key.into_owned());
                }
            }
            None => self.unknown_key = true,
        }
    }

    fn is_base(&mut self, start: u32) -> bool {
        self.base_next.take() == Some(start)
    }

    pub(super) fn note_member_value(&mut self, member: &MemberExpression<'_>) {
        let Some(key) = static_key(member) else {
            self.base_next = None;
            return;
        };
        let base = self.is_base(member.span().start);
        let global = GLOBAL_KEYS.contains(&key.as_ref());
        let holder = NATIVE_HOLDERS.contains(&key.as_ref());
        if !(global || holder) || base {
            return;
        }
        self.value |= global;
        if holder {
            self.holder_values.insert(key.into_owned());
        }
    }

    pub(super) fn note_closed_eval(&mut self, start: u32) {
        self.closed_evals.insert(start);
    }

    pub(super) fn note_identifier(&mut self, scoping: &Scoping, id: &IdentifierReference<'_>) {
        let base = self.is_base(id.span.start);
        if !Self::unresolved(scoping, id) {
            return;
        }
        let name = id.name.as_str();
        if RUNNERS.contains(&name) && !self.closed_evals.contains(&id.span.start) {
            self.value = true;
            self.open_eval |= name == "eval";
        }
        if base {
            return;
        }
        if GLOBAL_NAMES.contains(&name) {
            self.value = true;
        }
        if NATIVE_HOLDERS.contains(&name) {
            self.holder_values.insert(name.to_string());
        }
    }

    pub(super) fn note_this(&mut self, span: oxc_span::Span, this_global: bool) {
        let base = self.is_base(span.start);
        if this_global && !base {
            self.value = true;
        }
    }

    pub(super) fn note_target(
        &mut self,
        scoping: &Scoping,
        target: &SimpleAssignmentTarget<'_>,
        this_global: bool,
    ) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = target {
            self.note_name_write(scoping, id);
            return;
        }
        if let Some(member) = target.as_member_expression() {
            self.note_member_write(scoping, member, this_global);
        }
    }

    pub(super) fn note_name_write(&mut self, scoping: &Scoping, id: &IdentifierReference<'_>) {
        if Self::unresolved(scoping, id) {
            self.writes.insert(id.name.to_string());
        }
    }

    pub(super) fn note_member_write(
        &mut self,
        scoping: &Scoping,
        member: &MemberExpression<'_>,
        this_global: bool,
    ) {
        let global = self.global_base(scoping, member.object(), this_global);
        match static_key(member) {
            Some(name) => {
                if global {
                    self.writes.insert(name.to_string());
                }
                self.written_keys.insert(name.into_owned());
            }
            None => {
                if global {
                    self.unknown_write = true;
                }
            }
        }
    }

    pub(super) fn may_write(&self, name: &str) -> bool {
        self.value || self.with || self.unknown_write || self.writes.contains(name)
    }

    pub(super) fn may_reach(&self, name: &str) -> bool {
        self.may_write(name) || self.unknown_key || self.keys.contains(name)
    }
}
