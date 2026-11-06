use oxc_ast::ast::{
    Argument, ArrayExpression, ArrowFunctionExpression, AssignmentExpression, AssignmentOperator,
    AssignmentTarget, AssignmentTargetPattern, BindingPattern, BlockStatement, CallExpression,
    CatchClause, Class, Expression, ForInStatement, ForOfStatement, ForStatement, ForStatementLeft,
    Function, IdentifierReference, IfStatement, LogicalExpression, LogicalOperator,
    MemberExpression, NewExpression, SimpleAssignmentTarget, Statement, SwitchStatement,
    UnaryExpression, UpdateExpression, VariableDeclarator, WithStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;

use super::judge::{
    is_arguments_callee, is_bare_arguments, is_this_rooted, root_of, Deferred, Reach, Walker,
};
use super::prefix::prefix_gate_tests;
use super::scope::{
    declared_in_arrow, declared_in_block, declared_in_body, declared_in_for_head,
    declared_in_for_init, declared_in_switch, own_expression_name, push_pattern, Declared,
    FrameKind, Freshness,
};
use super::{gate_guard, member_peel_from, FinalKey, MemberAccess, RecognizedGate, TriggerKind};
use crate::stack;

fn may_replace_prototype(member: &MemberExpression<'_>) -> bool {
    member
        .static_property_name()
        .and_then(|name| name.as_str())
        .is_none_or(|name| name == "prototype")
}

impl<'a> Walker<'a, '_> {
    fn note_identifier_write(
        &mut self,
        id: &IdentifierReference<'a>,
        span: Span,
        capture: bool,
    ) -> bool {
        let name = id.name.as_str();
        let Some(binding) = self.resolve(name) else {
            let exempt =
                self.exempt_self == Some(name) && self.innermost_kind() == Some(FrameKind::Outer);
            if !exempt {
                self.push_trigger(span, TriggerKind::Identifier);
            }
            return false;
        };
        self.touch(binding, id.span);
        let from_inner = self.innermost_kind() != Some(FrameKind::Outer);
        if self.frame_kind_of(binding) == FrameKind::Outer && from_inner {
            if capture && self.in_region() {
                self.bindings[binding].writes += 1;
                self.captured.push(binding);
                self.out.captures.push(span);
                return true;
            }
            self.push_trigger(span, TriggerKind::Identifier);
            return false;
        }
        self.bindings[binding].writes += 1;
        self.check_region(binding);
        false
    }

    fn note_member_write(&mut self, member: &MemberExpression<'a>, span: Span) {
        if is_this_rooted(member.object()) {
            let kind = if self.this_is_nested() {
                TriggerKind::Monitored
            } else {
                TriggerKind::This
            };
            self.push_trigger(span, kind);
            return;
        }
        let Some((root, reach)) = root_of(member) else {
            self.push_trigger(span, TriggerKind::Unrecognized);
            return;
        };
        match self.resolve(root.name.as_str()) {
            Some(binding) => self.defer_frame_write(binding, member, span, reach),
            None => match member_peel_from(member) {
                Some(peel) => {
                    self.out.accesses.push(MemberAccess {
                        peel,
                        is_write: true,
                        local: false,
                        at: self.access_at(peel.container_span),
                    });
                    self.push_trigger(span, TriggerKind::Member(peel));
                }
                None => self.push_trigger(span, TriggerKind::Unrecognized),
            },
        }
    }

    fn defer_frame_write(
        &mut self,
        binding: usize,
        member: &MemberExpression<'a>,
        span: Span,
        reach: Reach,
    ) {
        if self.frame_kind_of(binding) != FrameKind::Outer {
            let entry = &mut self.bindings[binding];
            if entry.fresh == Freshness::Function
                && reach == Reach::Direct
                && may_replace_prototype(member)
            {
                entry.fresh = Freshness::Stale;
            }
            self.deferred.push(Deferred::Local {
                binding,
                span,
                reach,
            });
            return;
        }
        let computed = matches!(member, MemberExpression::ComputedMemberExpression(_));
        let peel = (reach == Reach::Direct && computed)
            .then(|| member_peel_from(member))
            .flatten();
        if let Some(peel) = peel {
            self.out.accesses.push(MemberAccess {
                peel,
                is_write: true,
                local: true,
                at: self.access_at(peel.container_span),
            });
        }
        self.deferred.push(Deferred::Captured {
            binding,
            span,
            peel,
        });
    }

    fn note_target(&mut self, target: &AssignmentTarget<'a>, span: Span, capture: bool) -> bool {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
            return self.note_identifier_write(id, span, capture);
        }
        match target.as_member_expression() {
            Some(member) => {
                self.note_member_write(member, span);
                self.walk_member_parts(member);
            }
            None => self.push_trigger(span, TriggerKind::Unrecognized),
        }
        false
    }

    fn note_simple_target(&mut self, target: &SimpleAssignmentTarget<'a>, span: Span) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = target {
            self.note_identifier_write(id, span, false);
            return;
        }
        match target.as_member_expression() {
            Some(member) => {
                self.note_member_write(member, span);
                self.walk_member_parts(member);
            }
            None => self.push_trigger(span, TriggerKind::Unrecognized),
        }
    }

    fn note_arguments_store(&mut self, target: &AssignmentTarget<'a>) -> bool {
        let Some(member) = target.as_member_expression() else {
            return false;
        };
        let Some(peel) = member_peel_from(member) else {
            return false;
        };
        let (None, FinalKey::Static(key)) = (peel.segment, peel.final_key) else {
            return false;
        };
        if self.resolve(peel.root).is_some() || !self.in_region() {
            return false;
        }
        self.out.arguments_stores.push((peel.root, key));
        true
    }

    fn walk_member_parts(&mut self, member: &MemberExpression<'a>) {
        match member.object().without_parentheses() {
            Expression::Identifier(id) => self.note_reference(id, true),
            other => self.visit_expression(other),
        }
        if let MemberExpression::ComputedMemberExpression(computed) = member {
            self.visit_expression(&computed.expression);
        }
    }

    fn walk_callee(&mut self, callee: &Expression<'a>, arguments: &[Argument<'a>]) {
        let local = match callee.without_parentheses() {
            Expression::Identifier(id) => {
                self.calling = true;
                self.note_reference(id, true);
                self.calling = false;
                self.resolve(id.name.as_str())
            }
            other => {
                self.note_receiver(other);
                self.visit_expression(other);
                None
            }
        };
        for argument in arguments {
            let bare = argument
                .as_expression()
                .is_some_and(|expr| matches!(expr, Expression::Identifier(_)));
            self.local_callee = local.filter(|_| bare);
            self.visit_argument(argument);
            self.local_callee = None;
        }
    }

    fn note_receiver(&mut self, callee: &Expression<'a>) {
        let Some(peel) = callee.as_member_expression().and_then(member_peel_from) else {
            return;
        };
        if let (None, FinalKey::Static(key)) = (peel.segment, peel.final_key) {
            if self.watched.contains(&peel.root) && self.resolve(peel.root).is_none() {
                self.out.receivers.push((peel.root, key));
            }
        }
    }

    fn note_function_write(&mut self, target: &AssignmentTarget<'a>, value: &Expression<'a>) {
        let Some(peel) = target.as_member_expression().and_then(member_peel_from) else {
            return;
        };
        let (None, FinalKey::Static(key)) = (peel.segment, peel.final_key) else {
            return;
        };
        if !self.watched.contains(&peel.root) || self.resolve(peel.root).is_some() {
            return;
        }
        let function = match value.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                Some(usize::MAX)
            }
            Expression::Identifier(id) => self.resolve(id.name.as_str()),
            _ => None,
        };
        self.pending_writes.push((peel.root, key, function));
    }

    fn recognize_gate(&mut self, test: &Expression<'a>, region: Span) -> bool {
        let conditional = !self.gate_tests.contains(&test.span());
        let Some(peel) = gate_guard(test) else {
            return false;
        };
        let FinalKey::Static(flag_key) = peel.final_key else {
            return false;
        };
        self.out.gates.push(RecognizedGate {
            root: peel.root,
            segment: peel.segment,
            flag_key,
            region,
            test: test.span(),
            conditional,
        });
        true
    }

    fn in_block_scope(&mut self, declared: Vec<Declared<'a>>, walk: impl FnOnce(&mut Self)) {
        if declared.is_empty() {
            walk(self);
            return;
        }
        self.push_block_frame(declared);
        walk(self);
        self.pop_frame();
    }

    fn walk_for_head(&mut self, left: &ForStatementLeft<'a>, span: Span) {
        match left {
            ForStatementLeft::VariableDeclaration(decl) => self.visit_variable_declaration(decl),
            other => match other.as_assignment_target() {
                Some(target) => {
                    self.note_target(target, span, false);
                }
                None => self.push_trigger(span, TriggerKind::Unrecognized),
            },
        }
    }
}

