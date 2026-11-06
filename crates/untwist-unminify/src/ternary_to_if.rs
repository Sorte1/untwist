use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_traverse::Ancestor;
use untwist_core::depth::LIFT_DEPTH_MAX;
use untwist_core::pass::{Cx, Interest, Locality, Pass, StatementKind, StatementVariants};

use crate::{nullish_coalescing, optional_chaining, remove_double_not};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    AlternateChain,
    OneLevel,
}

pub struct TernaryToIf {
    reach: Reach,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(TernaryToIf {
        reach: Reach::AlternateChain,
    })
}

#[doc(hidden)]
pub fn one_level_boxed_for_tests<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(TernaryToIf {
        reach: Reach::OneLevel,
    })
}

fn is_rewritable(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ExpressionStatement(node) => {
            matches!(node.expression, Expression::ConditionalExpression(_))
        }
        Statement::ReturnStatement(node) => {
            matches!(node.argument, Some(Expression::ConditionalExpression(_)))
        }
        _ => false,
    }
}

fn take_conditional<'a>(
    stmt: Statement<'a>,
    cx: &mut Cx<'a, '_>,
) -> Result<(ConditionalExpression<'a>, bool), Statement<'a>> {
    match stmt {
        Statement::ExpressionStatement(node) => {
            let node = node.unbox();
            let Expression::ConditionalExpression(conditional) = node.expression else {
                return Err(Statement::new_expression_statement(
                    node.span,
                    node.expression,
                    cx,
                ));
            };
            Ok((conditional.unbox(), false))
        }
        Statement::ReturnStatement(node) => {
            let node = node.unbox();
            let Some(Expression::ConditionalExpression(conditional)) = node.argument else {
                return Err(Statement::new_return_statement(
                    node.span,
                    node.argument,
                    cx,
                ));
            };
            Ok((conditional.unbox(), true))
        }
        other => Err(other),
    }
}

fn is_inert_leaf(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier(identifier) => identifier.name != "undefined",
        Expression::ThisExpression(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        _ => false,
    }
}

fn push_inert_children<'b, 'a>(
    expr: &'b Expression<'a>,
    pending: &mut Vec<&'b Expression<'a>>,
) -> bool {
    match expr {
        Expression::StaticMemberExpression(member) if !member.optional => {
            pending.push(&member.object);
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            pending.extend([&member.object, &member.expression]);
        }
        Expression::BinaryExpression(binary) => pending.extend([&binary.left, &binary.right]),
        Expression::LogicalExpression(logical) => pending.extend([&logical.left, &logical.right]),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
            pending.push(&unary.argument);
        }
        Expression::ConditionalExpression(conditional) => {
            pending.extend([
                &conditional.test,
                &conditional.consequent,
                &conditional.alternate,
            ]);
        }
        other => return is_inert_leaf(other),
    }
    true
}

fn is_inert(test: &Expression<'_>) -> bool {
    let mut pending = vec![test];
    while let Some(expr) = pending.pop() {
        if !push_inert_children(expr, &mut pending) {
            return false;
        }
    }
    true
}

fn claimed_on_a_revisit(value: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    let Expression::ConditionalExpression(conditional) = value else {
        return false;
    };
    !is_inert(&conditional.test)
        || nullish_coalescing::would_rewrite(conditional, cx)
        || optional_chaining::would_rewrite(conditional, cx)
        || remove_double_not::would_rewrite(conditional, cx)
}

fn lifted_depth(cx: &Cx<'_, '_>) -> usize {
    let mut depth = 0;
    for ancestor in cx.tcx.ancestors() {
        match ancestor {
            Ancestor::BlockStatementBody(_) => {}
            Ancestor::IfStatementAlternate(_) => depth += 1,
            _ => break,
        }
        if depth >= LIFT_DEPTH_MAX {
            break;
        }
    }
    depth
}

#[derive(Clone, Copy)]
struct Lift {
    reach: Reach,
    levels: usize,
}

fn branch<'a>(
    span: Span,
    value: Expression<'a>,
    returned: bool,
    lift: Lift,
    cx: &mut Cx<'a, '_>,
) -> Statement<'a> {
    let recurse =
        lift.reach == Reach::AlternateChain && lift.levels > 1 && !claimed_on_a_revisit(&value, cx);
    let statement = if returned {
        Statement::new_return_statement(span, Some(value), cx)
    } else {
        Statement::new_expression_statement(span, value, cx)
    };
    if !recurse || !is_rewritable(&statement) {
        return cx.block_of(statement);
    }
    let deeper = Lift {
        levels: lift.levels - 1,
        ..lift
    };
    match rewrite(statement, deeper, cx) {
        Ok(nested) => {
            cx.changed();
            cx.block_of(nested)
        }
        Err(original) => cx.block_of(original),
    }
}

fn rewrite<'a>(
    stmt: Statement<'a>,
    lift: Lift,
    cx: &mut Cx<'a, '_>,
) -> Result<Statement<'a>, Statement<'a>> {
    let span = stmt.span();
    let (conditional, returned) = take_conditional(stmt, cx)?;
    let consequent_span = conditional.consequent.span();
    let alternate_span = conditional.alternate.span();
    let consequent = branch(consequent_span, conditional.consequent, returned, lift, cx);
    let alternate = branch(alternate_span, conditional.alternate, returned, lift, cx);
    Ok(Statement::new_if_statement(
        span,
        conditional.test,
        consequent,
        Some(alternate),
        cx,
    ))
}

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[
    StatementKind::ExpressionStatement,
    StatementKind::ReturnStatement,
]);

impl<'a> Pass<'a> for TernaryToIf {
    fn name(&self) -> &'static str {
        "ternary-to-if"
    }

    fn cascades(&self) -> bool {
        true
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT.on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if !is_rewritable(stmt) {
            return;
        }
        let levels = LIFT_DEPTH_MAX.saturating_sub(lifted_depth(cx));
        if levels == 0 {
            return;
        }
        let lift = Lift {
            reach: self.reach,
            levels,
        };
        let span = stmt.span();
        let placeholder = Statement::new_empty_statement(span, cx);
        let taken = std::mem::replace(stmt, placeholder);
        match rewrite(taken, lift, cx) {
            Ok(rewritten) => {
                *stmt = rewritten;
                cx.changed();
            }
            Err(original) => *stmt = original,
        }
    }
}
