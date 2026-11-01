use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentTarget, AssignmentTargetPattern, BindingPattern,
    CallExpression, DoWhileStatement, Expression, ForInStatement, ForOfStatement, ForStatement,
    ForStatementLeft, Function, IdentifierReference, ImportExpression, MemberExpression, NewTarget,
    SimpleAssignmentTarget, Statement, Super, ThisExpression, UnaryExpression, VariableDeclarator,
    WhileStatement, WithStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use super::helpers::Helpers;
use super::names::{skipped, FORBIDDEN_KEYS, FORBIDDEN_NAMES, INTRINSICS};
use crate::stack;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Memo<'a> {
    Captured(&'a str),
    Named(&'a str),
}

pub(super) struct Rules<'s, 'a> {
    pub(super) decoder: &'s str,
    pub(super) machinery: &'s FxHashSet<&'a str>,
    pub(super) flags: &'s FxHashSet<&'a str>,
    pub(super) memo: Memo<'a>,
    pub(super) arrays: &'s FxHashSet<&'a str>,
    pub(super) helpers: &'s Helpers<'a>,
    pub(super) gates: &'s [Span],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Tail,
    Helper,
}

pub(super) struct Scanner<'s, 'a> {
    rules: &'s Rules<'s, 'a>,
    mode: Mode,
    declared: FxHashSet<&'a str>,
    pub(super) failed: bool,
    pub(super) reached: FxHashSet<Span>,
    pub(super) reads: Vec<&'a str>,
    pub(super) writes: Vec<&'a str>,
    pub(super) assigned: FxHashSet<&'a str>,
    pub(super) inits: FxHashMap<&'a str, u32>,
    depth: usize,
    loops: usize,
    writing: bool,
}

impl<'s, 'a> Scanner<'s, 'a> {
    pub(super) fn new(rules: &'s Rules<'s, 'a>, mode: Mode, declared: FxHashSet<&'a str>) -> Self {
        Self {
            rules,
            mode,
            declared,
            failed: false,
            reached: FxHashSet::default(),
            reads: Vec::new(),
            writes: Vec::new(),
            assigned: FxHashSet::default(),
            inits: FxHashMap::default(),
            depth: 0,
            loops: 0,
            writing: false,
        }
    }

    fn skips(&self, span: Span) -> bool {
        self.mode == Mode::Tail && skipped(self.rules.gates, span)
    }

    fn sensitive(&self, name: &str) -> bool {
        let memo_root = matches!(self.rules.memo, Memo::Captured(root) if root == name);
        memo_root || self.rules.machinery.contains(name) || self.rules.arrays.contains(name)
    }

    fn reference(&mut self, name: &'a str) {
        if FORBIDDEN_NAMES.contains(&name) || self.sensitive(name) {
            self.failed = true;
            return;
        }
        if self.declared.contains(name) {
            return;
        }
        if self.mode == Mode::Helper {
            if let Some(span) = self.rules.helpers.bindings.get(name) {
                self.reached.insert(*span);
                return;
            }
        }
        if self.writing || !INTRINSICS.contains(&name) {
            self.failed = true;
        }
    }

    fn memo_key(&self, member: &MemberExpression<'a>) -> Option<Option<&'a str>> {
        let MemberExpression::ComputedMemberExpression(node) = member else {
            return None;
        };
        let container = match (self.rules.memo, node.object.without_parentheses()) {
            (Memo::Captured(root), Expression::Identifier(id)) => id.name == root,
            (Memo::Named(segment), other) => other.as_member_expression().is_some_and(|inner| {
                matches!(inner.object(), Expression::Identifier(id) if id.name == self.rules.decoder)
                    && inner.static_property_name() == Some(segment)
            }),
            _ => false,
        };
        if !container {
            return None;
        }
        match node.expression.without_parentheses() {
            Expression::Identifier(key) => Some(Some(key.name.as_str())),
            _ => Some(None),
        }
    }

    fn memo_access(&mut self, key: Option<&'a str>) {
        let declared = key.is_some_and(|key| self.declared.contains(key));
        let (Some(key), true, Mode::Tail, 0, 0) =
            (key, declared, self.mode, self.depth, self.loops)
        else {
            self.failed = true;
            return;
        };
        if self.writing {
            self.writes.push(key);
        } else {
            self.reads.push(key);
        }
    }

    fn machinery_member(&mut self, member: &MemberExpression<'a>) -> bool {
        let Expression::Identifier(object) = member.object().without_parentheses() else {
            return false;
        };
        let name = object.name.as_str();
        if self.rules.arrays.contains(name) && !self.writing {
            if let MemberExpression::ComputedMemberExpression(node) = member {
                self.visit_expression(&node.expression);
            }
            return true;
        }
        if !self.rules.machinery.contains(name) {
            return false;
        }
        let flag = member
            .static_property_name()
            .is_some_and(|key| self.rules.flags.contains(key));
        if self.mode != Mode::Tail || name != self.rules.decoder || !flag || self.writing {
            self.failed = true;
        }
        true
    }

    fn installed_call(&mut self, call: &CallExpression<'a>) -> bool {
        let Some(member) = call.callee.as_member_expression() else {
            return false;
        };
        let on_decoder =
            matches!(member.object(), Expression::Identifier(id) if id.name == self.rules.decoder);
        let Some(key) = member.static_property_name().filter(|_| on_decoder) else {
            return false;
        };
        match self.rules.helpers.installed.get(key) {
            Some(spans)
                if self.mode == Mode::Tail && !self.rules.helpers.poisoned.contains(key) =>
            {
                self.reached.extend(spans.iter().copied());
            }
            _ => self.failed = true,
        }
        true
    }

    fn with_writing(&mut self, writing: bool, body: impl FnOnce(&mut Self)) {
        let previous = self.writing;
        self.writing = writing;
        body(self);
        self.writing = previous;
    }

    fn nested(&mut self, loops: bool, body: impl FnOnce(&mut Self)) {
        let (depth, count) = if loops { (0, 1) } else { (1, 0) };
        self.depth += depth;
        self.loops += count;
        body(self);
        self.depth -= depth;
        self.loops -= count;
    }
}

impl<'a> VisitJs<'a> for Scanner<'_, 'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.skips(it.span()) {
            stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.skips(it.span()) {
            stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.reference(it.name.as_str());
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if it
            .static_property_name()
            .is_some_and(|key| FORBIDDEN_KEYS.contains(&key))
        {
            self.failed = true;
            return;
        }
        if let Some(key) = self.memo_key(it) {
            self.memo_access(key);
            return;
        }
        if self.machinery_member(it) {
            return;
        }
        self.with_writing(false, |scanner| {
            walk_js::walk_member_expression(scanner, it)
        });
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.assigned.insert(id.name.as_str());
        }
        self.with_writing(true, |scanner| {
            walk_js::walk_simple_assignment_target(scanner, it)
        });
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let delete = it.operator == UnaryOperator::Delete;
        self.with_writing(delete, |scanner| {
            walk_js::walk_unary_expression(scanner, it)
        });
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (oxc_ast::ast::BindingPattern::BindingIdentifier(id), Some(_)) = (&it.id, &it.init) {
            *self.inits.entry(id.name.as_str()).or_default() += 1;
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.installed_call(it) {
            for argument in &it.arguments {
                self.visit_argument(argument);
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.nested(false, |scanner| walk_js::walk_function(scanner, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nested(false, |scanner| {
            walk_js::walk_arrow_function_expression(scanner, it)
        });
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.nested(true, |scanner| walk_js::walk_for_statement(scanner, it));
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.head(&it.left);
        self.nested(true, |scanner| walk_js::walk_for_in_statement(scanner, it));
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.head(&it.left);
        self.nested(true, |scanner| walk_js::walk_for_of_statement(scanner, it));
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.nested(true, |scanner| walk_js::walk_while_statement(scanner, it));
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.nested(true, |scanner| {
            walk_js::walk_do_while_statement(scanner, it)
        });
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.failed = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.failed = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.failed = true;
    }

    fn visit_with_statement(&mut self, _it: &WithStatement<'a>) {
        self.failed = true;
    }

    fn visit_import_expression(&mut self, _it: &ImportExpression<'a>) {
        self.failed = true;
    }

    fn visit_assignment_target_pattern(&mut self, _it: &AssignmentTargetPattern<'a>) {
        self.failed = true;
    }
}

impl<'a> Scanner<'_, 'a> {
    fn head(&mut self, left: &ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            for declarator in &declaration.declarations {
                if let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = &declarator.id {
                    self.assigned.insert(id.name.as_str());
                }
            }
        }
    }
}
