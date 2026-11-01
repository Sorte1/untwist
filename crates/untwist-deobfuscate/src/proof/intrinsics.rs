use oxc_ast::ast::*;
use oxc_ast::match_assignment_target;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SymbolId};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use super::numeric_keys::{numeric_deps, numeric_overwrite, overwritten_parameter, NumericKeys};
use super::property_reads::{
    argument_read, array_element_read, assignment_pattern_read, assignment_property_read,
    binary_read, binding_pattern_read, binding_property_read, chain_element_read,
    class_element_read, expression_read, jsx_member_read, jsx_name_read, jsx_object_read,
    maybe_default_read, member_read, object_property_read, simple_target_read, statement_read,
    KeyRead,
};
use super::writes::{key_of, PatchKey};

const REACHING_GLOBALS: &[&str] = &[
    "require",
    "module",
    "exports",
    "process",
    "vm",
    "importScripts",
    "queueMicrotask",
    "Worker",
    "structuredClone",
    "Function",
    "eval",
    "Reflect",
    "execScript",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "globalThis",
    "window",
    "self",
    "global",
    "frames",
];

const REACHING_KEYS: &[&str] = &[
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "setPrototypeOf",
    "defineProperty",
    "defineProperties",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "constructor",
    "require",
    "module",
    "process",
    "vm",
    "Function",
    "eval",
    "Reflect",
    "Object",
    "execScript",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "globalThis",
    "window",
    "self",
    "global",
    "frames",
];

const TARGET_WRITER: &str = "assign";

const RECEIVER_SETTERS: &[&str] = &["call", "apply", "bind"];

struct BaseScan<'s> {
    scoping: &'s Scoping,
    resolve: &'s dyn Fn(Span) -> Option<String>,
    blocked: bool,
    strict: Vec<bool>,
    fresh: FxHashSet<SymbolId>,
    rewritten: FxHashSet<SymbolId>,
    bases: Vec<SymbolId>,
    checked_writer: Option<Span>,
    numeric: NumericKeys,
    machinery: &'s [Span],
    parameters: bool,
    overwritten: FxHashSet<SymbolId>,
}

fn is_fresh(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}

fn static_key<'a>(expr: &Expression<'a>) -> PatchKey<'a> {
    let Expression::TemplateLiteral(template) = expr.without_parentheses() else {
        return key_of(expr);
    };
    let cooked = template
        .quasis
        .first()
        .and_then(|quasi| quasi.value.cooked.as_ref());
    match (template.expressions.is_empty(), cooked) {
        (true, Some(text)) => PatchKey::Named(text.as_str()),
        _ => PatchKey::Dynamic,
    }
}

fn member_key<'a>(member: &MemberExpression<'a>) -> Option<PatchKey<'a>> {
    match member {
        MemberExpression::StaticMemberExpression(m) => {
            Some(PatchKey::Named(m.property.name.as_str()))
        }
        MemberExpression::ComputedMemberExpression(m) => Some(static_key(&m.expression)),
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn arithmetic_update(operator: AssignmentOperator) -> bool {
    !matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::Addition
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalNullish
    )
}

