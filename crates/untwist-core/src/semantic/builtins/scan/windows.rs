use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::keys::{literal_text, ConstKeys};
use super::reach::{call_of, key_of, member_of, Key, CODE_RUNNERS, TIMERS};
use super::{Scan, This, GLOBAL_OBJECTS};

const WINDOW_CALLS: [&str; 2] = ["open", "composedPath"];

const WINDOW_ROUTES: [&str; 9] = [
    "defaultView",
    "contentWindow",
    "currentTarget",
    "target",
    "srcElement",
    "view",
    "source",
    "opener",
    "ownerGlobal",
];

pub(super) type WindowLocals = FxHashSet<SymbolId>;

fn is_route(name: &str) -> bool {
    WINDOW_ROUTES.contains(&name) || GLOBAL_OBJECTS.contains(&name)
}

fn routes_through(key: Key<'_>) -> bool {
    matches!(key, Key::Name(name) if is_route(name))
}

fn property_route(key: &PropertyKey<'_>) -> bool {
    key.static_name()
        .is_some_and(|name| name.as_str().is_some_and(is_route))
}

fn calls_for_a_window(expression: &Expression<'_>) -> bool {
    let Some(call) = call_of(expression) else {
        return false;
    };
    match call.callee.without_parentheses() {
        Expression::Identifier(id) => WINDOW_CALLS.contains(&id.name.as_str()),
        other => member_of(other).is_some_and(|(_, key)| match key {
            Key::Name(name) => WINDOW_CALLS.contains(&name),
            Key::Other => false,
            Key::Unknown(key) => {
                literal_text(key).is_none_or(|name| WINDOW_CALLS.contains(&name.as_str()))
            }
        }),
    }
}

pub(super) fn window_locals(
    program: &Program<'_>,
    scoping: &Scoping,
    consts: &ConstKeys,
) -> WindowLocals {
    let mut found = Windows {
        scoping,
        consts,
        locals: FxHashSet::default(),
        grew: true,
    };
    while found.grew {
        found.grew = false;
        found.visit_program(program);
    }
    found.locals
}

struct Windows<'s> {
    scoping: &'s Scoping,
    consts: &'s ConstKeys,
    locals: WindowLocals,
    grew: bool,
}

impl Windows<'_> {
    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn holds_window(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => match self.symbol_of(id) {
                Some(symbol) => self.locals.contains(&symbol),
                None => is_route(id.name.as_str()),
            },
            Expression::ConditionalExpression(conditional) => {
                self.holds_window(&conditional.consequent)
                    || self.holds_window(&conditional.alternate)
            }
            Expression::LogicalExpression(logical) => {
                self.holds_window(&logical.left) || self.holds_window(&logical.right)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .is_some_and(|last| self.holds_window(last)),
            Expression::AssignmentExpression(assignment) => self.holds_window(&assignment.right),
            other if calls_for_a_window(other) => true,
            other => member_of(other).is_some_and(|(object, key)| match key {
                Key::Other => self.holds_window(object),
                Key::Unknown(key) => self.key_text(key).is_some_and(|name| is_route(&name)),
                Key::Name(_) => routes_through(key),
            }),
        }
    }

    fn key_text(&self, key: &Expression<'_>) -> Option<String> {
        if let Some(text) = literal_text(key) {
            return Some(text);
        }
        let Expression::Identifier(id) = key.without_parentheses() else {
            return None;
        };
        self.consts.get(&self.symbol_of(id)?).cloned()
    }

    fn add(&mut self, symbol: Option<SymbolId>) {
        if let Some(symbol) = symbol {
            if self.locals.insert(symbol) {
                self.grew = true;
            }
        }
    }

    fn add_binding(&mut self, pattern: &BindingPattern<'_>) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => self.add(id.symbol_id.get()),
            BindingPattern::AssignmentPattern(assignment) => self.add_binding(&assignment.left),
            _ => {}
        }
    }

    fn add_target(&mut self, target: &AssignmentTargetMaybeDefault<'_>) {
        let target = match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with) => &with.binding,
            other => match other.as_assignment_target() {
                Some(target) => target,
                None => return,
            },
        };
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
            self.add(self.symbol_of(id));
        }
    }
}

impl<'a> VisitJs<'a> for Windows<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if self.holds_window(init) {
                self.add(id.symbol_id.get());
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if self.holds_window(&it.right) {
                self.add(self.symbol_of(id));
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if property_route(&it.key) {
            self.add_binding(&it.value);
        }
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        if property_route(&it.name) {
            self.add_target(&it.binding);
        }
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if is_route(it.binding.name.as_str()) {
            self.add(self.symbol_of(&it.binding));
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }
}

impl Scan<'_> {
    fn may_be_window(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => {
                self.global_named(id, &GLOBAL_OBJECTS)
                    || id
                        .reference_id
                        .get()
                        .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                        .map_or(is_route(id.name.as_str()), |symbol| {
                            self.window_local(symbol)
                        })
            }
            Expression::ThisExpression(_) => self.this_value() != This::NotGlobal,
            Expression::ConditionalExpression(conditional) => {
                self.may_be_window(&conditional.consequent)
                    || self.may_be_window(&conditional.alternate)
            }
            Expression::LogicalExpression(logical) => {
                self.may_be_window(&logical.left) || self.may_be_window(&logical.right)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .is_some_and(|last| self.may_be_window(last)),
            Expression::AssignmentExpression(assignment) => self.may_be_window(&assignment.right),
            other if calls_for_a_window(other) => true,
            other => member_of(other).is_some_and(|(object, key)| match key {
                Key::Other => self.may_be_window(object),
                Key::Unknown(key) => self.known_key(key).is_some_and(|name| is_route(&name)),
                Key::Name(_) => routes_through(key),
            }),
        }
    }

    pub(super) fn window_value(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => {
                !self.global_named(id, &GLOBAL_OBJECTS) && self.may_be_window(expression)
            }
            Expression::ThisExpression(_)
            | Expression::ConditionalExpression(_)
            | Expression::LogicalExpression(_)
            | Expression::SequenceExpression(_)
            | Expression::AssignmentExpression(_) => false,
            other => self.may_be_window(other),
        }
    }

    fn is_window_slot(&self, name: &str) -> bool {
        self.is_builtin_root_name(name)
            || GLOBAL_OBJECTS.contains(&name)
            || CODE_RUNNERS.contains(&name)
            || TIMERS.contains(&name)
    }

    pub(super) fn writes_window_key(
        &self,
        object: &Expression<'_>,
        key: Option<&Expression<'_>>,
    ) -> bool {
        let slot = key.is_none_or(|key| match self.sharpen(key_of(key)) {
            Key::Other => false,
            Key::Name(name) => self.is_window_slot(name),
            Key::Unknown(key) => self
                .known_key(key)
                .is_none_or(|name| self.is_window_slot(&name)),
        });
        self.judged_write(object, slot)
    }

    pub(super) fn writes_window_name(&self, object: &Expression<'_>, name: &str) -> bool {
        self.judged_write(object, self.is_window_slot(name))
    }

    fn judged_write(&self, object: &Expression<'_>, slot: bool) -> bool {
        let window = self.may_be_window(object);
        if slot && !window {
            self.window_writes.set(true);
        }
        slot && window
    }

    pub(super) fn settle_window_flows(&mut self) {
        if self.window_escaped && self.window_writes.get() {
            self.not_allowed();
        }
    }
}
