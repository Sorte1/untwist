use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use std::cell::{Cell, OnceCell};

use super::{Builtins, Reliance, Scanned};
use crate::semantic::{is_foreign_global, program_is_strict};

mod keys;
mod reach;

use reach::{call_of, key_of, member_of, Key, REACHING_KEYS};

const GLOBAL_OBJECTS: [&str; 7] = [
    "globalThis",
    "window",
    "self",
    "global",
    "top",
    "parent",
    "frames",
];

const DEFINERS: [(&str, &str); 8] = [
    ("Object", "defineProperty"),
    ("Object", "defineProperties"),
    ("Object", "setPrototypeOf"),
    ("Object", "assign"),
    ("Reflect", "defineProperty"),
    ("Reflect", "set"),
    ("Reflect", "setPrototypeOf"),
    ("Reflect", "deleteProperty"),
];

const LEGACY_DEFINERS: [&str; 2] = ["__defineGetter__", "__defineSetter__"];

const PROMISE: &str = "Promise";

fn literal_key<'k>(expression: &'k Expression<'_>) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn member_path<'k>(expression: &'k Expression<'_>, path: &mut Vec<&'k str>) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(id) => {
            path.push(id.name.as_str());
            true
        }
        Expression::StaticMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            path.push(member.property.name.as_str());
            reached
        }
        Expression::ComputedMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            match literal_key(&member.expression) {
                Some(key) => {
                    path.push(key);
                    reached
                }
                None => false,
            }
        }
        _ => false,
    }
}

fn target_path<'k>(target: &'k SimpleAssignmentTarget<'_>, path: &mut Vec<&'k str>) -> bool {
    match target {
        SimpleAssignmentTarget::StaticMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            path.push(member.property.name.as_str());
            reached
        }
        SimpleAssignmentTarget::ComputedMemberExpression(member) => {
            let reached = member_path(&member.object, path);
            match literal_key(&member.expression) {
                Some(key) => {
                    path.push(key);
                    reached
                }
                None => false,
            }
        }
        _ => false,
    }
}

fn installed_value<'k, 'a>(
    read: &Expression<'_>,
    write: &'k Expression<'a>,
) -> Option<&'k Expression<'a>> {
    let Expression::AssignmentExpression(assignment) = write.without_parentheses() else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let target = assignment.left.as_simple_assignment_target()?;
    let (mut read_path, mut written) = (Vec::new(), Vec::new());
    let same = member_path(read, &mut read_path)
        && target_path(target, &mut written)
        && read_path.len() > 1
        && read_path == written;
    same.then_some(&assignment.right)
}

fn installs_if_missing<'k, 'a>(logical: &'k LogicalExpression<'a>) -> Option<&'k Expression<'a>> {
    if logical.operator != LogicalOperator::Or {
        return None;
    }
    installed_value(&logical.left, &logical.right)
}

fn only_expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(expression) => Some(&expression.expression),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only_expression(only),
            _ => None,
        },
        _ => None,
    }
}

fn guarded_install<'k, 'a>(statement: &'k IfStatement<'a>) -> Option<&'k Expression<'a>> {
    if statement.alternate.is_some() {
        return None;
    }
    let Expression::UnaryExpression(test) = statement.test.without_parentheses() else {
        return None;
    };
    if test.operator != UnaryOperator::LogicalNot {
        return None;
    }
    installed_value(&test.argument, only_expression(&statement.consequent)?)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum This {
    Global,
    MaybeGlobal,
    NotGlobal,
}

#[derive(Clone, Copy)]
struct Frame {
    strict: bool,
    this: This,
}

impl Frame {
    fn of_program(program: &Program<'_>, module: bool) -> Self {
        Self {
            strict: module || program_is_strict(program),
            this: if module {
                This::NotGlobal
            } else {
                This::Global
            },
        }
    }
}

struct Scan<'s> {
    program: &'s Program<'s>,
    spelling: OnceCell<FxHashSet<SymbolId>>,
    reaching_text: OnceCell<bool>,
    unplain: OnceCell<FxHashSet<SymbolId>>,
    enumerated: OnceCell<keys::EnumeratedKeys>,
    strict: bool,
    opaque_key: Cell<bool>,
    scoping: &'s Scoping,
    reliance: &'s Reliance,
    writes_only: bool,
    first_only: bool,
    frames: Vec<Frame>,
    verdict: Builtins,
    promise_written: bool,
}

