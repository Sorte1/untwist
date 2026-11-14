use crate::transpile_util::{
    assignment_temp_symbol, identifier_symbol_is, is_repeatable_target, keys_match, member_parts,
    restore_logical_assignment, script_root, take_logical_assignment, take_temp_member_target,
    target_parts, MemberKey, TempVars,
};
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::ContentEq;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct NullishCoalescingAssignment {
    temps: TempVars,
    script_root: Option<ScopeId>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(NullishCoalescingAssignment::default())
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::LogicalExpression]);

impl<'a> Pass<'a> for NullishCoalescingAssignment {
    fn name(&self) -> &'static str {
        "nullish-coalescing-assignment"
    }

    fn locality(&self) -> Locality {
        Locality::Local
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::EXIT_VARIABLE_DECLARATION
            | Interest::EXIT_EXPRESSION
            | Interest::EXIT_STATEMENTS)
            .on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn enter_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.temps.reset();
        self.script_root = script_root(node, cx.scoping());
        cx.observe_names(node);
    }

    fn exit_variable_declaration(
        &mut self,
        node: &mut VariableDeclaration<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        self.temps.record_declaration(node, cx);
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::LogicalExpression(logical) = &*node else {
            return;
        };
        if logical.operator != LogicalOperator::Coalesce {
            return;
        }
        let root = self.script_root;
        if rewrite_temp_member(node, &mut self.temps, root, cx) || rewrite_simple(node, root, cx) {
            cx.changed();
        }
    }

    fn exit_statements(&mut self, node: &mut AVec<'a, Statement<'a>>, _cx: &mut Cx<'a, '_>) {
        self.temps.strip_declarations(node);
    }
}

fn inner_assignment<'k, 'a>(
    logical: &'k LogicalExpression<'a>,
) -> Option<&'k AssignmentExpression<'a>> {
    let Expression::AssignmentExpression(assignment) = &logical.right else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign).then_some(assignment)
}

fn key_is_identifier(key: &MemberKey<'_, '_>) -> bool {
    match key {
        MemberKey::Static(_) => true,
        MemberKey::Computed(expr) => matches!(expr, Expression::Identifier(_)),
    }
}

fn target_matches(target: &AssignmentTarget<'_>, expr: &Expression<'_>) -> bool {
    match (target, expr) {
        (AssignmentTarget::AssignmentTargetIdentifier(target), Expression::Identifier(value)) => {
            ContentEq::content_eq(&**target, &**value)
        }
        (
            AssignmentTarget::StaticMemberExpression(target),
            Expression::StaticMemberExpression(value),
        ) => ContentEq::content_eq(&**target, &**value),
        (
            AssignmentTarget::ComputedMemberExpression(target),
            Expression::ComputedMemberExpression(value),
        ) => ContentEq::content_eq(&**target, &**value),
        _ => false,
    }
}

fn temp_member_symbol(
    logical: &LogicalExpression<'_>,
    assignment: &AssignmentExpression<'_>,
    temps: &TempVars,
    script_root: Option<ScopeId>,
    cx: &Cx<'_, '_>,
) -> Option<SymbolId> {
    let (object, left_key) = member_parts(&logical.left)?;
    if !key_is_identifier(&left_key) {
        return None;
    }
    let Expression::AssignmentExpression(inner) = object else {
        return None;
    };
    if !matches!(inner.right, Expression::Identifier(_)) {
        return None;
    }
    let symbol = assignment_temp_symbol(object, cx)?;
    let (holder, right_key) = target_parts(&assignment.left)?;
    if !identifier_symbol_is(holder, symbol, cx)
        || !keys_match(&left_key, &right_key, script_root, cx)
        || !temps.is_temp(symbol, 1, cx)
    {
        return None;
    }
    Some(symbol)
}

fn rewrite_temp_member<'a>(
    node: &mut Expression<'a>,
    temps: &mut TempVars,
    script_root: Option<ScopeId>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::LogicalExpression(logical) = &*node else {
        return false;
    };
    let Some(assignment) = inner_assignment(logical) else {
        return false;
    };
    let Some(symbol) = temp_member_symbol(logical, assignment, temps, script_root, cx) else {
        return false;
    };
    let Some((span, logical_operator, left, assignment)) = take_logical_assignment(node, cx) else {
        return false;
    };
    let target = match take_temp_member_target(left, span, cx) {
        Ok(target) => target,
        Err(left) => {
            *node = restore_logical_assignment(span, left, logical_operator, assignment, cx);
            return false;
        }
    };
    temps.consume(symbol);
    *node = Expression::new_assignment_expression(
        span,
        AssignmentOperator::LogicalNullish,
        target,
        assignment.right,
        cx,
    );
    true
}

fn rewrite_simple<'a>(
    node: &mut Expression<'a>,
    script_root: Option<ScopeId>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::LogicalExpression(logical) = &*node else {
        return false;
    };
    if !matches!(
        logical.left,
        Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
    ) {
        return false;
    }
    let Some(assignment) = inner_assignment(logical) else {
        return false;
    };
    if !target_matches(&assignment.left, &logical.left)
        || !is_repeatable_target(&logical.left, script_root, cx)
    {
        return false;
    }
    let Some((span, _, _, assignment)) = take_logical_assignment(node, cx) else {
        return false;
    };
    *node = Expression::new_assignment_expression(
        span,
        AssignmentOperator::LogicalNullish,
        assignment.left,
        assignment.right,
        cx,
    );
    true
}
