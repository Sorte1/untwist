use oxc_ast::ast::*;
use oxc_span::GetSpan;

use super::roles::{origin_of, Text, MARKUP_CALLS, MARKUP_STORES};
use super::sites::{span_key, Sites};
use super::text::FUNCTION_RUNNERS;
use super::AKey;

impl<'a> Sites<'_, 'a> {
    pub(super) fn unbounded_text(&self) -> Text {
        if self.markup_unbounded {
            Text::FunctionOrMarkup
        } else {
            Text::Function
        }
    }

    pub(super) fn spells(&self, key: &AKey<'a>, names: &[&str]) -> bool {
        match key {
            AKey::Named(name) => names.contains(&name.as_ref()),
            AKey::Index => false,
            AKey::Pending(_) | AKey::Unknown => true,
            AKey::Filtered(filter) => names.iter().any(|name| self.filter_accepts(*filter, name)),
        }
    }

    pub(super) fn member_text(&self, keys: &[AKey<'a>]) -> Text {
        let function = keys.iter().all(
            |key| matches!(key, AKey::Named(name) if FUNCTION_RUNNERS.contains(&name.as_ref())),
        );
        if function {
            return Text::Function;
        }
        let markup = keys.iter().any(|key| self.spells(key, &MARKUP_CALLS));
        let script = keys.iter().any(|key| {
            origin_of(key).runner
                && !matches!(key, AKey::Named(name) if MARKUP_CALLS.contains(&name.as_ref()))
        });
        match (script, markup) {
            (true, true) => Text::ScriptOrMarkup,
            (false, true) => Text::Markup,
            _ => Text::Script,
        }
    }

    fn markup_store(&self, member: &MemberExpression<'a>) -> bool {
        self.member_keys(member)
            .iter()
            .any(|key| !matches!(key, AKey::Unknown) && self.spells(key, &MARKUP_STORES))
    }

    pub(super) fn note_markup_store(&mut self, it: &AssignmentExpression<'a>) {
        let Some(member) = it.left.as_member_expression() else {
            return;
        };
        if self.markup_store(member) && !self.texts_unsafe(&[Some(&it.right)], Text::Markup) {
            self.inert_markup.insert(span_key(member.span()));
        }
    }

    pub(super) fn check_markup_target(&mut self, member: &MemberExpression<'a>) {
        if self.markup_store(member) && !self.inert_markup.remove(&span_key(member.span())) {
            self.mark_source(member.span(), "markup from text");
            self.mark_opaque_text(member.span());
        }
    }

    pub(super) fn check_markup_property(&mut self, key: &AKey<'a>, property: &ObjectProperty<'a>) {
        let named = matches!(key, AKey::Named(name) if MARKUP_STORES.contains(&name.as_ref()));
        if named && self.texts_unsafe(&[Some(&property.value)], Text::Markup) {
            self.mark_source(property.span, "markup from text");
            self.mark_opaque_text(property.span);
        }
    }
}
