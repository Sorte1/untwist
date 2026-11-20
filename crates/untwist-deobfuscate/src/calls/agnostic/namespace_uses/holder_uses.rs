use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use std::borrow::Cow;

use super::{member_address, same, str_const, Walker, KIND_DEPTH};

const PROTO_KEYS: [&str; 6] = [
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "constructor",
];

#[derive(Default)]
pub(super) struct HolderUses<'a> {
    pub(super) escaped: bool,
    pub(super) protos: Vec<Option<&'a Expression<'a>>>,
    pub(super) written: Vec<Cow<'a, str>>,
    pub(super) called: Vec<Cow<'a, str>>,
}

enum Use<'a> {
    Plain,
    Escape,
    Proto(Option<&'a Expression<'a>>),
    Written(Cow<'a, str>),
    Called(Cow<'a, str>),
}

fn proto_key(key: &str) -> bool {
    PROTO_KEYS.contains(&key)
}

pub(super) const INSTANCEOF_BUILTIN: &str = "@instanceof";

const INSTANCE_CONSTRUCTORS: [&str; 16] = [
    "Object", "Function", "Array", "String", "Number", "Boolean", "Symbol", "BigInt", "RegExp",
    "Date", "Error", "Map", "Set", "WeakMap", "WeakSet", "Promise",
];

pub(super) fn harmless_binary(operator: BinaryOperator, right: bool) -> bool {
    matches!(
        operator,
        BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
    ) || (operator == BinaryOperator::In && right)
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn collect_holder_use(&mut self, id: &IdentifierReference<'a>) {
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        let read = id
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_read());
        if !read {
            return;
        }
        let found = self.holder_use(self.stack.len() - 1);
        let uses = self.holder_uses.entry(symbol).or_default();
        match found {
            Use::Plain => {}
            Use::Escape => uses.escaped = true,
            Use::Proto(value) => uses.protos.push(value),
            Use::Written(key) => uses.written.push(key),
            Use::Called(key) => uses.called.push(key),
        }
    }

    fn holder_use(&self, at: usize) -> Use<'a> {
        let Some((up, child)) = self.parent(at) else {
            return Use::Escape;
        };
        match self.stack[up] {
            AstKind::StaticMemberExpression(m) if same(&m.object, child) => {
                self.member_use_of(up, Some(Cow::Borrowed(m.property.name.as_str())))
            }
            AstKind::ComputedMemberExpression(m) if same(&m.object, child) => {
                let key = str_const(&m.expression, self.resolve, KIND_DEPTH).map(Cow::Owned);
                self.member_use_of(up, key)
            }
            AstKind::UnaryExpression(u)
                if matches!(
                    u.operator,
                    UnaryOperator::Typeof | UnaryOperator::Void | UnaryOperator::LogicalNot
                ) =>
            {
                Use::Plain
            }
            AstKind::BinaryExpression(b) if harmless_binary(b.operator, same(&b.right, child)) => {
                Use::Plain
            }
            AstKind::BinaryExpression(b)
                if b.operator == BinaryOperator::Instanceof
                    && same(&b.left, child)
                    && self.builtin_constructor(&b.right) =>
            {
                Use::Called(Cow::Borrowed(INSTANCEOF_BUILTIN))
            }
            AstKind::ExpressionStatement(_) => Use::Plain,
            AstKind::IfStatement(s) if same(&s.test, child) => Use::Plain,
            AstKind::ConditionalExpression(c) if same(&c.test, child) => Use::Plain,
            AstKind::WhileStatement(s) if same(&s.test, child) => Use::Plain,
            AstKind::DoWhileStatement(s) if same(&s.test, child) => Use::Plain,
            AstKind::ForInStatement(f) if same(&f.right, child) => Use::Plain,
            _ => Use::Escape,
        }
    }

    fn member_use_of(&self, member_at: usize, key: Option<Cow<'a, str>>) -> Use<'a> {
        let Some((up, child)) = self.parent(member_at) else {
            return Use::Plain;
        };
        let proto = key.as_deref().is_none_or(proto_key);
        match self.stack[up] {
            AstKind::CallExpression(call) if same(&call.callee, child) => match key {
                Some(key) if !proto => Use::Called(key),
                _ => Use::Escape,
            },
            AstKind::TaggedTemplateExpression(tagged) if same(&tagged.tag, child) => Use::Escape,
            AstKind::AssignmentExpression(assign) if self.targets(&assign.left, child) => match key
            {
                Some(key) if !proto => Use::Written(key),
                _ if numeric_assignment(assign.operator) => Use::Plain,
                _ => Use::Proto(Some(&assign.right)),
            },
            AstKind::UpdateExpression(_) => Use::Plain,
            AstKind::ForInStatement(f) if !same(&f.right, child) => self.target_use(key),
            AstKind::ForOfStatement(f) if !same(&f.right, child) => self.target_use(key),
            AstKind::AssignmentTargetWithDefault(_)
            | AstKind::ArrayAssignmentTarget(_)
            | AstKind::AssignmentTargetPropertyProperty(_)
            | AstKind::AssignmentTargetRest(_) => self.target_use(key),
            _ => Use::Plain,
        }
    }

    fn builtin_constructor(&self, expr: &Expression<'a>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if self.symbol(id).is_none()
                && INSTANCE_CONSTRUCTORS.contains(&id.name.as_str())
                && !self.free_written(id.name.as_str()))
    }

    fn targets(&self, left: &AssignmentTarget<'a>, child: oxc_allocator::Address) -> bool {
        left.as_member_expression()
            .is_some_and(|member| member_address(member) == child)
    }

    fn target_use(&self, key: Option<Cow<'a, str>>) -> Use<'a> {
        match key {
            Some(key) if !proto_key(&key) => Use::Written(key),
            _ => Use::Proto(None),
        }
    }
}

fn numeric_assignment(operator: AssignmentOperator) -> bool {
    !matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalNullish
    )
}
