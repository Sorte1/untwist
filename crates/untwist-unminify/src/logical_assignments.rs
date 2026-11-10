use crate::transpile_util::{
    assignment_temp_symbol, identifier_symbol_is, is_repeatable, is_repeatable_target, keys_match,
    member_parts, restore_logical_assignment, take_logical_assignment, take_temp_member_target,
    target_parts, TempVars,
};
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::{ContentEq, Span};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct LogicalAssignments {
    temps: TempVars,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(LogicalAssignments::default())
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::LogicalExpression]);

impl<'a> Pass<'a> for LogicalAssignments {
    fn name(&self) -> &'static str {
        "logical-assignments"
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

    fn enter_program(&mut self, _node: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {
        self.temps.reset();
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
        let operator = match logical.operator {
            LogicalOperator::Or => AssignmentOperator::LogicalOr,
            LogicalOperator::And => AssignmentOperator::LogicalAnd,
            LogicalOperator::Coalesce => return,
        };
        let rewritten = rewrite_plain(node, operator, cx)
            || rewrite_temp_object(node, operator, &mut self.temps, cx)
            || rewrite_temp_key(node, operator, &mut self.temps, cx)
            || rewrite_temp_both(node, operator, &mut self.temps, cx);
        if rewritten {
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

fn rewrite_plain<'a>(
    node: &mut Expression<'a>,
    operator: AssignmentOperator,
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
    if !target_matches(&assignment.left, &logical.left) || !is_repeatable_target(&logical.left, cx)
    {
        return false;
    }
    let Some((span, _, _, assignment)) = take_logical_assignment(node, cx) else {
        return false;
    };
    *node = Expression::new_assignment_expression(
        span,
        operator,
        assignment.left,
        assignment.right,
        cx,
    );
    true
}

fn rewrite_temp_object<'a>(
    node: &mut Expression<'a>,
    operator: AssignmentOperator,
    temps: &mut TempVars,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::LogicalExpression(logical) = &*node else {
        return false;
    };
    let Some(assignment) = inner_assignment(logical) else {
        return false;
    };
    let Some((object, left_key)) = member_parts(&logical.left) else {
        return false;
    };
    let Some(symbol) = assignment_temp_symbol(object, cx) else {
        return false;
    };
    let Some((holder, right_key)) = target_parts(&assignment.left) else {
        return false;
    };
    if !identifier_symbol_is(holder, symbol, cx)
        || !keys_match(&left_key, &right_key, cx)
        || !temps.is_temp(symbol, 1, cx)
    {
        return false;
    }
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
    *node = Expression::new_assignment_expression(span, operator, target, assignment.right, cx);
    true
}

fn temp_key_symbol(
    logical: &LogicalExpression<'_>,
    assignment: &AssignmentExpression<'_>,
    temps: &TempVars,
    cx: &Cx<'_, '_>,
) -> Option<SymbolId> {
    let Expression::ComputedMemberExpression(member) = &logical.left else {
        return None;
    };
    if member.optional {
        return None;
    }
    let symbol = assignment_temp_symbol(&member.expression, cx)?;
    let AssignmentTarget::ComputedMemberExpression(target) = &assignment.left else {
        return None;
    };
    if target.optional
        || !ContentEq::content_eq(&target.object, &member.object)
        || !is_repeatable(&member.object, cx)
        || !identifier_symbol_is(&target.expression, symbol, cx)
        || !temps.is_temp(symbol, 1, cx)
    {
        return None;
    }
    Some(symbol)
}

fn take_temp_key_target<'a>(
    left: Expression<'a>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Result<AssignmentTarget<'a>, Expression<'a>> {
    let member = match left {
        Expression::ComputedMemberExpression(member) => member.unbox(),
        other => return Err(other),
    };
    let member_span = member.span;
    let member_optional = member.optional;
    match member.expression {
        Expression::AssignmentExpression(key) => {
            Ok(AssignmentTarget::new_computed_member_expression(
                span,
                member.object,
                key.unbox().right,
                false,
                cx,
            ))
        }
        other => Err(Expression::new_computed_member_expression(
            member_span,
            member.object,
            other,
            member_optional,
            cx,
        )),
    }
}

fn rewrite_temp_key<'a>(
    node: &mut Expression<'a>,
    operator: AssignmentOperator,
    temps: &mut TempVars,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::LogicalExpression(logical) = &*node else {
        return false;
    };
    let Some(assignment) = inner_assignment(logical) else {
        return false;
    };
    let Some(symbol) = temp_key_symbol(logical, assignment, temps, cx) else {
        return false;
    };
    let Some((span, logical_operator, left, assignment)) = take_logical_assignment(node, cx) else {
        return false;
    };
    let target = match take_temp_key_target(left, span, cx) {
        Ok(target) => target,
        Err(left) => {
            *node = restore_logical_assignment(span, left, logical_operator, assignment, cx);
            return false;
        }
    };
    temps.consume(symbol);
    *node = Expression::new_assignment_expression(span, operator, target, assignment.right, cx);
    true
}