impl BaseScan<'_> {
    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id())
    }

    fn is_unresolved_object(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => id.name == "Object" && self.symbol(id).is_none(),
            _ => false,
        }
    }

    fn in_machinery(&self, span: Span) -> bool {
        self.machinery
            .iter()
            .any(|unit| unit.start <= span.start && span.end <= unit.end)
    }

    fn key_name(&self, key: PatchKey<'_>) -> Option<String> {
        match key {
            PatchKey::Named(name) => Some(name.to_string()),
            PatchKey::Pending(span) if self.in_machinery(span) => (self.resolve)(span),
            PatchKey::Pending(_) | PatchKey::Dynamic | PatchKey::Whole => None,
        }
    }

    fn check_read(&mut self, read: KeyRead<'_, '_>, span: Span) -> bool {
        match read {
            KeyRead::Static(name) => self.check_key(PatchKey::Named(&name), span),
            KeyRead::Computed(key) => self.check_computed_key(key, span),
            KeyRead::Private | KeyRead::Nested | KeyRead::Defines | KeyRead::Unkeyed => false,
            KeyRead::Unmodelled => {
                self.blocked = true;
                false
            }
        }
    }

    fn check_computed_key(&mut self, key: &Expression<'_>, span: Span) -> bool {
        let named = self.check_key(static_key(key), span);
        if named || self.in_machinery(span) {
            return named;
        }
        let mut deps = Vec::new();
        if numeric_deps(key, self.scoping, &mut deps) {
            for symbol in deps {
                self.numeric.require(symbol);
            }
        } else {
            self.blocked = true;
        }
        false
    }

    fn spoil_number(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.numeric.spoil(symbol);
        }
    }

    fn declare_number(&mut self, id: &BindingIdentifier<'_>, init: Option<&Expression<'_>>) {
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        self.numeric.declare(symbol);
        match init {
            Some(value) => self.numeric.assign(symbol, value, self.scoping),
            None => self.numeric.spoil(symbol),
        }
    }

    fn assign_identifier(
        &mut self,
        id: &IdentifierReference<'_>,
        operator: AssignmentOperator,
        value: &Expression<'_>,
    ) {
        let Some(symbol) = self.symbol(id) else {
            self.blocked = true;
            return;
        };
        let numeric = operator == AssignmentOperator::Assign && numeric_overwrite(value);
        if self.overwritten.contains(&symbol) && !numeric {
            self.numeric.spoil(symbol);
        }
        match operator {
            AssignmentOperator::Assign | AssignmentOperator::Addition => {
                if operator != AssignmentOperator::Assign || !is_fresh(value) {
                    self.rewritten.insert(symbol);
                }
                self.numeric.assign(symbol, value, self.scoping);
            }
            other if arithmetic_update(other) => {
                self.rewritten.insert(symbol);
            }
            _ => self.rewrite(id),
        }
    }

    fn update_in_place(&mut self, id: &IdentifierReference<'_>) {
        match self.symbol(id) {
            Some(symbol) => {
                self.rewritten.insert(symbol);
                if self.overwritten.contains(&symbol) {
                    self.numeric.spoil(symbol);
                }
            }
            None => self.blocked = true,
        }
    }

    fn check_key(&mut self, key: PatchKey<'_>, span: Span) -> bool {
        let Some(name) = self.key_name(key) else {
            return false;
        };
        let unchecked_writer = name == TARGET_WRITER && self.checked_writer != Some(span);
        self.blocked |= unchecked_writer || REACHING_KEYS.contains(&name.as_str());
        true
    }

    fn check_base(&mut self, base: &Expression<'_>) {
        if is_fresh(base) {
            return;
        }
        let Expression::Identifier(id) = base.without_parentheses() else {
            self.blocked = true;
            return;
        };
        match self.symbol(id) {
            Some(symbol) => self.bases.push(symbol),
            None => self.blocked = true,
        }
    }

    fn rewrite(&mut self, id: &IdentifierReference<'_>) {
        match self.symbol(id) {
            Some(symbol) => {
                self.rewritten.insert(symbol);
                self.numeric.spoil(symbol);
            }
            None => self.blocked = true,
        }
    }

    fn declare(&mut self, id: &BindingIdentifier<'_>, fresh: bool) {
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        if fresh {
            self.fresh.insert(symbol);
        } else {
            self.rewritten.insert(symbol);
            self.numeric.spoil(symbol);
        }
    }

    fn declare_pattern(&mut self, pattern: &BindingPattern<'_>) {
        for id in pattern.get_binding_identifiers() {
            self.declare(id, false);
        }
    }

    fn check_receiver_setter(&mut self, key: PatchKey<'_>, function: &Expression<'_>) {
        let sets_receiver = self
            .key_name(key)
            .is_some_and(|name| RECEIVER_SETTERS.contains(&name.as_str()));
        if sets_receiver {
            self.check_base(function);
        }
    }

    fn declare_loop_head(&mut self, left: &ForStatementLeft<'_>) {
        match left {
            ForStatementLeft::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    self.declare_pattern(&declarator.id);
                }
            }
            match_assignment_target!(ForStatementLeft) => {}
        }
    }

    fn is_strict(&self) -> bool {
        self.strict.last().copied().unwrap_or(false)
    }

    fn check_callee(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) {
        let Some(member) = callee.without_parentheses().as_member_expression() else {
            return;
        };
        let Some(name) = member_key(member).and_then(|key| self.key_name(key)) else {
            self.blocked = true;
            return;
        };
        if name != TARGET_WRITER {
            return;
        }
        self.checked_writer = Some(member.span());
        match arguments.first().and_then(Argument::as_expression) {
            Some(target) => self.check_base(target),
            None => self.blocked = true,
        }
    }

    fn verdict(self) -> bool {
        let fresh_bases = self
            .bases
            .iter()
            .all(|symbol| self.fresh.contains(symbol) && !self.rewritten.contains(symbol));
        !self.blocked && fresh_bases && self.numeric.holds()
    }
}

