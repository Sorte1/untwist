use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use std::borrow::Cow;

use super::keys::Keys;
use super::roles::*;
use super::sites::*;
use super::text::closed_runner_read;
use super::AKey;

impl<'s, 'a> Sites<'s, 'a> {
    fn stringified_runner(&self, call: &CallExpression<'a>) -> Option<Span> {
        if !call.arguments.is_empty() {
            return None;
        }
        let member = call.callee.without_parentheses().as_member_expression()?;
        if !self.keys_are(member, &["toString"]) {
            return None;
        }
        let mut base = member.object().without_parentheses();
        while let Some(inner) = base.as_member_expression() {
            if !self.keys_are(inner, &["prototype", "constructor", "toString"]) {
                return None;
            }
            base = inner.object().without_parentheses();
        }
        let Expression::Identifier(id) = base else {
            return None;
        };
        (RUNNERS.contains(&id.name.as_str()) && self.fresh.symbol(id).is_none()).then_some(id.span)
    }

    fn keys_are(&self, member: &MemberExpression<'a>, allowed: &[&str]) -> bool {
        let keys = self.member_keys(member);
        !keys.is_empty()
            && keys
                .iter()
                .all(|key| matches!(key, AKey::Named(name) if allowed.contains(&name.as_ref())))
    }

    fn is_intrinsic_value(&self, expr: &Expression<'a>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => {
                RUNNERS.contains(&id.name.as_str()) && self.fresh.symbol(id).is_none()
            }
            other => other.as_member_expression().is_some_and(|member| {
                self.member_keys(member).iter().any(|key| match key {
                    AKey::Named(name) => {
                        is_definer_key(name)
                            || READERS.contains(&name.as_ref())
                            || key_roles(name).runner
                            || matches!(name.as_ref(), "call" | "apply" | "bind")
                    }
                    AKey::Index => false,
                    AKey::Pending(_) | AKey::Unknown => true,
                })
            }),
        }
    }

    pub(super) fn note_call_shape(&mut self, call: &CallExpression<'a>) {
        let callee = call.callee.without_parentheses();
        self.direct.insert(span_key(callee.span()));
        if let Some(runner) = self.stringified_runner(call) {
            self.direct.insert(span_key(runner));
        }
        let Some(member) = callee.as_member_expression() else {
            return;
        };
        let keys = self.member_keys(member);
        self.reach_call(call, member, &keys);
        self.note_construct(call, &keys);
        for (function, forward) in self
            .forwarded_functions(call, member, &keys)
            .into_iter()
            .flatten()
        {
            let function = function.without_parentheses();
            self.direct.insert(span_key(function.span()));
            self.note_forwarded_reader(function, call.span);
            self.check_forwarded_runner(call, function, forward);
        }
        self.note_reader_arguments(call, &keys);
    }

    fn note_construct(&mut self, call: &CallExpression<'a>, keys: &[AKey<'a>]) {
        let constructs = keys.iter().any(|key| match key {
            AKey::Named(name) if name == "parse" => call.arguments.len() >= 2,
            other => origin_of(other).construct,
        });
        if constructs && !self.in_machinery(call.span) {
            self.refute_fresh = true;
        }
    }

    fn forwarded_functions<'k>(
        &self,
        call: &'k CallExpression<'a>,
        member: &'k MemberExpression<'a>,
        keys: &[AKey<'a>],
    ) -> [Option<(&'k Expression<'a>, Forward)>; 2] {
        let named = |wanted: &[&str]| {
            keys.iter()
                .any(|key| matches!(key, AKey::Named(name) if wanted.contains(&name.as_ref())))
        };
        let forward = Forward {
            call: named(&["call"]),
            apply: named(&["apply"]),
            reflect: false,
        };
        let forwarded = (forward.call || forward.apply).then(|| (member.object(), forward));
        let reflect_apply = matches!(member.object().without_parentheses(), Expression::Identifier(obj)
            if obj.name == "Reflect" && self.fresh.symbol(obj).is_none())
            && named(&["apply"]);
        let reflected = reflect_apply
            .then(|| call.arguments.first().and_then(Argument::as_expression))
            .flatten()
            .map(|function| {
                let forward = Forward {
                    reflect: true,
                    ..Forward::default()
                };
                (function, forward)
            });
        [forwarded, reflected]
    }

    fn note_reader_arguments(&mut self, call: &CallExpression<'a>, keys: &[AKey<'a>]) {
        let Some(first_key) = keys
            .iter()
            .filter_map(|key| match key {
                AKey::Named(name) if READERS.contains(&name.as_ref()) => {
                    Some(first_key_argument(name))
                }
                _ => None,
            })
            .min()
        else {
            return;
        };
        for argument in call.arguments.iter().skip(first_key) {
            if let Some(expr) = argument.as_expression() {
                let keys = self.key_set(expr);
                for key in keys {
                    let origin = reflective_origin(&key);
                    if origin.definer || origin.runner {
                        self.mark_origin(origin, call.span, "reflective read");
                    }
                }
            }
        }
    }

    pub(super) fn note_forwarded_reader(&mut self, function: &Expression<'a>, span: Span) {
        let Some(member) = function.as_member_expression() else {
            return;
        };
        let reads = self
            .member_keys(member)
            .iter()
            .any(|key| matches!(key, AKey::Named(name) if READERS.contains(&name.as_ref())));
        if reads {
            let origin = Origin {
                definer: true,
                ..Origin::default()
            };
            self.mark_origin(origin, span, "forwarded reflective read");
        }
    }

    pub(super) fn pattern_key(&mut self, key: &PropertyKey<'a>, span: Span) {
        let keys = match key {
            PropertyKey::StaticIdentifier(name) => {
                Keys::One(AKey::Named(Cow::Borrowed(name.name.as_str())))
            }
            PropertyKey::PrivateIdentifier(_) => Keys::Many(Vec::new()),
            other => other
                .as_expression()
                .map_or_else(|| Keys::Many(Vec::new()), |expr| self.key_set(expr)),
        };
        for key in keys {
            self.mark_origin(origin_of(&key), span, "destructured read");
        }
    }
}