fn names_a_foreign_root(scoping: &Scoping, reliance: &Reliance) -> bool {
    scoping.root_unresolved_references().keys().any(|name| {
        let name = name.as_str();
        let relied = reliance.roots.contains(&name)
            || GLOBAL_OBJECTS.contains(&name)
            || reach::CODE_RUNNERS.contains(&name);
        relied && is_foreign_global(name)
    })
}

fn first_verdict(scoping: &Scoping, reliance: &Reliance) -> Builtins {
    match names_a_foreign_root(scoping, reliance) {
        true => Builtins::Written,
        false => Builtins::Intact,
    }
}

pub fn scan(
    program: &Program<'_>,
    scoping: &Scoping,
    module: bool,
    writes_only: bool,
    reliance: &Reliance,
) -> Scanned {
    let run = |strict: bool| {
        let mut scan = Scan::new(program, scoping, module, reliance, strict);
        scan.writes_only = writes_only;
        scan.visit_program(program);
        scan
    };
    let lenient = run(false);
    let verdict = match lenient.verdict {
        Builtins::Intact if lenient.opaque_key.get() => run(true).verdict.min_aliased(),
        verdict => verdict,
    };
    Scanned {
        verdict,
        promise_written: lenient.promise_written,
    }
}

pub fn intact(program: &Program<'_>, scoping: &Scoping, module: bool, reliance: &Reliance) -> bool {
    let run = |strict: bool| {
        let mut scan = Scan::new(program, scoping, module, reliance, strict);
        scan.first_only = true;
        scan.visit_program(program);
        scan
    };
    let lenient = run(false);
    lenient.verdict == Builtins::Intact
        && (!lenient.opaque_key.get() || run(true).verdict == Builtins::Intact)
}

impl<'s> Scan<'s> {
    fn new(
        program: &'s Program<'s>,
        scoping: &'s Scoping,
        module: bool,
        reliance: &'s Reliance,
        strict: bool,
    ) -> Self {
        Self {
            program,
            spelling: OnceCell::new(),
            reaching_text: OnceCell::new(),
            unplain: OnceCell::new(),
            enumerated: OnceCell::new(),
            strict,
            opaque_key: Cell::new(false),
            scoping,
            reliance,
            writes_only: false,
            first_only: false,
            frames: vec![Frame::of_program(program, module)],
            verdict: first_verdict(scoping, reliance),
            promise_written: false,
        }
    }
}