impl<'a> VisitJs<'a> for BaseScan<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.blocked {
            return;
        }
        self.check_read(statement_read(it), it.span());
        walk_js::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.blocked {
            return;
        }
        self.check_read(expression_read(it), it.span());
        walk_js::walk_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        let named = self.check_read(member_read(it), it.span());
        if let Some(key) = member_key(it) {
            self.check_receiver_setter(key, it.object());
        }
        if !(named && self.is_unresolved_object(it.object())) {
            self.visit_expression(it.object());
        }
        if let MemberExpression::ComputedMemberExpression(member) = it {
            self.visit_expression(&member.expression);
        }
    }

    fn visit_chain_element(&mut self, it: &ChainElement<'a>) {
        self.check_read(chain_element_read(it), it.span());
        walk_js::walk_chain_element(self, it);
    }

    fn visit_argument(&mut self, it: &Argument<'a>) {
        self.check_read(argument_read(it), it.span());
        walk_js::walk_argument(self, it);
    }

    fn visit_array_expression_element(&mut self, it: &ArrayExpressionElement<'a>) {
        self.check_read(array_element_read(it), it.span());
        walk_js::walk_array_expression_element(self, it);
    }

    fn visit_class_element(&mut self, it: &ClassElement<'a>) {
        self.check_read(class_element_read(it), it.span());
        walk_js::walk_class_element(self, it);
    }

    fn visit_object_property_kind(&mut self, it: &ObjectPropertyKind<'a>) {
        self.check_read(object_property_read(it), it.span());
        walk_js::walk_object_property_kind(self, it);
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        self.check_read(binding_pattern_read(it), it.span());
        walk_js::walk_binding_pattern(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.check_read(binding_property_read(it), it.span);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_pattern(&mut self, it: &AssignmentTargetPattern<'a>) {
        self.check_read(assignment_pattern_read(it), it.span());
        walk_js::walk_assignment_target_pattern(self, it);
    }

    fn visit_assignment_target_maybe_default(&mut self, it: &AssignmentTargetMaybeDefault<'a>) {
        self.check_read(maybe_default_read(it), it.span());
        walk_js::walk_assignment_target_maybe_default(self, it);
    }

    fn visit_assignment_target_property(&mut self, it: &AssignmentTargetProperty<'a>) {
        self.check_read(assignment_property_read(it), it.span());
        walk_js::walk_assignment_target_property(self, it);
    }

    fn visit_jsx_element_name(&mut self, it: &JSXElementName<'a>) {
        self.check_read(jsx_name_read(it), it.span());
        walk_js::walk_jsx_element_name(self, it);
    }

    fn visit_jsx_member_expression_object(&mut self, it: &JSXMemberExpressionObject<'a>) {
        self.check_read(jsx_object_read(it), it.span());
        walk_js::walk_jsx_member_expression_object(self, it);
    }

    fn visit_jsx_member_expression(&mut self, it: &JSXMemberExpression<'a>) {
        self.check_read(jsx_member_read(it), it.span);
        walk_js::walk_jsx_member_expression(self, it);
    }

    fn visit_program(&mut self, it: &Program<'a>) {
        let strict = it.source_type.is_module() || it.has_use_strict_directive();
        self.strict.push(strict);
        walk_js::walk_program(self, it);
        self.strict.pop();
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.declare(id, true);
            self.spoil_number(id);
        }
        if let Some(symbol) = self
            .parameters
            .then(|| overwritten_parameter(it, self.scoping))
            .flatten()
        {
            self.overwritten.insert(symbol);
            self.numeric.declare(symbol);
        }
        let strict = self.is_strict() || it.has_use_strict_directive();
        self.strict.push(strict);
        walk_js::walk_function(self, it, flags);
        self.strict.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(id) = &it.id {
            self.declare(id, true);
            self.spoil_number(id);
        }
        self.strict.push(true);
        walk_js::walk_class(self, it);
        self.strict.pop();
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.blocked |= !self.is_strict();
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.declare_loop_head(&it.left);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.declare_loop_head(&it.left);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        for id in it.pattern.get_binding_identifiers() {
            match id.symbol_id.get() {
                Some(symbol) if self.overwritten.contains(&symbol) => {
                    self.rewritten.insert(symbol);
                }
                _ => self.declare(id, false),
            }
        }
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_catch_parameter(&mut self, it: &CatchParameter<'a>) {
        self.declare_pattern(&it.pattern);
        walk_js::walk_catch_parameter(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        match &it.id {
            BindingPattern::BindingIdentifier(id) => {
                self.declare(id, it.init.as_ref().is_none_or(is_fresh));
                self.declare_number(id, it.init.as_ref());
            }
            other => self.declare_pattern(other),
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left else {
            walk_js::walk_assignment_expression(self, it);
            return;
        };
        self.assign_identifier(id, it.operator, &it.right);
        self.visit_identifier_reference(id);
        self.visit_expression(&it.right);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &it.argument else {
            walk_js::walk_update_expression(self, it);
            return;
        };
        self.update_in_place(id);
        self.visit_identifier_reference(id);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.check_read(simple_target_read(it), it.span());
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.rewrite(id);
        } else if let Some(member) = it.as_member_expression() {
            self.check_base(member.object());
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.rewrite(&it.binding);
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                self.check_base(member.object());
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.check_callee(&it.callee, &it.arguments);
        if !self.is_unresolved_object(&it.callee) {
            self.visit_expression(&it.callee);
        }
        self.visit_arguments(&it.arguments);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.check_callee(&it.callee, &it.arguments);
        if !self.is_unresolved_object(&it.callee) {
            self.visit_expression(&it.callee);
        }
        self.visit_arguments(&it.arguments);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.check_read(binary_read(it), it.span);
        let intrinsic_operand =
            it.operator == BinaryOperator::Instanceof && self.is_unresolved_object(&it.right);
        self.visit_expression(&it.left);
        if !intrinsic_operand {
            self.visit_expression(&it.right);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.symbol(it).is_some() {
            return;
        }
        let name = it.name.as_str();
        self.blocked |= name == "Object" || REACHING_GLOBALS.contains(&name);
    }
}

fn base_scan<'s>(
    scoping: &'s Scoping,
    resolve: &'s dyn Fn(Span) -> Option<String>,
    machinery: &'s [Span],
) -> BaseScan<'s> {
    BaseScan {
        scoping,
        resolve,
        blocked: false,
        strict: Vec::new(),
        fresh: FxHashSet::default(),
        rewritten: FxHashSet::default(),
        bases: Vec::new(),
        checked_writer: None,
        numeric: NumericKeys::default(),
        machinery,
        parameters: false,
        overwritten: FxHashSet::default(),
    }
}

pub(crate) enum Machinery<'k, 'a> {
    Function(&'k Function<'a>),
    Statement(&'k Statement<'a>),
}

pub(crate) fn machinery_unreachable(
    scoping: &Scoping,
    parts: &[(Machinery<'_, '_>, bool)],
) -> bool {
    let resolve = |_: Span| None;
    let mut scan = base_scan(scoping, &resolve, &[]);
    scan.parameters = true;
    for (part, strict) in parts {
        scan.strict.push(*strict);
        match part {
            Machinery::Function(function) => scan.visit_function(function, ScopeFlags::Function),
            Machinery::Statement(statement) => scan.visit_statement(statement),
        }
        scan.strict.pop();
    }
    scan.verdict()
}
