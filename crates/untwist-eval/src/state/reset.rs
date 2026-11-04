use oxc_ast::ast::{
    AssignmentExpression, AssignmentOperator, AssignmentTarget, AssignmentTargetPattern,
    BindingIdentifier, BindingPattern, Expression, Function, IdentifierReference, LogicalOperator,
    MemberExpression, SimpleAssignmentTarget, Statement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;

use super::collect::{Collected, FinalKey, MemberPeel, TriggerKind};
use super::recipe::{fold_constant, Prim};
use super::{span_contains, Owner};
use crate::stack;

pub(super) struct ResetGate<'a> {
    pub(super) owner: usize,
    pub(super) region: Span,
    pub(super) writes: [Span; 2],
    pub(super) memo_key: &'a str,
}

struct Candidate<'n, 'a> {
    index: usize,
    region: Span,
    test: &'n Expression<'a>,
    reset: &'n AssignmentExpression<'a>,
    store: &'n AssignmentExpression<'a>,
}

struct Keys<'a> {
    decoder: &'a str,
    value: &'a str,
    array: &'a str,
    state: &'a str,
    memo: &'a str,
}

fn as_assignment<'n, 'a>(expr: &'n Expression<'a>) -> Option<&'n AssignmentExpression<'a>> {
    match expr.without_parentheses() {
        Expression::AssignmentExpression(node) if node.operator == AssignmentOperator::Assign => {
            Some(node)
        }
        _ => None,
    }
}

type Pair<'n, 'a> = (&'n AssignmentExpression<'a>, &'n AssignmentExpression<'a>);

fn pair_of<'n, 'a>(exprs: &[&'n Expression<'a>]) -> Option<Pair<'n, 'a>> {
    let &[reset, store] = exprs else {
        return None;
    };
    Some((as_assignment(reset)?, as_assignment(store)?))
}

fn sequence_pair<'n, 'a>(expr: &'n Expression<'a>) -> Option<Pair<'n, 'a>> {
    let Expression::SequenceExpression(sequence) = expr.without_parentheses() else {
        return None;
    };
    let exprs: Vec<&Expression<'a>> = sequence.expressions.iter().collect();
    pair_of(&exprs)
}

fn consequent_pair<'n, 'a>(statement: &'n Statement<'a>) -> Option<Pair<'n, 'a>> {
    match statement {
        Statement::ExpressionStatement(node) => sequence_pair(&node.expression),
        Statement::BlockStatement(block) => {
            let exprs: Option<Vec<&Expression<'a>>> = block
                .body
                .iter()
                .map(|statement| match statement {
                    Statement::ExpressionStatement(node) => Some(&node.expression),
                    _ => None,
                })
                .collect();
            match exprs?.as_slice() {
                &[single] => sequence_pair(single),
                exprs => pair_of(exprs),
            }
        }
        _ => None,
    }
}

fn candidate<'n, 'a>(index: usize, statement: &'n Statement<'a>) -> Option<Candidate<'n, 'a>> {
    let (test, region, (reset, store)) = match statement {
        Statement::ExpressionStatement(node) => match node.expression.without_parentheses() {
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                let pair = sequence_pair(&logical.right)?;
                (&logical.left, logical.right.span(), pair)
            }
            _ => return None,
        },
        Statement::IfStatement(node) if node.alternate.is_none() => {
            let pair = consequent_pair(&node.consequent)?;
            (&node.test, node.consequent.span(), pair)
        }
        _ => return None,
    };
    Some(Candidate {
        index,
        region,
        test,
        reset,
        store,
    })
}

fn own_key<'a>(member: &MemberExpression<'a>, decoder: &str) -> Option<&'a str> {
    match member.object().without_parentheses() {
        Expression::Identifier(id) if id.name.as_str() == decoder => member.static_property_name(),
        _ => None,
    }
}

