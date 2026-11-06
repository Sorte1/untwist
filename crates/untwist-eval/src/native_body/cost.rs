use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, BinaryExpression, CallExpression, Expression,
    ForStatementInit, MemberExpression, SimpleAssignmentTarget, Statement, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};

use crate::ops::string_key;
use crate::value::{js_string_from_literal, PropKey};

const LENGTH_CHARGE_DIVISOR: u64 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event<'a> {
    Read(&'a str),
    Get(&'a str),
    Call,
    Concat,
}

pub(super) struct Walk<'a, 'p> {
    pub(super) total: u64,
    path: &'p [bool],
    taken: usize,
    events: Vec<(u64, Event<'a>)>,
}

pub(super) fn length_charge(len: usize) -> u64 {
    1 + len as u64 / LENGTH_CHARGE_DIVISOR
}

pub(super) fn concat_charge(left: usize, right: usize) -> u64 {
    let compared = left.min(right) as u64 / LENGTH_CHARGE_DIVISOR;
    1 + compared + length_charge(left + right)
}

pub(super) fn scan_charge(haystack: usize, needle: usize) -> u64 {
    let work = (haystack as u64).saturating_mul(needle.max(1) as u64);
    1 + work / LENGTH_CHARGE_DIVISOR
}

fn is_text(expression: &Expression<'_>) -> bool {
    matches!(expression, Expression::StringLiteral(_))
}

fn key_length(key: &PropKey) -> usize {
    match key {
        PropKey::Name(text) => text.len(),
        PropKey::Index(index) => index
            .checked_ilog10()
            .map_or(1, |digits| digits as usize + 1),
    }
}

impl<'a, 'p> Walk<'a, 'p> {
    pub(super) fn new(path: &'p [bool]) -> Self {
        Self {
            total: 0,
            path,
            taken: 0,
            events: Vec::new(),
        }
    }

    pub(super) fn finished(self) -> Option<Self> {
        (self.taken == self.path.len()).then_some(self)
    }

    pub(super) fn at(&self, event: Event<'_>) -> Option<u64> {
        let mut found = self.events.iter().filter(|(_, seen)| *seen == event);
        let (cost, _) = found.next()?;
        found.next().is_none().then_some(*cost)
    }

    pub(super) fn first(&self, event: Event<'_>) -> Option<u64> {
        self.events
            .iter()
            .find(|(_, seen)| *seen == event)
            .map(|(cost, _)| *cost)
    }

    pub(super) fn count(&self, event: Event<'_>) -> usize {
        self.events
            .iter()
            .filter(|(_, seen)| *seen == event)
            .count()
    }

    fn add(&mut self, steps: u64) {
        self.total += steps;
    }

    fn note(&mut self, event: Event<'a>) {
        self.events.push((self.total, event));
    }

    fn branch(&mut self) -> Option<bool> {
        let taken = *self.path.get(self.taken)?;
        self.taken += 1;
        Some(taken)
    }

    pub(super) fn statement(&mut self, statement: &'a Statement<'a>) -> Option<()> {
        self.add(1);
        match statement {
            Statement::ExpressionStatement(node) => self.expression(&node.expression),
            Statement::EmptyStatement(_) => Some(()),
            Statement::VariableDeclaration(declaration) => self.declaration(declaration),
            Statement::BlockStatement(block) => {
                let lexical = block.body.iter().any(|inner| {
                    matches!(inner, Statement::VariableDeclaration(declaration) if declaration.kind != VariableDeclarationKind::Var)
                });
                if lexical {
                    return None;
                }
                block
                    .body
                    .iter()
                    .try_for_each(|inner| self.statement(inner))
            }
            _ => None,
        }
    }

    pub(super) fn declaration(&mut self, declaration: &'a VariableDeclaration<'a>) -> Option<()> {
        declaration
            .declarations
            .iter()
            .filter_map(|declarator| declarator.init.as_ref())
            .try_for_each(|init| self.expression(init))
    }

    pub(super) fn for_init(&mut self, init: Option<&'a ForStatementInit<'a>>) -> Option<()> {
        match init {
            None => Some(()),
            Some(ForStatementInit::VariableDeclaration(declaration)) => {
                self.declaration(declaration)
            }
            Some(other) => self.expression(other.as_expression()?),
        }
    }

    pub(super) fn expression(&mut self, expression: &'a Expression<'a>) -> Option<()> {
        if let Expression::Identifier(id) = expression {
            self.note(Event::Read(id.name.as_str()));
            self.add(1);
            return Some(());
        }
        self.add(1);
        match expression {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => Some(()),
            Expression::ParenthesizedExpression(node) => self.expression(&node.expression),
            Expression::UnaryExpression(node) => {
                if node.operator == UnaryOperator::Delete || is_text(&node.argument) {
                    return None;
                }
                self.expression(&node.argument)?;
                self.add(1);
                Some(())
            }
            Expression::BinaryExpression(node) => self.binary(node),
            _ => self.compound(expression),
        }
    }

    fn binary(&mut self, node: &'a BinaryExpression<'a>) -> Option<()> {
        if matches!(
            node.operator,
            BinaryOperator::In | BinaryOperator::Instanceof
        ) {
            return None;
        }
        let concat = node.operator == BinaryOperator::Addition
            && (is_text(&node.left) || is_text(&node.right));
        if !concat && (is_text(&node.left) || is_text(&node.right)) {
            return None;
        }
        self.expression(&node.left)?;
        self.expression(&node.right)?;
        match node.operator {
            _ if concat => self.note(Event::Concat),
            BinaryOperator::Equality | BinaryOperator::Inequality => self.add(2),
            _ => self.add(1),
        }
        Some(())
    }

    fn compound(&mut self, expression: &'a Expression<'a>) -> Option<()> {
        match expression {
            Expression::LogicalExpression(node) => {
                self.expression(&node.left)?;
                match self.branch()? {
                    true => self.expression(&node.right),
                    false => Some(()),
                }
            }
            Expression::ConditionalExpression(node) => {
                self.expression(&node.test)?;
                match self.branch()? {
                    true => self.expression(&node.consequent),
                    false => self.expression(&node.alternate),
                }
            }
            Expression::SequenceExpression(node) => node
                .expressions
                .iter()
                .try_for_each(|part| self.expression(part)),
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                self.member_value(expression.as_member_expression()?)
            }
            Expression::CallExpression(call) => self.call_link(call),
            Expression::AssignmentExpression(node) => self.assignment(node),
            Expression::UpdateExpression(node) => match &node.argument {
                SimpleAssignmentTarget::AssignmentTargetIdentifier(_) => Some(()),
                _ => None,
            },
            _ => None,
        }
    }

    fn key(&mut self, member: &'a MemberExpression<'a>) -> Option<Option<&'a str>> {
        match member {
            MemberExpression::StaticMemberExpression(node) => {
                Some(Some(node.property.name.as_str()))
            }
            MemberExpression::ComputedMemberExpression(node) => match &node.expression {
                Expression::StringLiteral(literal) => {
                    let key = string_key(js_string_from_literal(literal));
                    self.add(1 + length_charge(key_length(&key)));
                    Some(literal.value.as_str())
                }
                other => {
                    self.expression(other)?;
                    Some(None)
                }
            },
            MemberExpression::PrivateFieldExpression(_) => None,
        }
    }

    fn get(&mut self, key: Option<&'a str>) {
        if let Some(key) = key {
            self.note(Event::Get(key));
        }
        self.add(1);
    }

    fn member_value(&mut self, member: &'a MemberExpression<'a>) -> Option<()> {
        if member.optional() {
            return None;
        }
        self.expression(member.object())?;
        let key = self.key(member)?;
        self.get(key);
        Some(())
    }

    fn member_link(&mut self, member: &'a MemberExpression<'a>) -> Option<Option<&'a str>> {
        if member.optional() {
            return None;
        }
        self.add(1);
        self.chain_base(member.object())?;
        self.key(member)
    }

    fn chain_base(&mut self, base: &'a Expression<'a>) -> Option<()> {
        match base {
            Expression::StaticMemberExpression(_) => {
                let key = self.member_link(base.as_member_expression()?)?;
                self.get(key);
                Some(())
            }
            Expression::ComputedMemberExpression(_) => None,
            Expression::CallExpression(call) => self.call_link(call),
            other => self.expression(other),
        }
    }

    fn call_link(&mut self, call: &'a CallExpression<'a>) -> Option<()> {
        if call.optional {
            return None;
        }
        match call.callee.without_parentheses().as_member_expression() {
            Some(member) => {
                let key = self.member_link(member)?;
                self.get(key);
            }
            None => self.expression(&call.callee)?,
        }
        for argument in &call.arguments {
            self.expression(argument.as_expression()?)?;
        }
        self.note(Event::Call);
        Some(())
    }

    fn assignment(&mut self, node: &'a AssignmentExpression<'a>) -> Option<()> {
        let compound = match node.operator {
            AssignmentOperator::Assign => None,
            AssignmentOperator::Addition => Some(true),
            AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalNullish => return None,
            _ => Some(false),
        };
        match &node.left {
            AssignmentTarget::AssignmentTargetIdentifier(_) => {}
            AssignmentTarget::ComputedMemberExpression(member) if compound.is_none() => {
                self.expression(&member.object)?;
                match &member.expression {
                    Expression::StringLiteral(_) => return None,
                    key => self.expression(key)?,
                }
            }
            _ => return None,
        }
        self.expression(&node.right)?;
        match compound {
            Some(true) => self.note(Event::Concat),
            Some(false) => self.add(1),
            None => {}
        }
        Some(())
    }
}
