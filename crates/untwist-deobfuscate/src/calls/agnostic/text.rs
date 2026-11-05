use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};

use super::roles::*;
use super::sites::*;
use super::AKey;

const FUNCTION_RUNNERS: [&str; 4] = ["Function", "constructor", "compileFunction", "_compile"];

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
        .as_str()
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

    pub(super) fn text_writes(&self, text: &str, function: bool) -> bool {
        let body = if function {
            format!("(function () {{\n{text}\n}});")
        } else {
            text.to_string()
        };
        crate::proof::literal_text_writes(&body)
            || crate::proof::literal_text_runs(&body)
            || (!function && crate::proof::literal_text_binds(&body))
    }

    pub(super) fn check_text(
        &mut self,
        call_span: Span,
        arguments: &[Argument<'a>],
        function: bool,
    ) {
        self.mark_source(call_span, "code from text");
        if self.text_unsafe(arguments, function) {
            self.text_judged = true;
            self.mark_code(call_span);
            self.text_judged = false;
        }
    }

    pub(super) fn check_template(
        &mut self,
        span: Span,
        quasi: &TemplateLiteral<'a>,
        function: bool,
    ) {
        self.mark_source(span, "code from a template");
        let text = quasi
            .quasis
            .first()
            .and_then(|element| element.value.cooked)
            .filter(|_| quasi.expressions.is_empty());
        self.reach_template_text(span, text.as_deref());
        if text.is_none_or(|text| self.text_writes(&text, function)) {
            self.text_judged = true;
            self.mark_code(span);
            self.text_judged = false;
        }
    }

    pub(super) fn text_unsafe(&self, arguments: &[Argument<'a>], function: bool) -> bool {
        let texts: Vec<Option<&Expression<'a>>> =
            arguments.iter().map(Argument::as_expression).collect();
        self.texts_unsafe(&texts, function)
    }

    fn texts_unsafe(&self, texts: &[Option<&Expression<'a>>], function: bool) -> bool {
        let first_fn = texts
            .first()
            .copied()
            .flatten()
            .is_some_and(|first| self.fresh.is_user_function(first));
        if first_fn || texts.is_empty() {
            return false;
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
                        if !self.text_writes(&text, function) =>
                    {
                        continue
                    }
                    Some(untwist_eval::fold::Const::Str(_)) | None => return true,
                    Some(_) => continue,
                }
            }
            match self.key_class(inner, KEY_DEPTH) {
                AKey::Named(text) if !self.text_writes(&text, function) => {}
                _ => return true,
            }
        }
        false
    }

    pub(super) fn check_forwarded_runner(
        &mut self,
        call: &CallExpression<'a>,
        function: &Expression<'a>,
        forward: Forward,
    ) {
        let Some(text_function) = self.runner_callee(function) else {
            return;
        };
        self.mark_source(call.span, "a forwarded runner");
        self.reach_forwarded(call, forward);
        let call_texts = forward.call.then(|| {
            let texts: Vec<Option<&Expression<'a>>> = call
                .arguments
                .iter()
                .skip(1)
                .map(Argument::as_expression)
                .collect();
            self.texts_unsafe(&texts, text_function)
        });
        let listed = if forward.reflect { 2 } else { 1 };
        let list_texts = (forward.apply || forward.reflect)
            .then(|| self.listed_texts_unsafe(call.arguments.get(listed), text_function));
        if call_texts.unwrap_or(false) || list_texts.unwrap_or(false) {
            self.mark_text_code(call.span);
        }
    }

    fn listed_texts_unsafe(&self, list: Option<&Argument<'a>>, function: bool) -> bool {
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
        self.texts_unsafe(&texts, function)
    }

    pub(super) fn runner_callee(&self, callee: &Expression<'a>) -> Option<bool> {
        match callee.without_parentheses() {
            Expression::Identifier(id)
                if RUNNERS.contains(&id.name.as_str()) && self.fresh.symbol(id).is_none() =>
            {
                Some(id.name == "Function")
            }
            other => {
                let member = other.as_member_expression()?;
                if self.table_element(member) {
                    return None;
                }
                let keys = self.member_keys(member);
                keys.iter().any(|key| origin_of(key).runner).then(|| {
                    keys.iter().all(|key| {
                        matches!(key, AKey::Named(name) if FUNCTION_RUNNERS.contains(&name.as_ref()))
                    })
                })
            }
        }
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
        if self.runner_callee(&inner.callee) != Some(true) {
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
        let Some(text_function) = self.runner_callee(arg) else {
            return;
        };
        self.mark_source(call.span, "a proxied runner");
        self.direct
            .insert(span_key(arg.without_parentheses().span()));
        let inner: &'a CallExpression<'a> = self.alloc(inner);
        let mut flagged = false;
        let bound = self.with_params(function, call, || {
            flagged = self.text_unsafe(&inner.arguments, text_function);
        });
        if bound.is_none() || flagged {
            self.mark_text_code(call.span);
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