fn identifier<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn state_test<'a>(test: &Expression<'a>, decoder: &str) -> Option<(&'a str, &'a str)> {
    let Expression::BinaryExpression(node) = test.without_parentheses() else {
        return None;
    };
    if node.operator != BinaryOperator::StrictInequality {
        return None;
    }
    let side = |member: &Expression<'a>, value: &Expression<'a>| {
        let key = own_key(
            member.without_parentheses().as_member_expression()?,
            decoder,
        )?;
        Some((key, identifier(value)?))
    };
    side(&node.left, &node.right).or_else(|| side(&node.right, &node.left))
}

fn target_key<'a>(assignment: &AssignmentExpression<'a>, decoder: &str) -> Option<&'a str> {
    own_key(assignment.left.as_member_expression()?, decoder)
}

fn is_empty_object(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::ObjectExpression(object) if object.properties.is_empty())
}

fn single_declarator<'n, 'a>(
    statement: &'n Statement<'a>,
) -> Option<(&'a str, &'n Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = &declaration.declarations[..] else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.name.as_str(), declarator.init.as_ref()?))
}

fn first_element_of<'a>(init: &Expression<'a>) -> Option<&'a str> {
    let Expression::ComputedMemberExpression(member) = init.without_parentheses() else {
        return None;
    };
    let index = fold_constant(&member.expression, &[])?;
    if !matches!(index, Prim::Num(number) if number == 0.0) {
        return None;
    }
    identifier(&member.object)
}

fn calls_array(init: &Expression<'_>, array: &str) -> bool {
    matches!(init.without_parentheses(), Expression::CallExpression(call)
        if call.arguments.is_empty() && identifier(&call.callee) == Some(array))
}

fn declared_before<'n, 'a>(
    statements: &'n [Statement<'a>],
    name: &str,
) -> Option<(usize, &'n Expression<'a>)> {
    statements
        .iter()
        .enumerate()
        .find_map(|(index, statement)| {
            let (declared, init) = single_declarator(statement)?;
            (declared == name).then_some((index, init))
        })
}

fn keys_of<'a>(
    statements: &[Statement<'a>],
    found: &Candidate<'_, 'a>,
    decoder: &'a str,
    string_array: &str,
) -> Option<Keys<'a>> {
    let (state, value) = state_test(found.test, decoder)?;
    let memo = target_key(found.reset, decoder)?;
    let stored = target_key(found.store, decoder)?;
    let shaped = memo != state
        && stored == state
        && is_empty_object(&found.reset.right)
        && identifier(&found.store.right) == Some(value);
    if !shaped {
        return None;
    }
    let (value_at, value_init) = declared_before(&statements[..found.index], value)?;
    let array = first_element_of(value_init)?;
    let (_, array_init) = declared_before(&statements[..value_at], array)?;
    calls_array(array_init, string_array).then_some(Keys {
        decoder,
        value,
        array,
        state,
        memo,
    })
}

struct Uses<'s, 'a> {
    keys: &'s Keys<'a>,
    value_bindings: u32,
    array_bindings: u32,
    value_refs: u32,
    array_writes: u32,
    state_accesses: u32,
    memo_reads: u32,
    opaque: bool,
}

impl<'a> Uses<'_, 'a> {
    fn proven(&self) -> bool {
        self.value_bindings == 1
            && self.array_bindings == 1
            && self.value_refs == 2
            && self.array_writes == 0
            && self.state_accesses == 2
            && self.memo_reads == 0
            && !self.opaque
    }

    fn is_own_memo(&self, expr: &Expression<'a>) -> bool {
        expr.without_parentheses()
            .as_member_expression()
            .and_then(|member| own_key(member, self.keys.decoder))
            == Some(self.keys.memo)
    }
}