impl<'a> VisitJs<'a> for Walker<'a, '_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let kind = if self.call_target == Some(it.span) {
            FrameKind::Call
        } else {
            FrameKind::Nested
        };
        let Some(body) = &it.body else {
            return;
        };
        let outer = self.strict;
        let strict = outer || it.has_use_strict_directive();
        let mut declared = declared_in_body(&it.params, &body.statements, strict);
        declared.extend(own_expression_name(it));
        let creation = self.creation;
        if kind == FrameKind::Nested && creation.is_none() {
            self.creation = Some(self.creation_of(it.span, it.is_declaration()));
        }
        let first = self.bindings.len();
        self.push_frame(kind, false, declared, body.span.start);
        self.enter_arguments_scope(it, first);
        if kind == FrameKind::Call {
            prefix_gate_tests(&body.statements, &mut self.gate_tests);
            self.note_inner_parameters(it, first);
        }
        self.strict = strict;
        self.visit_formal_parameters(&it.params);
        self.visit_function_body(body);
        self.leave_arguments_scope();
        self.pop_frame();
        self.strict = outer;
        self.creation = creation;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let creation = self.creation;
        self.creation = Some(creation.unwrap_or(it.span));
        let outer = self.strict;
        self.strict = outer || it.has_use_strict_directive();
        self.push_frame(
            FrameKind::Nested,
            true,
            declared_in_arrow(it, self.strict),
            it.body.span().start,
        );
        self.visit_formal_parameters(&it.params);
        self.visit_arrow_function_body(&it.body);
        self.pop_frame();
        self.strict = outer;
        self.creation = creation;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.push_trigger(it.span, TriggerKind::Unrecognized);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.push_trigger(it.span, TriggerKind::Unrecognized);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.push_trigger(it.span(), TriggerKind::Unrecognized);
    }

    fn visit_assignment_target_pattern(&mut self, it: &AssignmentTargetPattern<'a>) {
        self.push_trigger(it.span(), TriggerKind::Unrecognized);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let bare = it.operator == AssignmentOperator::Assign && is_bare_arguments(&it.right);
        self.note_function_write(&it.left, &it.right);
        let mut exempt = self.note_target(&it.left, it.span, bare);
        if bare && !exempt {
            exempt = self.note_arguments_store(&it.left);
        }
        let previous = self.arguments_exempt;
        if exempt {
            self.arguments_exempt = Some(it.right.span());
        }
        self.visit_expression(&it.right);
        self.arguments_exempt = previous;
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        self.note_simple_target(&it.argument, it.span);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator != UnaryOperator::Delete {
            walk_js::walk_unary_expression(self, it);
            return;
        }
        match it.argument.without_parentheses().as_member_expression() {
            Some(member) => {
                self.note_member_write(member, it.span);
                self.walk_member_parts(member);
            }
            None => {
                self.push_trigger(it.span, TriggerKind::Unrecognized);
                walk_js::walk_unary_expression(self, it);
            }
        }
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.in_block_scope(declared_in_for_head(&it.left), |walker| {
            walker.walk_for_head(&it.left, it.span);
            walker.visit_expression(&it.right);
            walker.visit_statement(&it.body);
        });
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.in_block_scope(declared_in_for_head(&it.left), |walker| {
            walker.walk_for_head(&it.left, it.span);
            walker.visit_expression(&it.right);
            walker.visit_statement(&it.body);
        });
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.in_block_scope(declared_in_for_init(it.init.as_ref()), |walker| {
            walk_js::walk_for_statement(walker, it);
        });
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        let declared = declared_in_block(&it.body, self.strict);
        self.in_block_scope(declared, |walker| {
            walk_js::walk_block_statement(walker, it);
        });
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.visit_expression(&it.discriminant);
        let declared = declared_in_switch(it, self.strict);
        self.in_block_scope(declared, |walker| {
            for case in &it.cases {
                walker.visit_switch_case(case);
            }
        });
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        let mut declared = Vec::new();
        if let Some(param) = &it.param {
            push_pattern(&param.pattern, &mut declared);
        }
        self.in_block_scope(declared, |walker| {
            walk_js::walk_catch_clause(walker, it);
        });
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if it.init.is_some() && self.in_region() {
            if let Some(id) = it.id.get_binding_identifier() {
                if let Some(binding) = self.resolve(id.name.as_str()) {
                    self.touch(binding, id.span);
                    self.check_region(binding);
                }
            }
        }
        self.alias_of = self.alias_binding(it);
        walk_js::walk_variable_declarator(self, it);
        self.alias_of = None;
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.note_index_read(it);
        if let Some(peel) = member_peel_from(it) {
            let local = self.resolve(peel.root).is_some();
            self.out.accesses.push(MemberAccess {
                peel,
                is_write: false,
                local,
                at: self.access_at(peel.container_span),
            });
        }
        if is_arguments_callee(it) {
            self.push_trigger(it.span(), TriggerKind::Monitored);
        }
        self.walk_member_parts(it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        for element in &it.elements {
            if !element.as_expression().is_some_and(Expression::is_literal) {
                self.visit_array_expression_element(element);
            }
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.walk_callee(&it.callee, &it.arguments);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.walk_callee(&it.callee, &it.arguments);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.note_reference(it, false);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.visit_expression(&it.test);
        let region = it.consequent.span();
        if it.alternate.is_none() && self.recognize_gate(&it.test, region) {
            self.enter_region(region);
            self.visit_statement(&it.consequent);
            self.leave_region();
        } else {
            self.visit_statement(&it.consequent);
        }
        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.visit_expression(&it.left);
        let region = it.right.span();
        if it.operator == LogicalOperator::And && self.recognize_gate(&it.left, region) {
            self.enter_region(region);
            self.visit_expression(&it.right);
            self.leave_region();
        } else {
            self.visit_expression(&it.right);
        }
    }
}