impl<'s, 'a> VisitJs<'a> for Sites<'s, 'a> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.walks_on() {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.walks_on() {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.note_store(it);
        if it.operator == AssignmentOperator::Assign {
            let held: &'a AssignmentExpression<'a> = self.alloc(it);
            if let Some(member) = held.left.as_member_expression() {
                self.identity = self
                    .identity_write(member, &held.right)
                    .then(|| span_key(member.span()));
            }
            if let AssignmentTarget::ObjectAssignmentTarget(target) = &held.left {
                self.reach_assignment_target(target, &held.right);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = it.as_member_expression() {
            self.direct.insert(span_key(member.span()));
            let kind = if self.identity.take() == Some(span_key(member.span())) {
                IDENTITY
            } else {
                "write"
            };
            self.write_site(member, kind);
        } else if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            if self.fresh.symbol(id).is_none() {
                self.push(
                    id.span,
                    AKey::Named(Cow::Borrowed(id.name.as_str())),
                    "global",
                );
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        self.reach_object(it);
        if self.in_machinery(it.span) {
            walk_js::walk_object_expression(self, it);
            return;
        }
        for property in &it.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(p) => {
                    let key = self.property_key(&p.key);
                    self.defined.push(key);
                }
                ObjectPropertyKind::SpreadProperty(_) => self.defined.push(AKey::Unknown),
            }
        }
        walk_js::walk_object_expression(self, it);
    }

    fn visit_class_body(&mut self, it: &ClassBody<'a>) {
        if self.in_machinery(it.span) {
            walk_js::walk_class_body(self, it);
            return;
        }
        self.note_class_stores(it);
        for element in &it.body {
            let key = match element {
                ClassElement::MethodDefinition(method) => Some(&method.key),
                ClassElement::PropertyDefinition(property) => Some(&property.key),
                ClassElement::AccessorProperty(property) => Some(&property.key),
                _ => None,
            };
            if let Some(key) = key {
                let key = self.property_key(key);
                self.defined.push(key);
            }
        }
        walk_js::walk_class_body(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = expr_member(&it.argument) {
                self.direct.insert(span_key(member.span()));
                self.write_site(member, "delete");
            }
        }
        if matches!(
            it.operator,
            UnaryOperator::Typeof | UnaryOperator::LogicalNot | UnaryOperator::Void
        ) {
            self.mark_test(&it.argument);
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.observe_in(it);
        if it.operator.is_equality()
            || it.operator.is_compare()
            || it.operator == BinaryOperator::In
        {
            self.mark_test(&it.left);
            self.mark_test(&it.right);
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match self.static_truth(&it.test) {
            Some(true) => return self.visit_statement(&it.consequent),
            Some(false) => {
                if let Some(alternate) = &it.alternate {
                    self.visit_statement(alternate);
                }
                return;
            }
            None => {}
        }
        self.mark_test(&it.test);
        self.visit_expression(&it.test);
        self.visit_statement(&it.consequent);
        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        match self.static_truth(&it.test) {
            Some(true) => return self.visit_expression(&it.consequent),
            Some(false) => return self.visit_expression(&it.alternate),
            None => {}
        }
        self.mark_test(&it.test);
        walk_js::walk_conditional_expression(self, it);
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.mark_test(&it.test);
        walk_js::walk_while_statement(self, it);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.mark_test(&it.test);
        walk_js::walk_do_while_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.reach_enumerated(&it.right, it.span);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        if let Some(test) = &it.test {
            self.mark_test(test);
        }
        walk_js::walk_for_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if self.fresh.bindings.walks_in_place(it) {
            walk_js::walk_function(self, it, flags);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.observe_arguments(it.span, &it.callee, &it.arguments);
        if let Some((func, symbol)) = self.inline_target(it) {
            if !self.afford_inline(func) {
                self.inline_exhausted(it.span);
                return;
            }
            for argument in &it.arguments {
                if let Some(expr) = argument.as_expression() {
                    self.visit_expression(expr);
                }
            }
            self.inline_helper(func, symbol, it);
            return;
        }
        if let Some(func) = self.recursive_inline(it) {
            self.walk_unbound(func, it);
            return;
        }
        if self.fresh.is_user_function(&it.callee) {
            for argument in &it.arguments {
                if let Some(expr) = argument.as_expression() {
                    if self.is_intrinsic_value(expr) {
                        #[cfg(feature = "refusal-spans")]
                        self.trail(it.span, || {
                            "an intrinsic passed to a user function".to_string()
                        });
                        self.mark_text_code(it.span);
                    }
                    self.direct
                        .insert(span_key(expr.without_parentheses().span()));
                }
            }
        }
        self.note_call_shape(it);
        self.proxied_runner(it);
        self.note_creation(it);
        if let Some(function) = self.runner_callee(&it.callee) {
            if !self.creation_only.contains(&span_key(it.span)) {
                self.reach_text(it.span, &it.arguments);
                if closed_runner_read(it, self.fresh.scoping).is_some() {
                    self.mark_closed_runner_read(it.span);
                } else {
                    self.check_text(it.span, &it.arguments, function);
                }
            }
        }
        if !self.fresh.is_user_function(&it.callee) {
            let call = self.alloc(it);
            self.calls.push(call);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.observe_template(it);
        if let Some(function) = self.runner_callee(&it.tag) {
            self.check_template(it.span, &it.quasi, function);
        } else if !self.fresh.is_user_function(&it.tag) {
            let tagged = self.alloc(it);
            self.tagged.push(tagged);
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.observe_arguments(it.span, &it.callee, &it.arguments);
        self.direct
            .insert(span_key(it.callee.without_parentheses().span()));
        let plain_global = matches!(it.callee.without_parentheses(), Expression::Identifier(id)
            if id.name != "Proxy" && self.fresh.symbol(id).is_none());
        if !plain_global && !self.fresh.is_user_function(&it.callee) && !self.in_machinery(it.span)
        {
            self.refute_fresh = true;
        }
        if let Some(function) = self.runner_callee(&it.callee) {
            self.reach_text(it.span, &it.arguments);
            self.check_text(it.span, &it.arguments, function);
        } else if !self.fresh.is_user_function(&it.callee) {
            let new = self.alloc(it);
            self.news.push(new);
        }
        walk_js::walk_new_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        let parent_keys = self.member_keys(it);
        self.reach_member(it, &parent_keys);
        let transparent = !parent_keys.is_empty()
            && parent_keys.iter().all(|key| match key {
                AKey::Named(name) => !matches!(name.as_ref(), "call" | "apply" | "bind"),
                AKey::Index => true,
                AKey::Pending(_) | AKey::Unknown => false,
            });
        if transparent && expr_member(it.object()).is_some() {
            self.direct
                .insert(span_key(it.object().without_parentheses().span()));
        }
        let start = span_key(it.span());
        let escaped = !self.direct.contains(&start) && !self.tests.contains(&start);
        if escaped
            && parent_keys.iter().any(|key| origin_of(key).any())
            && !self.holds_array(it.object())
        {
            let has_sink = self.flow.sink_of(it.span()).is_some();
            for key in parent_keys.iter() {
                self.mark_origin_sink(origin_of(key), it.span(), "escaped read", has_sink);
            }
        }
        walk_js::walk_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "Proxy" && self.fresh.symbol(it).is_none() && !self.in_machinery(it.span) {
            self.refute_fresh = true;
        }
        if RUNNERS.contains(&it.name.as_str())
            && self.fresh.symbol(it).is_none()
            && !self.direct.contains(&span_key(it.span))
            && !self.tests.contains(&span_key(it.span))
        {
            let origin = Origin {
                runner: true,
                ..Origin::default()
            };
            self.mark_origin(origin, it.span, "escaped runner name");
        }
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.pattern_key(&it.key, it.span);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let origin = origin_of(&AKey::Named(Cow::Borrowed(it.binding.name.as_str())));
        self.mark_origin(origin, it.span, "destructured read");
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.pattern_key(&it.name, it.span);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
        if it.rest.is_some() {
            let origin = origin_of(&AKey::Unknown);
            self.mark_origin(origin, it.span, "rest pattern");
        }
        walk_js::walk_object_pattern(self, it);
    }

    fn visit_object_assignment_target(&mut self, it: &ObjectAssignmentTarget<'a>) {
        if it.rest.is_some() {
            let origin = origin_of(&AKey::Unknown);
            self.mark_origin(origin, it.span, "rest pattern");
        }
        walk_js::walk_object_assignment_target(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            self.reach_binding(&it.id, init);
            self.note_forwarders(&it.id, init);
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

fn first_key_argument(reader: &str) -> usize {
    usize::from(TARGETED_READERS.contains(&reader))
}