impl Scan<'_> {
    pub(super) fn names_a_reaching_key(&self) -> bool {
        *self
            .reaching_text
            .get_or_init(|| keys::names_a_reaching_key(self.program, &REACHING_KEYS))
    }

    pub(super) fn enumerated_owner(
        &self,
        reference: oxc_syntax::reference::ReferenceId,
    ) -> Option<SymbolId> {
        self.enumerated
            .get_or_init(|| keys::enumerated_keys(self.program, self.scoping))
            .get(&reference)
            .copied()
    }

    pub(super) fn plain_symbol(&self, symbol: SymbolId) -> bool {
        !self
            .unplain
            .get_or_init(|| keys::unplain_symbols(self.program, self.scoping))
            .contains(&symbol)
    }

    pub(super) fn spelling_symbol(&self, symbol: SymbolId) -> bool {
        self.spelling
            .get_or_init(|| keys::spelling_symbols(self.program, self.scoping))
            .contains(&symbol)
    }

    fn pattern_key_reaches(&self, computed: bool, key: &PropertyKey<'_>) -> bool {
        let name = match (computed, key) {
            (true, PropertyKey::StaticIdentifier(_) | PropertyKey::PrivateIdentifier(_)) => None,
            (true, _) => match key.as_expression().map(key_of) {
                Some(Key::Name(name)) => Some(name),
                Some(Key::Other) | None => return false,
                Some(Key::Unknown(expression)) => {
                    return self.spells(expression, |name| self.pattern_name_reaches(name))
                }
            },
            (false, PropertyKey::StaticIdentifier(id)) => Some(id.name.as_str()),
            (false, PropertyKey::StringLiteral(literal)) => literal.value.as_str(),
            (false, _) => None,
        };
        name.is_some_and(|name| self.pattern_name_reaches(name))
    }

    fn pattern_name_reaches(&self, name: &str) -> bool {
        REACHING_KEYS.contains(&name) || self.names_a_root(name)
    }

    fn pattern_read(&mut self, reaches: bool) {
        if reaches && !self.writes_only {
            self.aliased();
        }
    }

    pub(super) fn this_value(&self) -> This {
        self.frame().this
    }

    fn frame(&self) -> Frame {
        self.frames.last().copied().unwrap_or(Frame {
            strict: true,
            this: This::NotGlobal,
        })
    }

    fn within(&mut self, frame: Frame, walk: impl FnOnce(&mut Self)) {
        self.frames.push(frame);
        walk(self);
        self.frames.pop();
    }

    fn settled(&self) -> bool {
        self.verdict == Builtins::Written || (self.first_only && self.verdict != Builtins::Intact)
    }

    fn global_named(&self, id: &IdentifierReference<'_>, names: &[&str]) -> bool {
        names.contains(&id.name.as_str())
            && id
                .reference_id
                .get()
                .is_none_or(|reference| self.scoping.get_reference(reference).symbol_id().is_none())
    }

    fn is_global_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, &GLOBAL_OBJECTS),
            Expression::ThisExpression(_) => self.frame().this == This::Global,
            _ => false,
        }
    }

    fn is_builtin(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.global_named(id, self.reliance.roots),
            Expression::StaticMemberExpression(member) => {
                let name = member.property.name.as_str();
                (name == "prototype" && self.is_builtin(&member.object))
                    || (self.reliance.roots.contains(&name)
                        && self.is_global_object(&member.object))
            }
            _ => false,
        }
    }

    fn is_root(&self, expression: &Expression<'_>) -> bool {
        self.is_named_root(expression)
            || self.is_code_runner_name(expression)
            || self.is_reached(expression)
    }

    fn is_named_root(&self, expression: &Expression<'_>) -> bool {
        if self.is_builtin(expression) || self.is_global_object(expression) {
            return true;
        }
        matches!(
            expression.without_parentheses(),
            Expression::StaticMemberExpression(member)
                if GLOBAL_OBJECTS.contains(&member.property.name.as_str())
                    && self.is_global_object(&member.object)
        )
    }

    fn reaches_builtin(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.without_parentheses();
        loop {
            if self.is_builtin(current) {
                return true;
            }
            current = match current {
                Expression::StaticMemberExpression(member) => member.object.without_parentheses(),
                Expression::ComputedMemberExpression(member) => member.object.without_parentheses(),
                _ => return false,
            };
        }
    }

    fn global_key(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        let named =
            |name: &str| self.reliance.roots.contains(&name) || GLOBAL_OBJECTS.contains(&name);
        (self.is_global_object(object) || self.is_named_root(object) && !self.is_builtin(object))
            && key.is_none_or(named)
    }

    fn builtin_slot<'e>(&self, object: &'e Expression<'_>) -> Option<(&'e str, bool)> {
        match object.without_parentheses() {
            Expression::Identifier(id) if self.global_named(id, self.reliance.roots) => {
                Some((id.name.as_str(), false))
            }
            Expression::StaticMemberExpression(member)
                if member.property.name.as_str() == "prototype" =>
            {
                match member.object.without_parentheses() {
                    Expression::Identifier(id) if self.global_named(id, self.reliance.roots) => {
                        Some((id.name.as_str(), true))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn writes_member(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if let Some((root, on_prototype)) = self.builtin_slot(object) {
            return self.reliance.covers(root, on_prototype, key);
        }
        self.reaches_builtin(object) || self.global_key(object, key)
    }

    fn is_maybe_global_this(&self, expression: &Expression<'_>) -> bool {
        matches!(
            expression.without_parentheses(),
            Expression::ThisExpression(_)
        ) && self.this_value() == This::MaybeGlobal
    }

    fn is_reached_value(&self, expression: &Expression<'_>) -> bool {
        self.is_reached(expression) && !self.is_maybe_global_this(expression)
    }

    fn writes_reached(&self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if self.is_maybe_global_this(object) {
            return key.is_none_or(|key| self.names_a_root(key) || GLOBAL_OBJECTS.contains(&key));
        }
        if key.is_some_and(|key| self.names_a_root(key))
            || (self.is_reached(object) && self.relies_on_key(key))
        {
            return true;
        }
        let mut current = object.without_parentheses();
        while let Some((inner, _)) = member_of(current) {
            current = inner.without_parentheses();
            if self.is_reached_value(current) {
                return true;
            }
        }
        false
    }

    fn judge_write(&mut self, object: &Expression<'_>, key: Option<&str>) -> bool {
        if self.writes_member(object, key) {
            self.written();
            return true;
        }
        if self.writes_reached(object, key) {
            self.aliased();
        }
        false
    }

    fn member_write(&mut self, target: &Expression<'_>) -> bool {
        match target.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                self.judge_write(&member.object, Some(member.property.name.as_str()))
            }
            Expression::ComputedMemberExpression(member) => {
                self.judge_write(&member.object, literal_key(&member.expression))
            }
            _ => false,
        }
    }

    fn written(&mut self) {
        self.verdict = Builtins::Written;
    }

    fn reaches_promise(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.without_parentheses();
        loop {
            current = match current {
                Expression::Identifier(id) => return self.global_named(id, &[PROMISE]),
                Expression::StaticMemberExpression(member) => {
                    if member.property.name.as_str() == PROMISE
                        && self.is_global_object(&member.object)
                    {
                        return true;
                    }
                    member.object.without_parentheses()
                }
                Expression::ComputedMemberExpression(member) => {
                    if literal_key(&member.expression) == Some(PROMISE)
                        && self.is_global_object(&member.object)
                    {
                        return true;
                    }
                    member.object.without_parentheses()
                }
                _ => return false,
            };
        }
    }

    fn promise_target(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.global_named(id, &[PROMISE])
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.reaches_promise(&member.object)
                    || (member.property.name.as_str() == PROMISE
                        && self.is_global_object(&member.object))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.reaches_promise(&member.object)
                    || (literal_key(&member.expression) == Some(PROMISE)
                        && self.is_global_object(&member.object))
            }
            _ => false,
        }
    }

    fn defines_on_promise(&self, arguments: &[Argument<'_>]) -> bool {
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return false;
        };
        let key = parts.next().flatten().and_then(literal_key);
        self.reaches_promise(target) || (key == Some(PROMISE) && self.is_global_object(target))
    }

    fn aliased(&mut self) {
        if self.verdict == Builtins::Intact {
            self.verdict = Builtins::Aliased;
        }
    }

    fn is_definer(&self, callee: &Expression<'_>) -> bool {
        let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
            return false;
        };
        let name = member.property.name.as_str();
        let Expression::Identifier(root) = member.object.without_parentheses() else {
            return false;
        };
        DEFINERS
            .iter()
            .any(|(owner, method)| *method == name && self.global_named(root, &[owner]))
    }

    fn defines_on_builtin(&mut self, arguments: &[Argument<'_>]) -> bool {
        let mut parts = arguments.iter().map(Argument::as_expression);
        let Some(Some(target)) = parts.next() else {
            return true;
        };
        let key = parts.next().flatten().and_then(literal_key);
        self.judge_write(target, key)
    }

    fn legacy_definition(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
            return false;
        };
        if !LEGACY_DEFINERS.contains(&member.property.name.as_str()) {
            return false;
        }
        let key = arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(literal_key);
        self.judge_write(&member.object, key)
    }

    fn visit_unless_root(&mut self, expression: &Expression<'_>) {
        match expression.without_parentheses() {
            Expression::LogicalExpression(logical) if installs_if_missing(logical).is_none() => {
                self.visit_unless_root(&logical.left);
                self.visit_unless_root(&logical.right);
                return;
            }
            Expression::ConditionalExpression(conditional) => {
                self.visit_unless_root(&conditional.test);
                self.visit_unless_root(&conditional.consequent);
                self.visit_unless_root(&conditional.alternate);
                return;
            }
            Expression::SequenceExpression(sequence) => {
                sequence
                    .expressions
                    .iter()
                    .for_each(|expression| self.visit_unless_root(expression));
                return;
            }
            _ => {}
        }
        if self.writes_only || !self.is_root(expression) {
            self.visit_expression(expression);
        } else if self.is_reached(expression) {
            self.visit_reached(expression);
        }
    }

    fn visit_reached(&mut self, expression: &Expression<'_>) {
        if let Some(call) = call_of(expression) {
            if self.runs_code(&call.callee, &call.arguments) {
                self.aliased();
            }
            if !self.reads_prototypes(&call.callee) {
                self.visit_callee(&call.callee);
            }
            self.visit_arguments(&call.arguments);
            return;
        }
        let Some((object, _)) = member_of(expression) else {
            return;
        };
        self.visit_unless_root(object);
        if let Expression::ComputedMemberExpression(member) = expression.without_parentheses() {
            self.visit_expression(&member.expression);
        }
        if let Expression::ChainExpression(chain) = expression.without_parentheses() {
            if let Some(MemberExpression::ComputedMemberExpression(member)) =
                chain.expression.as_member_expression()
            {
                self.visit_expression(&member.expression);
            }
        }
    }

    fn visit_callee(&mut self, callee: &Expression<'_>) {
        match callee.without_parentheses() {
            Expression::ConditionalExpression(conditional) => {
                self.visit_expression(&conditional.test);
                self.visit_unless_root(&conditional.consequent);
                self.visit_unless_root(&conditional.alternate);
            }
            _ => self.visit_unless_root(callee),
        }
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        let reaches = self.pattern_key_reaches(it.computed, &it.key);
        self.pattern_read(reaches);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        let reaches = self.pattern_key_reaches(it.computed, &it.name);
        self.pattern_read(reaches);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let reaches = self.pattern_name_reaches(it.binding.name.as_str());
        self.pattern_read(reaches);
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.settled() {
            return;
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.settled() {
            return;
        }
        if !self.writes_only && (self.is_root(it) || self.reads_prototypes(it)) {
            self.aliased();
            if self.is_reached(it) {
                self.visit_reached(it);
            }
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.visit_unless_root(&it.object);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.visit_unless_root(&it.object);
        self.visit_expression(&it.expression);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.is_definer(&it.callee) && self.defines_on_promise(&it.arguments) {
            self.promise_written = true;
        }
        let defines = self.is_definer(&it.callee) && self.defines_on_builtin(&it.arguments);
        if defines || self.legacy_definition(&it.callee, &it.arguments) {
            self.written();
            return;
        }
        if self.runs_code(&it.callee, &it.arguments) {
            self.aliased();
        }
        if !self.reads_prototypes(&it.callee) {
            self.visit_callee(&it.callee);
        }
        self.visit_arguments(&it.arguments);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.visit_unless_root(&it.expression);
    }

    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        if let Some((last, discarded)) = it.expressions.split_last() {
            discarded
                .iter()
                .for_each(|expression| self.visit_unless_root(expression));
            self.visit_expression(last);
        }
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if self.runs_code(&it.callee, &it.arguments) {
            self.aliased();
        }
        self.visit_callee(&it.callee);
        self.visit_arguments(&it.arguments);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::Typeof if self.is_root(&it.argument) => {}
            UnaryOperator::LogicalNot | UnaryOperator::Void => self.visit_unless_root(&it.argument),
            UnaryOperator::Delete if self.member_write(&it.argument) => self.written(),
            UnaryOperator::Delete if self.reaches_promise(&it.argument) => {
                self.promise_written = true;
            }
            _ => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator.is_equality() {
            self.visit_unless_root(&it.left);
            self.visit_unless_root(&it.right);
            return;
        }
        self.visit_expression(&it.left);
        if matches!(it.operator, BinaryOperator::Instanceof | BinaryOperator::In) {
            self.visit_unless_root(&it.right);
        } else {
            self.visit_expression(&it.right);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let installs = matches!(
            it.operator,
            AssignmentOperator::LogicalOr | AssignmentOperator::LogicalNullish
        ) && it
            .left
            .as_simple_assignment_target()
            .is_some_and(|target| target_path(target, &mut Vec::new()));
        if installs {
            self.visit_expression(&it.right);
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        if let Some(value) = guarded_install(it) {
            self.visit_expression(value);
            return;
        }
        self.visit_unless_root(&it.test);
        self.visit_statement(&it.consequent);
        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.visit_unless_root(&it.test);
        self.visit_statement(&it.body);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.visit_statement(&it.body);
        self.visit_unless_root(&it.test);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.visit_unless_root(&it.test);
        self.visit_expression(&it.consequent);
        self.visit_expression(&it.alternate);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        if let Some(value) = installs_if_missing(it) {
            self.visit_expression(value);
            return;
        }
        if it.operator == LogicalOperator::And {
            self.visit_unless_root(&it.left);
        } else {
            self.visit_expression(&it.left);
        }
        self.visit_expression(&it.right);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if self.promise_target(it) {
            self.promise_written = true;
        }
        let written = match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.global_named(id, self.reliance.roots) || self.global_named(id, &GLOBAL_OBJECTS)
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                self.judge_write(&member.object, Some(member.property.name.as_str()))
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                self.judge_write(&member.object, literal_key(&member.expression))
            }
            _ => false,
        };
        if written {
            self.written();
            return;
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let strict = self.frame().strict || it.has_use_strict_directive();
        let frame = Frame {
            strict,
            this: if strict {
                This::NotGlobal
            } else {
                This::MaybeGlobal
            },
        };
        self.within(frame, |scan| walk_js::walk_function(scan, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let outer = self.frame();
        let frame = Frame {
            strict: outer.strict || it.has_use_strict_directive(),
            ..outer
        };
        self.within(frame, |scan| {
            walk_js::walk_arrow_function_expression(scan, it);
        });
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let frame = Frame {
            strict: true,
            ..self.frame()
        };
        self.within(frame, |scan| walk_js::walk_class(scan, it));
    }
}