impl<'a> VisitJs<'a> for Uses<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let name = it.name.as_str();
        self.value_bindings += u32::from(name == self.keys.value);
        self.array_bindings += u32::from(name == self.keys.array);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.value_refs += u32::from(name == self.keys.value);
        self.opaque |= name == "eval";
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.array_writes += u32::from(id.name.as_str() == self.keys.array);
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_pattern(&mut self, _it: &AssignmentTargetPattern<'a>) {
        self.opaque = true;
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let written = target_key_of(&it.left, self.keys.decoder);
        if written == Some(self.keys.state) || written == Some(self.keys.memo) {
            self.state_accesses += u32::from(written == Some(self.keys.state));
            self.visit_expression(&it.right);
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if let MemberExpression::ComputedMemberExpression(computed) = it {
            if self.is_own_memo(&computed.object) {
                self.visit_expression(&computed.expression);
                return;
            }
        }
        let key = own_key(it, self.keys.decoder);
        self.state_accesses += u32::from(key == Some(self.keys.state));
        self.memo_reads += u32::from(key == Some(self.keys.memo));
        walk_js::walk_member_expression(self, it);
    }
}

fn target_key_of<'a>(target: &AssignmentTarget<'a>, decoder: &str) -> Option<&'a str> {
    own_key(target.as_member_expression()?, decoder)
}

fn uses_are_exact(function: &Function<'_>, keys: &Keys<'_>) -> bool {
    let mut uses = Uses {
        keys,
        value_bindings: 0,
        array_bindings: 0,
        value_refs: 0,
        array_writes: 0,
        state_accesses: 0,
        memo_reads: 0,
        opaque: false,
    };
    uses.visit_function(function, ScopeFlags::Function);
    uses.proven()
}

fn touches(peel: &MemberPeel<'_>, keys: &Keys<'_>) -> bool {
    let named = |key: &str| matches!(peel.final_key, FinalKey::Static(found) if found == key);
    peel.root == keys.decoder
        && match peel.segment {
            None => named(keys.state) || named(keys.memo),
            Some(segment) => segment == keys.state || segment == keys.memo,
        }
}

fn untouched_elsewhere(owned: &[(Owner, Collected<'_>)], home: usize, keys: &Keys<'_>) -> bool {
    owned
        .iter()
        .filter(|(owner, _)| *owner != Owner::Decoder(home))
        .flat_map(|(_, collected)| collected.accesses.iter())
        .all(|access| !touches(&access.peel, keys))
}

fn is_own_write(kind: &TriggerKind<'_>, keys: &Keys<'_>, segment: Option<&str>) -> bool {
    let TriggerKind::Member(peel) = kind else {
        return false;
    };
    peel.root == keys.decoder
        && peel.segment == segment
        && match (segment, peel.final_key) {
            (None, FinalKey::Static(key)) => key == keys.memo,
            (Some(_), FinalKey::Computed(_)) => true,
            _ => false,
        }
}

fn memo_is_proven(collected: &Collected<'_>, keys: &Keys<'_>) -> bool {
    let initialized = collected.gates.iter().any(|gate| {
        !gate.conditional
            && gate.root == keys.decoder
            && gate.segment.is_none()
            && collected.triggers.iter().any(|trigger| {
                span_contains(gate.region, trigger.span) && is_own_write(&trigger.kind, keys, None)
            })
    });
    let filled = collected
        .triggers
        .iter()
        .any(|trigger| is_own_write(&trigger.kind, keys, Some(keys.memo)));
    initialized && filled
}

pub(super) struct Decoder<'n, 'a> {
    pub(super) function: &'n Function<'a>,
    pub(super) name: &'a str,
    pub(super) index: usize,
    pub(super) string_array: &'a str,
}

pub(super) fn find<'a>(
    decoder: &Decoder<'_, 'a>,
    owned: &[(Owner, Collected<'a>)],
) -> Option<ResetGate<'a>> {
    let statements = &decoder.function.body.as_ref()?.statements;
    let collected = owned
        .iter()
        .find(|(owner, _)| *owner == Owner::Decoder(decoder.index))
        .map(|(_, collected)| collected)?;
    statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| candidate(index, statement))
        .find_map(|found| {
            let keys = keys_of(statements, &found, decoder.name, decoder.string_array)?;
            let proven = memo_is_proven(collected, &keys)
                && uses_are_exact(decoder.function, &keys)
                && untouched_elsewhere(owned, decoder.index, &keys);
            proven.then_some(ResetGate {
                owner: decoder.index,
                region: found.region,
                writes: [found.reset.span, found.store.span],
                memo_key: keys.memo,
            })
        })
}