fn temp_both_symbols(
    logical: &LogicalExpression<'_>,
    assignment: &AssignmentExpression<'_>,
    temps: &TempVars,
    cx: &Cx<'_, '_>,
) -> Option<(SymbolId, SymbolId)> {
    let Expression::ComputedMemberExpression(member) = &logical.left else {
        return None;
    };
    if member.optional {
        return None;
    }
    let object_symbol = assignment_temp_symbol(&member.object, cx)?;
    let key_symbol = assignment_temp_symbol(&member.expression, cx)?;
    let AssignmentTarget::ComputedMemberExpression(target) = &assignment.left else {
        return None;
    };
    if target.optional
        || !identifier_symbol_is(&target.object, object_symbol, cx)
        || !identifier_symbol_is(&target.expression, key_symbol, cx)
        || !temps.is_temp(object_symbol, 1, cx)
        || !temps.is_temp(key_symbol, 1, cx)
    {
        return None;
    }
    Some((object_symbol, key_symbol))
}

fn take_temp_both_target<'a>(
    left: Expression<'a>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Result<AssignmentTarget<'a>, Expression<'a>> {
    let member = match left {
        Expression::ComputedMemberExpression(member) => member.unbox(),
        other => return Err(other),
    };
    let member_span = member.span;
    let member_optional = member.optional;
    let key_expr = member.expression;
    let object = match member.object {
        Expression::AssignmentExpression(object) => object.unbox(),
        other => {
            return Err(Expression::new_computed_member_expression(
                member_span,
                other,
                key_expr,
                member_optional,
                cx,
            ))
        }
    };
    match key_expr {
        Expression::AssignmentExpression(key) => {
            let target = AssignmentTarget::new_computed_member_expression(
                span,
                object.right,
                key.unbox().right,
                false,
                cx,
            );
            Ok(target)
        }
        other => {
            let restored_object = restore_assignment_expression(object, cx);
            Err(Expression::new_computed_member_expression(
                member_span,
                restored_object,
                other,
                member_optional,
                cx,
            ))
        }
    }
}

fn restore_assignment_expression<'a>(
    assignment: AssignmentExpression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Expression<'a> {
    Expression::new_assignment_expression(
        assignment.span,
        assignment.operator,
        assignment.left,
        assignment.right,
        cx,
    )
}

fn rewrite_temp_both<'a>(
    node: &mut Expression<'a>,
    operator: AssignmentOperator,
    temps: &mut TempVars,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Expression::LogicalExpression(logical) = &*node else {
        return false;
    };
    let Some(assignment) = inner_assignment(logical) else {
        return false;
    };
    let Some((object_symbol, key_symbol)) = temp_both_symbols(logical, assignment, temps, cx)
    else {
        return false;
    };
    let Some((span, logical_operator, left, assignment)) = take_logical_assignment(node, cx) else {
        return false;
    };
    let target = match take_temp_both_target(left, span, cx) {
        Ok(target) => target,
        Err(left) => {
            *node = restore_logical_assignment(span, left, logical_operator, assignment, cx);
            return false;
        }
    };
    temps.consume(object_symbol);
    temps.consume(key_symbol);
    *node = Expression::new_assignment_expression(span, operator, target, assignment.right, cx);
    true
}
