use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use untwist_core::ast_util::js_text::js_text;

use super::roles::*;
use super::sites::*;
use super::AKey;
use crate::value_flow::{flow_rule_disabled, STORAGE_PROXY};

pub(super) const FUNCTION_RUNNERS: [&str; 4] =
    ["Function", "constructor", "compileFunction", "_compile"];

pub(super) fn closed_runner_read<'a>(
    call: &CallExpression<'a>,
    scoping: &Scoping,
) -> Option<&'a str> {
    if call.optional || !call.callee.is_specific_id("eval") {
        return None;
    }
    let [Argument::StringLiteral(literal)] = call.arguments.as_slice() else {
        return None;
    };
    let name = literal
        .value
        .as_str()?
        .trim()
        .trim_end_matches(';')
        .trim_end();
    if !RUNNERS.contains(&name) {
        return None;
    }
    let names = untwist_core::semantic::closed_eval_names(call)?;
    if names.len() != 1 || names[0] != name {
        return None;
    }
    let bound = scoping.symbol_ids().any(|symbol| {
        let bound_name = scoping.symbol_name(symbol);
        bound_name == name || bound_name == "eval"
    });
    (!bound).then_some(name)
}

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn mark_closed_runner_read(&mut self, span: Span) {
        self.mark_source(span, "a closed eval of a runner");
        let has_sink = self.flow.sink_of(span).is_some();
        let origin = Origin {
            runner: true,
            ..Origin::default()
        };
        self.mark_origin_sink(origin, span, "closed eval of a runner", has_sink);
    }

    pub(super) fn text_writes(&self, text: &str, kind: Text) -> bool {
        if kind.markup() && text.contains('<') {
            return true;
        }
        if kind == Text::Markup {
            return false;
        }
        let function = kind.function();
        let body = if function {
            format!("(function () {{\n{text}\n}});")
        } else {
            text.to_string()
        };
        crate::proof::literal_text_writes(&body)
            || crate::proof::literal_text_runs(&body)
            || (!function && crate::proof::literal_text_binds(&body))
    }

    pub(super) fn check_text(&mut self, call_span: Span, arguments: &[Argument<'a>], kind: Text) {
        self.mark_source(call_span, "code from text");
        if !self.text_unsafe(arguments, kind) {
            return;
        }
        let plain = kind
            .plain()
            .is_some_and(|plain| self.text_unsafe(arguments, plain));
        self.mark_judged(call_span, kind, plain);
    }

    fn mark_judged(&mut self, span: Span, kind: Text, plain: bool) {
        if kind.markup() && !plain {
            self.mark_opaque_text(span);
            return;
        }
        self.mark_judged_code(span);
    }

    pub(super) fn mark_judged_code(&mut self, span: Span) {
        self.text_judged = true;
        self.mark_code(span);
        self.text_judged = false;
    }

    fn mark_forwarded(&mut self, span: Span, kind: Text, plain: bool) {
        if kind.markup() && !plain {
            self.mark_opaque_text(span);
        } else {
            self.mark_text_code(span);
        }
    }

    pub(super) fn check_template(&mut self, span: Span, quasi: &TemplateLiteral<'a>, kind: Text) {
        self.mark_source(span, "code from a template");
        let text = quasi
            .quasis
            .first()
            .and_then(|element| element.value.cooked)
            .filter(|_| quasi.expressions.is_empty())
            .map(js_text);
        self.reach_template_text(span, text.as_deref());
        let writes = |kind: Text| {
            text.as_deref()
                .is_none_or(|text| self.text_writes(text, kind))
        };
        if writes(kind) {
            let plain = kind.plain().is_some_and(writes);
            self.mark_judged(span, kind, plain);
        }
    }

    pub(super) fn text_unsafe(&self, arguments: &[Argument<'a>], kind: Text) -> bool {
        let texts: Vec<Option<&Expression<'a>>> =
            arguments.iter().map(Argument::as_expression).collect();
        self.texts_unsafe(&texts, kind)
    }

    pub(super) fn texts_unsafe(&self, texts: &[Option<&Expression<'a>>], kind: Text) -> bool {
        let first_fn = texts
            .first()
            .copied()
            .flatten()
            .is_some_and(|first| self.fresh.is_user_function(first));
        let function = kind.function();
        if texts.is_empty() || (first_fn && !function) {
            return false;
        }
        if function && self.built_function_unsafe(texts) {
            return true;
        }
        for text in texts {
            let Some(expr) = text else {
                return true;
            };
            let inner = expr.without_parentheses();
            if matches!(
                inner,
                Expression::NumericLiteral(_)
                    | Expression::BooleanLiteral(_)
                    | Expression::NullLiteral(_)
            ) || self.fresh.is_numeric(inner)
            {
                continue;
            }
            if self.native_chain(inner) {
                match untwist_eval::fold::fold_literal_chain(inner) {
                    Some(untwist_eval::fold::Const::Str(text))
                        if !self.text_writes(&text, kind) =>
                    {
                        continue
                    }
                    Some(untwist_eval::fold::Const::Str(_)) | None => return true,
                    Some(_) => continue,
                }
            }
            match self.key_class(inner, KEY_DEPTH) {
                AKey::Named(text) if !self.text_writes(&text, kind) => {}
                _ => return true,
            }
        }
        false
    }

    fn built_function_unsafe(&self, texts: &[Option<&Expression<'a>>]) -> bool {
        let resolved = texts
            .iter()
            .map(|text| text.and_then(|expr| self.argument_text(expr.without_parentheses())));
        crate::proof::function_source(resolved)
            .is_none_or(|source| self.text_writes(&source, Text::Script))
    }

    fn argument_text(&self, inner: &Expression<'a>) -> Option<String> {
        if self.native_chain(inner) {
            return match untwist_eval::fold::fold_literal_chain(inner) {
                Some(untwist_eval::fold::Const::Str(text)) => Some(text.to_string()),
                _ => None,
            };
        }
        match self.key_class(inner, KEY_DEPTH) {
            AKey::Named(text) => Some(text.into_owned()),
            _ => None,
        }
    }

    pub(super) fn check_forwarded_runner(
        &mut self,
        call: &CallExpression<'a>,
        function: &Expression<'a>,
        forward: Forward,
    ) {
        let Some(kind) = self.runner_callee(function) else {
            return;
        };
        self.mark_source(call.span, "a forwarded runner");
        self.reach_forwarded(call, forward);
        if !self.forwarded_unsafe(call, forward, kind) {
            return;
        }
        let plain = kind
            .plain()
            .is_some_and(|plain| self.forwarded_unsafe(call, forward, plain));
        self.mark_forwarded(call.span, kind, plain);
    }

    fn forwarded_unsafe(&self, call: &CallExpression<'a>, forward: Forward, kind: Text) -> bool {
        let call_texts = forward.call.then(|| {
            let texts: Vec<Option<&Expression<'a>>> = call
                .arguments
                .iter()
                .skip(1)
                .map(Argument::as_expression)
                .collect();
            self.texts_unsafe(&texts, kind)
        });
        let listed = if forward.reflect { 2 } else { 1 };
        let list_texts = (forward.apply || forward.reflect)
            .then(|| self.listed_texts_unsafe(call.arguments.get(listed), kind));
        call_texts.unwrap_or(false) || list_texts.unwrap_or(false)
    }

    fn listed_texts_unsafe(&self, list: Option<&Argument<'a>>, kind: Text) -> bool {
        let Some(list) = list else {
            return false;
        };
        let Some(Expression::ArrayExpression(array)) =
            list.as_expression().map(Expression::without_parentheses)
        else {
            return true;
        };
        let texts: Vec<Option<&Expression<'a>>> = array
            .elements
            .iter()
            .map(ArrayExpressionElement::as_expression)
            .collect();
        self.texts_unsafe(&texts, kind)
    }

    pub(super) fn runner_callee(&self, callee: &Expression<'a>) -> Option<Text> {
        match callee.without_parentheses() {
            Expression::Identifier(id)
                if RUNNERS.contains(&id.name.as_str()) && self.fresh.symbol(id).is_none() =>
            {
                Some(if id.name == "Function" {
                    Text::Function
                } else {
                    Text::Script
                })
            }
            other => {
                let member = other.as_member_expression()?;
                if self.table_element(member) {
                    return None;
                }
                let keys = self.member_keys(member);
                keys.iter()
                    .any(|key| origin_of(key).runner)
                    .then(|| self.member_text(&keys))
            }
        }
    }

    pub(super) fn table_dispatch(&self, callee: &Expression<'a>) -> bool {
        let Some(member) = callee.without_parentheses().as_member_expression() else {
            return false;
        };
        if self.table_element(member) {
            return true;
        }
        let forwards =
            matches!(self.member_keys(member).as_slice(), [AKey::Named(name)] if name == "call");
        forwards
            && member
                .object()
                .without_parentheses()
                .as_member_expression()
                .is_some_and(|element| self.table_element(element))
    }

    fn table_element(&self, member: &MemberExpression<'a>) -> bool {
        let MemberExpression::ComputedMemberExpression(element) = member else {
            return false;
        };
        matches!(element.object.without_parentheses(), Expression::Identifier(id)
            if self.fresh.symbol(id).is_some_and(|symbol| self.fresh.bindings.is_table(symbol)))
    }

    pub(super) fn note_creation(&mut self, call: &CallExpression<'a>) {
        let Some(member) = expr_member(&call.callee) else {
            return;
        };
        let Expression::CallExpression(inner) = member.object().without_parentheses() else {
            return;
        };
        if self.runner_callee(&inner.callee) != Some(Text::Function) {
            return;
        }
        let keys = self.member_keys(member);
        let safe = !keys.is_empty()
            && keys.iter().all(|key| {
                matches!(key, AKey::Named(name) if !matches!(name.as_ref(), "call" | "apply" | "bind"))
            });
        if safe {
            self.creation_only.insert(span_key(inner.span));
            self.creation_keys.extend(keys);
        }
    }

    pub(super) fn forwarded_runner_index(&self, call: &CallExpression<'a>) -> Option<usize> {
        let (function, body) = self.proxy(self.alloc(call))?;
        let Expression::CallExpression(inner) = body.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(callee) = inner.callee.without_parentheses() else {
            return None;
        };
        let symbol = self.fresh.symbol(callee)?;
        let index = function.params.items.iter().position(|param| {
            matches!(&param.pattern, BindingPattern::BindingIdentifier(binding)
                if binding.symbol_id.get() == Some(symbol))
        })?;
        let arg = call.arguments.get(index)?.as_expression()?;
        self.runner_callee(arg)?;
        Some(index)
    }

    pub(super) fn proxied_runner(&mut self, call: &CallExpression<'a>) {
        let Some((function, body)) = self.proxy(self.alloc(call)) else {
            return;
        };
        let Expression::CallExpression(inner) = body.without_parentheses() else {
            return;
        };
        let Expression::Identifier(callee) = inner.callee.without_parentheses() else {
            return;
        };
        let Some(arg) = self.substituted_param(function, call, callee) else {
            return;
        };
        let Some(kind) = self.runner_callee(arg) else {
            return;
        };
        self.mark_source(call.span, "a proxied runner");
        self.direct
            .insert(span_key(arg.without_parentheses().span()));
        let inner: &'a CallExpression<'a> = self.alloc(inner);
        let mut flagged = false;
        let mut plain = false;
        let bound = self.with_params(function, call, || {
            flagged = self.text_unsafe(&inner.arguments, kind);
            plain = kind
                .plain()
                .is_some_and(|plain| self.text_unsafe(&inner.arguments, plain));
        });
        let judged = !flow_rule_disabled(STORAGE_PROXY);
        if judged && (bound.is_none() || flagged) {
            self.mark_forwarded(
                call.span,
                kind,
                plain || bound.is_none() && kind.plain().is_some(),
            );
        }
    }

    fn takes_selectors(&self, callee: &Expression<'a>) -> bool {
        let Some(member) = callee.without_parentheses().as_member_expression() else {
            return false;
        };
        let keys = self.member_keys(member);
        !keys.is_empty()
            && keys.iter().all(
                |key| matches!(key, AKey::Named(name) if SELECTOR_TAKERS.contains(&name.as_ref())),
            )
    }

    pub(super) fn literal_texts(&mut self, call: &'a CallExpression<'a>) {
        if self.in_machinery(call.span)
            || self.out.code_text.is_some()
            || self.takes_selectors(&call.callee)
        {
            return;
        }
        let mut texts = Vec::new();
        for argument in &call.arguments {
            let Some(expr) = argument.as_expression() else {
                continue;
            };
            match expr.without_parentheses() {
                Expression::ObjectExpression(object) => {
                    for property in &object.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = property {
                            texts.push(self.key_class(&p.value, KEY_DEPTH));
                        }
                    }
                }
                other => texts.push(self.key_class(other, KEY_DEPTH)),
            }
        }
        let reaches = texts.iter().any(|text| match text {
            AKey::Named(text) => {
                CODE_HINTS.iter().any(|hint| text.contains(hint))
                    && crate::proof::literal_text_reaches(text)
            }
            _ => false,
        });
        if reaches {
            self.mark_code(call.span);
        }
    }
}
