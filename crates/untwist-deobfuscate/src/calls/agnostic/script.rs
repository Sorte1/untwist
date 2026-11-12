use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};

use super::roles::{
    Text, KEY_DEPTH, SCRIPT_CREATORS, SCRIPT_SOURCE, SCRIPT_TEXT_STORES, TEXT_NODE_CALLS,
};
use super::sites::{expr_member, span_key, Sites};
use super::AKey;

const CODE_URLS: [&str; 3] = ["data:", "javascript:", "blob:"];
const TEXT_NODE: &str = "Text";

pub(super) struct ScriptSink {
    span: Span,
    writes: bool,
    reaches: bool,
}

fn tag_arguments<'k, 'a>(key: &AKey<'a>, arguments: &'k [Argument<'a>]) -> &'k [Argument<'a>] {
    let (from, to) = match key {
        AKey::Named(name) if name == "createElement" => (0, 1),
        AKey::Named(name) if name == "createElementNS" => (1, 2),
        _ => (0, 2),
    };
    let len = arguments.len();
    &arguments[from.min(len)..to.min(len)]
}

fn names_script(tag: &str) -> bool {
    let tag = tag.trim().to_ascii_lowercase();
    tag == "script" || tag.ends_with(":script")
}

fn loads_code(url: &str) -> bool {
    let url = url.trim_start().to_ascii_lowercase();
    CODE_URLS.iter().any(|scheme| url.starts_with(scheme))
}

impl<'a> Sites<'_, 'a> {
    fn creates(&self, key: &AKey<'a>) -> bool {
        self.script_spells(key, &SCRIPT_CREATORS)
    }

    fn script_spells(&self, key: &AKey<'a>, names: &[&str]) -> bool {
        !matches!(key, AKey::Unknown) && self.spells(key, names)
    }

    fn tag_inert(&self, argument: &Argument<'a>) -> bool {
        argument.as_expression().is_some_and(
            |tag| matches!(self.key_class(tag, KEY_DEPTH), AKey::Named(tag) if !names_script(&tag)),
        )
    }

    pub(super) fn note_script_creator(&mut self, call: &CallExpression<'a>) {
        if self.script_creator {
            return;
        }
        let Some(member) = expr_member(&call.callee) else {
            return;
        };
        let keys = self.member_keys(member);
        self.script_creator = keys.iter().any(|key| {
            self.creates(key)
                && !tag_arguments(key, &call.arguments)
                    .iter()
                    .all(|tag| self.tag_inert(tag))
        });
    }

    pub(super) fn note_script_read(&mut self, keys: &[AKey<'a>]) {
        self.script_creator |= keys.iter().any(|key| self.creates(key));
    }

    fn push_script_text(&mut self, span: Span, text: Option<&Expression<'a>>) {
        let sink = match text {
            Some(text) => ScriptSink {
                span,
                writes: self.texts_unsafe(&[Some(text)], Text::Script),
                reaches: self.text_hits(span, text),
            },
            None => self.unknown_script(span),
        };
        self.push_script_sink(sink);
    }

    fn unknown_script(&self, span: Span) -> ScriptSink {
        ScriptSink {
            span,
            writes: true,
            reaches: self.text_watched(span),
        }
    }

    fn push_script_sink(&mut self, sink: ScriptSink) {
        if sink.writes || sink.reaches {
            self.script_sinks.push(sink);
        }
    }

    fn push_script_source(&mut self, span: Span, url: &Expression<'a>) {
        let inert = self
            .text_value(url, KEY_DEPTH)
            .is_some_and(|url| !loads_code(&url));
        if !inert {
            let sink = self.unknown_script(span);
            self.push_script_sink(sink);
        }
    }

    fn script_store(&self, member: &MemberExpression<'a>) -> (bool, bool) {
        let keys = self.member_keys(member);
        let text = keys
            .iter()
            .any(|key| self.script_spells(key, &SCRIPT_TEXT_STORES));
        let source = keys
            .iter()
            .any(|key| self.script_spells(key, &[SCRIPT_SOURCE]));
        (text, source)
    }

    pub(super) fn note_script_store(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator != AssignmentOperator::Assign {
            return;
        }
        let Some(member) = it.left.as_member_expression() else {
            return;
        };
        let (text, source) = self.script_store(member);
        if !text && !source {
            return;
        }
        self.noted_scripts.insert(span_key(member.span()));
        if text {
            self.push_script_text(member.span(), Some(&it.right));
        }
        if source {
            self.push_script_source(member.span(), &it.right);
        }
    }

    pub(super) fn check_script_target(&mut self, member: &MemberExpression<'a>) {
        if self.noted_scripts.remove(&span_key(member.span())) {
            return;
        }
        let (text, source) = self.script_store(member);
        if text || source {
            self.push_script_text(member.span(), None);
        }
    }

    fn push_text_arguments(&mut self, span: Span, arguments: &[Argument<'a>]) {
        for argument in arguments {
            self.push_script_text(span, argument.as_expression());
        }
    }

    pub(super) fn note_text_node_call(&mut self, call: &CallExpression<'a>) {
        let Some(member) = expr_member(&call.callee) else {
            return;
        };
        let keys = self.member_keys(member);
        if keys
            .iter()
            .any(|key| self.script_spells(key, &TEXT_NODE_CALLS))
        {
            self.push_text_arguments(call.span, &call.arguments);
        }
    }

    pub(super) fn note_text_node_new(&mut self, new: &NewExpression<'a>) {
        let named = match new.callee.without_parentheses() {
            Expression::Identifier(id) => id.name == TEXT_NODE && self.fresh.symbol(id).is_none(),
            other => expr_member(other).is_some_and(|member| {
                self.member_keys(member)
                    .iter()
                    .any(|key| self.script_spells(key, &[TEXT_NODE]))
            }),
        };
        if named {
            self.push_text_arguments(new.span, &new.arguments);
        }
    }

    pub(super) fn judge_script_sinks(&mut self) {
        let sinks = std::mem::take(&mut self.script_sinks);
        if !self.script_creator {
            return;
        }
        for sink in sinks {
            if sink.reaches {
                self.hit_text(sink.span);
            }
            if sink.writes {
                self.mark_source(sink.span, "script text");
                self.mark_judged_code(sink.span);
            }
        }
    }
}
