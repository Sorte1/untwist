use oxc_ast::ast::*;
use oxc_span::Span;
use std::borrow::Cow;

use super::keys::Keys;
use super::roles::*;
use super::sites::*;
use super::{AKey, Agnostic};
use crate::proof::ProtoRoot;

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn keys_of(&self, expr: &Expression<'a>, keys: &mut Vec<AKey<'a>>) {
        match expr.without_parentheses() {
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            keys.push(self.property_key(&p.key))
                        }
                        ObjectPropertyKind::SpreadProperty(_) => keys.push(AKey::Unknown),
                    }
                }
            }
            Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_) => keys.push(AKey::Index),
            Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_) => {}
            Expression::Identifier(id)
                if id.name == "undefined" && self.fresh.symbol(id).is_none() => {}
            other if self.fresh.is_numeric(other) || self.native_chain(other) => {
                keys.push(AKey::Index)
            }
            _ => keys.push(AKey::Unknown),
        }
    }

    pub(super) fn literal_scalar(&self, id: &IdentifierReference<'a>) -> bool {
        self.fresh.literal_init(id).is_some_and(|init| {
            !matches!(
                init.without_parentheses(),
                Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
            )
        }) || id.name == "undefined"
            || self.fresh.is_numeric_identifier(id)
    }

    pub(super) fn elem_keys(&self, elem: Elem<'_, 'a>) -> Keys<'a> {
        match elem {
            Elem::Hidden => Keys::One(AKey::Unknown),
            Elem::Value(expr) => self.key_set(expr),
        }
    }

    pub(super) fn may_bind(&self, call: &CallExpression<'a>) -> bool {
        if self.bind_unbounded {
            return true;
        }
        let Some(member) = expr_member(&call.callee) else {
            return false;
        };
        let names = |member: &MemberExpression<'a>, name: &str| {
            self.member_keys(member)
                .iter()
                .any(|key| matches!(key, AKey::Named(text) if text == name))
        };
        names(member, "bind")
            || ((names(member, "call") || names(member, "apply"))
                && expr_member(member.object()).is_some_and(|inner| names(inner, "bind")))
    }

    pub(super) fn may_apply(&self, call: &CallExpression<'a>) -> bool {
        if self.apply_unbounded {
            return true;
        }
        expr_member(&call.callee).is_some_and(|member| {
            self.member_keys(member)
                .iter()
                .any(|key| matches!(key, AKey::Named(text) if text == "apply"))
        })
    }

    pub(super) fn hidden_contents(&self, elem: Elem<'_, 'a>) -> bool {
        match elem {
            Elem::Hidden => true,
            Elem::Value(expr) => match expr.without_parentheses() {
                Expression::Identifier(id) => !self.literal_scalar(id),
                Expression::ArrayExpression(_) => false,
                _ => self.is_target(elem),
            },
        }
    }

    pub(super) fn may_be_callable(&self, elem: Elem<'_, 'a>) -> bool {
        match elem {
            Elem::Hidden => true,
            Elem::Value(expr) => match expr.without_parentheses() {
                Expression::StringLiteral(_)
                | Expression::NumericLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
                | Expression::TemplateLiteral(_)
                | Expression::ObjectExpression(_)
                | Expression::ArrayExpression(_)
                | Expression::UnaryExpression(_)
                | Expression::BinaryExpression(_) => false,
                Expression::Identifier(id) => !self.literal_scalar(id),
                other => !(self.native_chain(other) || self.fresh.is_numeric(other)),
            },
        }
    }

    pub(super) fn group(
        &self,
        elems: &[Elem<'_, 'a>],
        binds: bool,
        applies: bool,
        keys: &mut Vec<AKey<'a>>,
    ) {
        for (index, elem) in elems.iter().enumerate() {
            if applies && self.hidden_contents(*elem) {
                keys.push(AKey::Unknown);
            }
            if !self.is_target(*elem) {
                continue;
            }
            keys.extend([
                AKey::Named(Cow::Borrowed("__proto__")),
                AKey::Index,
                AKey::Named(Cow::Borrowed("length")),
            ]);
            match elems.get(index + 1) {
                Some(next) => keys.extend(self.elem_keys(*next)),
                None => keys.push(AKey::Named(Cow::Borrowed("undefined"))),
            }
            if index >= 2 {
                keys.extend(self.elem_keys(elems[index - 2]));
            }
            for later in &elems[index + 1..] {
                match later {
                    Elem::Value(expr) => self.keys_of(expr, keys),
                    Elem::Hidden => keys.push(AKey::Unknown),
                }
            }
            if binds && index + 1 == elems.len() {
                keys.push(AKey::Unknown);
            }
        }
    }

    pub(super) fn array_elems<'k>(&self, expr: &'k Expression<'a>) -> Option<Vec<Elem<'k, 'a>>> {
        let Expression::ArrayExpression(array) = expr.without_parentheses() else {
            return None;
        };
        let mut elems = Vec::with_capacity(array.elements.len());
        for element in &array.elements {
            if matches!(element, ArrayExpressionElement::SpreadElement(_)) {
                elems.extend([Elem::Hidden; SPREAD_WIDTH]);
                break;
            }
            elems.push(element.as_expression().map_or(Elem::Hidden, Elem::Value));
        }
        Some(elems)
    }

    pub(super) fn generic(&mut self, call: &'a CallExpression<'a>) {
        let receiver = expr_member(&call.callee).map(MemberExpression::object);
        let args = arg_elems(&call.arguments);
        let binds = self.may_bind(call);
        let applies = self.may_apply(call);
        let mut keys: Vec<AKey<'a>> = Vec::new();
        if let Some(receiver) = receiver {
            if self.is_target(Elem::Value(receiver)) {
                keys.extend([
                    AKey::Named(Cow::Borrowed("__proto__")),
                    AKey::Index,
                    AKey::Named(Cow::Borrowed("length")),
                ]);
                if args.len() >= 2 && self.may_be_callable(args[1]) {
                    keys.extend(self.elem_keys(args[0]));
                }
            }
            if let Some(elems) = self.array_elems(receiver) {
                self.group(&elems, false, false, &mut keys);
            }
        }
        self.group(&args, binds, applies, &mut keys);
        for argument in &call.arguments {
            if let Some(elems) = argument
                .as_expression()
                .and_then(|expr| self.array_elems(expr))
            {
                self.group(&elems, false, false, &mut keys);
            }
        }
        keys.sort();
        keys.dedup();
        for key in keys {
            self.push(call.span, key, "opaque-call");
        }
    }

    pub(super) fn keyed(
        &mut self,
        span: Span,
        target: Option<Elem<'_, 'a>>,
        key: Option<Elem<'_, 'a>>,
        kind: &'static str,
    ) {
        let Some(target) = target else {
            return;
        };
        if let Elem::Value(expr) = target {
            self.note_rebind(expr);
        }
        let keys = key.map_or_else(
            || Keys::One(AKey::Named(Cow::Borrowed("undefined"))),
            |key| self.elem_keys(key),
        );
        if !self.is_target(target) {
            if !self.in_machinery(span) {
                self.defined.extend(keys);
            }
            return;
        }
        let root = match target {
            Elem::Value(expr) => self.target_root(expr),
            Elem::Hidden => ProtoRoot::Either,
        };
        for key in keys {
            self.push_rooted(span, key, kind, root);
        }
    }

    pub(super) fn batch(
        &mut self,
        span: Span,
        target: Option<Elem<'_, 'a>>,
        sources: &[Elem<'_, 'a>],
    ) {
        let Some(target) = target else {
            return;
        };
        let mut keys = Vec::new();
        for source in sources {
            match source {
                Elem::Value(expr) => self.keys_of(expr, &mut keys),
                Elem::Hidden => keys.push(AKey::Unknown),
            }
        }
        if !self.is_target(target) {
            if !self.in_machinery(span) {
                self.defined.extend(keys);
            }
            return;
        }
        for key in keys {
            self.push(span, key, "batch");
        }
    }

    pub(super) fn named_kind(
        &mut self,
        span: Span,
        key: &str,
        receiver: Option<Elem<'_, 'a>>,
        args: &[Elem<'_, 'a>],
    ) {
        let at = |index: usize| args.get(index).copied();
        if DEFINE_KEYED.contains(&key) {
            self.keyed(span, at(0), at(1), "define");
        } else if key == "set" {
            self.keyed(span, at(0), at(1), "set");
            self.keyed(span, at(3), at(1), "set-receiver");
            if receiver.is_some_and(|receiver| self.is_target(receiver)) {
                self.push(
                    span,
                    AKey::Named(Cow::Borrowed("__proto__")),
                    "proto-setter",
                );
            }
        } else if BATCH.contains(&key) {
            self.batch(span, at(0), args.get(1..).unwrap_or(&[]));
        } else if key == "setPrototypeOf" {
            if at(0).is_some_and(|target| self.is_target(target)) {
                self.push(span, AKey::Named(Cow::Borrowed("__proto__")), "set-proto");
            }
        } else if GETTERS.contains(&key) {
            self.keyed(span, receiver, at(0), "getter");
        } else if MUTATORS.contains(&key)
            && receiver.is_some_and(|receiver| self.is_target(receiver))
        {
            self.push(span, AKey::Index, "mutator");
            self.push(span, AKey::Named(Cow::Borrowed("length")), "mutator");
        }
    }

    pub(super) fn spread_of<'k>(&self, argument: Option<&'k Argument<'a>>) -> Vec<Elem<'k, 'a>> {
        match argument.and_then(Argument::as_expression) {
            Some(expr) => self
                .array_elems(expr)
                .unwrap_or_else(|| vec![Elem::Hidden; 4]),
            None => Vec::new(),
        }
    }

    pub(super) fn forwarded(
        &mut self,
        call: &'a CallExpression<'a>,
        member: &'a MemberExpression<'a>,
        forward: &str,
    ) {
        let args = arg_elems(&call.arguments);
        let inner_keys = expr_member(member.object()).map(|inner| self.member_keys(inner));
        let receiver = args.first().copied();
        let rest: Vec<Elem<'_, 'a>> = if forward == "call" {
            args.get(1..).unwrap_or(&[]).to_vec()
        } else {
            self.spread_of(call.arguments.get(1))
        };
        if let Some(inner_keys) = inner_keys {
            for key in inner_keys {
                match key {
                    AKey::Named(name) => self.named_kind(call.span, &name, receiver, &rest),
                    AKey::Index => {}
                    AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => self.generic(call),
                }
            }
        }
        if forward == "apply" {
            if let Some(Elem::Value(first)) = args.first().copied() {
                if let Some(inner) = expr_member(first) {
                    let spread = self.spread_of(call.arguments.get(2));
                    for key in self.member_keys(inner) {
                        if let AKey::Named(name) = key {
                            self.named_kind(call.span, &name, args.get(1).copied(), &spread);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn judge_call(&mut self, call: &'a CallExpression<'a>) {
        if self.runner_callee(&call.callee).is_none() {
            self.literal_texts(call);
        }
        let unbounded = self.out.unbounded.is_some();
        let runner = self.out.runner_unbounded.is_some();
        if runner
            && self.runner_callee(&call.callee).is_none()
            && !self.creation_only.contains(&span_key(call.span))
        {
            self.reach_text(call.span, &call.arguments);
            self.check_text(call.span, &call.arguments, true);
        }
        if unbounded {
            self.generic(call);
            return;
        }
        let Some(member) = expr_member(&call.callee) else {
            return;
        };
        let args = arg_elems(&call.arguments);
        for key in self.call_keys(member) {
            match key {
                AKey::Named(name) if name == "call" || name == "apply" => {
                    self.forwarded(call, member, &name);
                }
                AKey::Named(name) => {
                    self.named_kind(call.span, &name, Some(Elem::Value(member.object())), &args);
                }
                AKey::Index => {}
                AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => self.generic(call),
            }
        }
    }

    fn into_saturated(mut self) -> (Agnostic<'a>, bool, bool) {
        self.out.saturated = true;
        self.out.pending_texts = self.pending_texts.take();
        (self.out, true, true)
    }

    pub(super) fn judge_collected(mut self) -> (Agnostic<'a>, bool, bool) {
        if !self.walks_on() {
            return self.into_saturated();
        }
        let calls = std::mem::take(&mut self.calls);
        for call in calls {
            if !self.walks_on() {
                return self.into_saturated();
            }
            self.judge_call(call);
        }
        let saturated_while_watching = self.saturated;
        let news = std::mem::take(&mut self.news);
        let tagged = std::mem::take(&mut self.tagged);
        if self.out.runner_unbounded.is_some() {
            for new in news {
                self.reach_text(new.span, &new.arguments);
                self.check_text(new.span, &new.arguments, true);
            }
            for template in tagged {
                self.check_template(template.span, &template.quasi, true);
            }
        }
        if saturated_while_watching {
            return self.into_saturated();
        }
        let (chains, natives) = self.assumptions_hold();
        if !self.creations_hold() {
            let first = self.creation_only.iter().min();
            if let Some(span) = first.map(|key| Span::new((key >> 32) as u32, *key as u32)) {
                self.out.code_text.get_or_insert(span);
            }
        }
        self.out.pending_texts = self.pending_texts.take();
        (self.out, chains, natives)
    }

    pub(super) fn assumptions_hold(&self) -> (bool, bool) {
        let assumed = self.assumed.borrow();
        let chains = !self.out.sites.iter().any(|site| match &site.key {
            AKey::Named(name) => assumed.contains(name),
            AKey::Index => false,
            AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => !assumed.is_empty(),
        });
        let assigns = !self.assign_used.get()
            || !self
                .out
                .sites
                .iter()
                .filter(|site| site.outside)
                .any(|site| match &site.key {
                    AKey::Named(name) => ASSIGN_REFUTERS.contains(&name.as_ref()),
                    AKey::Index => false,
                    AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => true,
                });
        let natives = assigns
            && (!self.fresh_used.get()
                || !self.refute_fresh
                    && !self.defined_keys().any(|key| match key {
                        AKey::Named(name) => FRESH_REFUTERS.contains(&name.as_ref()),
                        AKey::Index => false,
                        AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => true,
                    }));
        (chains, natives)
    }

    fn defined_keys(&self) -> impl Iterator<Item = &AKey<'a>> + '_ {
        self.out
            .sites
            .iter()
            .filter(|site| site.outside)
            .map(|site| &site.key)
            .chain(&self.defined)
    }

    pub(super) fn creations_hold(&self) -> bool {
        !self.creation_keys.iter().any(|created| {
            self.defined_keys().any(|key| match key {
                AKey::Named(_) => key == created,
                AKey::Index => false,
                AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => true,
            })
        })
    }
}
