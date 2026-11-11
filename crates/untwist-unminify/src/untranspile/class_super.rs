use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;

use super::class_ctor::{argument, is_this, plain};
use super::identify::{resolved, Table};
use super::shapes::Kind;
use super::values::into_inner;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SuperForm {
    Assign,
    Chained,
    Return,
}

#[derive(Clone, Copy)]
pub(super) struct Derived {
    pub(super) form: SuperForm,
    pub(super) at: usize,
    pub(super) this: Option<SymbolId>,
    pub(super) target: u32,
    pub(super) end: u32,
    pub(super) declared: Option<usize>,
    pub(super) concat: Option<SymbolId>,
    pub(super) spread_concat: bool,
}

struct Found {
    form: SuperForm,
    this: Option<SymbolId>,
    target: u32,
    end: u32,
    concat: Option<SymbolId>,
}

fn concat_symbol(call: &CallExpression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    argument(call, 2)
        .and_then(concat_source)
        .and_then(|source| resolved(source, scoping))
}

fn is_arguments(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    matches!(expression, Expression::Identifier(id)
        if id.name.as_str() == "arguments"
            && id.reference_id.get().is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none()))
}

fn concat_source<'k, 'a>(expression: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };
    let empty = matches!(member.object.without_parentheses(),
        Expression::ArrayExpression(array) if array.elements.is_empty());
    let single = plain(call) && call.arguments.len() == 1;
    (empty && single && !call.optional && !member.optional && member.property.name == "concat")
        .then(|| argument(call, 0))
        .flatten()
        .filter(|source| matches!(source, Expression::Identifier(_)))
}

fn super_arguments_ok(call: &CallExpression<'_>, scoping: &Scoping) -> bool {
    match argument(call, 2) {
        None => call.arguments.len() == 2,
        Some(Expression::ArrayExpression(array)) => !super::values::has_elision(array),
        Some(other) => is_arguments(other, scoping) || concat_source(other).is_some(),
    }
}

fn super_call<'k, 'a>(
    expression: &'k Expression<'a>,
    symbol: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<&'k CallExpression<'a>> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    let shaped = plain(call)
        && (2..=3).contains(&call.arguments.len())
        && table.kind_of_callee(&call.callee, scoping) == Some(Kind::CallSuper)
        && is_this(argument(call, 0))
        && argument(call, 1).and_then(|it| resolved(it, scoping)) == Some(symbol)
        && super_arguments_ok(call, scoping);
    shaped.then_some(call)
}

fn assigned_super(
    assign: &AssignmentExpression<'_>,
    symbol: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<(SymbolId, u32, Option<SymbolId>)> {
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let call = super_call(&assign.right, symbol, table, scoping)?;
    let this = scoping
        .get_reference(target.reference_id.get()?)
        .symbol_id()?;
    Some((this, target.span.start, concat_symbol(call, scoping)))
}

const DESCENT: u32 = 8;

fn is_local_callee(callee: &Expression<'_>, scoping: &Scoping) -> bool {
    resolved(callee, scoping).is_some()
}

fn first_operand<'k, 'a>(
    expression: &'k Expression<'a>,
    scoping: &Scoping,
) -> Option<&'k Expression<'a>> {
    match expression {
        Expression::ParenthesizedExpression(it) => Some(&it.expression),
        Expression::StaticMemberExpression(it) => Some(&it.object),
        Expression::ComputedMemberExpression(it) => Some(&it.object),
        Expression::AssignmentExpression(it) => match &it.left {
            AssignmentTarget::StaticMemberExpression(member) => Some(&member.object),
            AssignmentTarget::ComputedMemberExpression(member) => Some(&member.object),
            _ => None,
        },
        Expression::CallExpression(call) if !call.optional => match &call.callee {
            Expression::StaticMemberExpression(member) => Some(&member.object),
            Expression::ComputedMemberExpression(member) => Some(&member.object),
            callee if is_local_callee(callee, scoping) => call.arguments.first()?.as_expression(),
            _ => None,
        },
        _ => None,
    }
}

fn first_operand_mut<'k, 'a>(
    expression: &'k mut Expression<'a>,
    scoping: &Scoping,
) -> Option<&'k mut Expression<'a>> {
    match expression {
        Expression::ParenthesizedExpression(it) => Some(&mut it.expression),
        Expression::StaticMemberExpression(it) => Some(&mut it.object),
        Expression::ComputedMemberExpression(it) => Some(&mut it.object),
        Expression::AssignmentExpression(it) => match &mut it.left {
            AssignmentTarget::StaticMemberExpression(member) => Some(&mut member.object),
            AssignmentTarget::ComputedMemberExpression(member) => Some(&mut member.object),
            _ => None,
        },
        Expression::CallExpression(call) if !call.optional => {
            let member = matches!(
                call.callee,
                Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)
            );
            if member {
                return match &mut call.callee {
                    Expression::StaticMemberExpression(member) => Some(&mut member.object),
                    Expression::ComputedMemberExpression(member) => Some(&mut member.object),
                    _ => None,
                };
            }
            if !is_local_callee(&call.callee, scoping) {
                return None;
            }
            call.arguments.first_mut()?.as_expression_mut()
        }
        _ => None,
    }
}

fn nested_super<'k, 'a>(
    expression: &'k Expression<'a>,
    symbol: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<Found> {
    let mut at = first_operand(expression, scoping)?;
    for _ in 0..DESCENT {
        if let Expression::AssignmentExpression(assign) = at {
            if let Some((this, target, concat)) = assigned_super(assign, symbol, table, scoping) {
                return Some(Found {
                    form: SuperForm::Chained,
                    this: Some(this),
                    target,
                    end: assign.span.end,
                    concat,
                });
            }
        }
        at = first_operand(at, scoping)?;
    }
    None
}

fn super_slot<'k, 'a>(
    expression: &'k mut Expression<'a>,
    scoping: &Scoping,
) -> Option<&'k mut Expression<'a>> {
    let mut at = first_operand_mut(expression, scoping)?;
    for _ in 0..DESCENT {
        let is_super = matches!(at, Expression::AssignmentExpression(assign)
            if matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_)));
        if is_super {
            return Some(at);
        }
        at = first_operand_mut(at, scoping)?;
    }
    None
}

fn is_identifier_assignment(expression: &Expression<'_>) -> bool {
    matches!(expression.without_parentheses(), Expression::AssignmentExpression(assign)
        if matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_)))
}

fn statement_expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(statement) => Some(&statement.expression),
        Statement::VariableDeclaration(declaration) => match declaration.declarations.as_slice() {
            [declarator] => declarator.init.as_ref(),
            _ => None,
        },
        _ => None,
    }
}

fn super_statement(
    statement: &Statement<'_>,
    symbol: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<Found> {
    if let Statement::ReturnStatement(ret) = statement {
        let call = super_call(ret.argument.as_ref()?, symbol, table, scoping)?;
        return Some(Found {
            form: SuperForm::Return,
            this: None,
            target: 0,
            end: ret.span.end,
            concat: concat_symbol(call, scoping),
        });
    }
    let expression = statement_expression(statement)?;
    if let (Statement::ExpressionStatement(_), Expression::AssignmentExpression(assign)) =
        (statement, expression)
    {
        if let Some((this, target, concat)) = assigned_super(assign, symbol, table, scoping) {
            return Some(Found {
                form: SuperForm::Assign,
                this: Some(this),
                target,
                end: assign.span.end,
                concat,
            });
        }
    }
    if !is_identifier_assignment(expression) {
        return nested_super(expression, symbol, table, scoping);
    }
    let Expression::AssignmentExpression(assign) = expression.without_parentheses() else {
        return None;
    };
    let (this, target, concat) = assigned_super(assign, symbol, table, scoping)?;
    Some(Found {
        form: SuperForm::Chained,
        this: Some(this),
        target,
        end: assign.span.end,
        concat,
    })
}

pub(super) fn find_super(
    statements: &[Statement<'_>],
    symbol: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<Derived> {
    let mut found = None;
    for (at, statement) in statements.iter().enumerate() {
        if let Some(it) = super_statement(statement, symbol, table, scoping) {
            if found.is_some() {
                return None;
            }
            found = Some(Derived {
                form: it.form,
                at,
                this: it.this,
                target: it.target,
                end: it.end,
                declared: None,
                concat: it.concat,
                spread_concat: false,
            });
        }
    }
    found
}

fn super_arguments<'a>(
    call: Expression<'a>,
    spread_concat: bool,
    cx: &Cx<'a, '_>,
) -> AVec<'a, Argument<'a>> {
    let mut out = AVec::new_in(cx);
    let Expression::CallExpression(call) = into_inner(call) else {
        return out;
    };
    let Some(third) = call.unbox().arguments.into_iter().nth(2) else {
        return out;
    };
    match Expression::try_from(third).map(into_inner) {
        Ok(Expression::ArrayExpression(array)) => {
            for element in array.unbox().elements {
                if let Some(argument) = element_argument(element) {
                    out.push(argument);
                }
            }
        }
        Ok(other) => {
            let source = match other {
                Expression::CallExpression(call) if spread_concat => call
                    .unbox()
                    .arguments
                    .into_iter()
                    .next()
                    .and_then(|argument| Expression::try_from(argument).ok()),
                other => Some(other),
            };
            if let Some(source) = source {
                out.push(Argument::new_spread_element(SPAN, into_inner(source), cx));
            }
        }
        Err(_) => {}
    }
    out
}

fn element_argument(element: ArrayExpressionElement<'_>) -> Option<Argument<'_>> {
    match element {
        ArrayExpressionElement::SpreadElement(spread) => Some(Argument::SpreadElement(spread)),
        ArrayExpressionElement::Elision(_) => None,
        other => Some(Argument::from(other.into_expression())),
    }
}

fn take_super<'a>(slot: &mut Expression<'a>, cx: &Cx<'a, '_>) -> Option<Expression<'a>> {
    let span = slot.span();
    let inner = std::mem::replace(slot, Expression::new_this_expression(span, cx));
    match into_inner(inner) {
        Expression::AssignmentExpression(inner) => Some(inner.unbox().right),
        _ => None,
    }
}

fn statement_expression_mut<'k, 'a>(
    statement: &'k mut Statement<'a>,
) -> Option<&'k mut Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(statement) => Some(&mut statement.expression),
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.first_mut()?.init.as_mut()
        }
        _ => None,
    }
}

pub(super) fn split_super<'a>(
    mut statement: Statement<'a>,
    form: SuperForm,
    cx: &Cx<'a, '_>,
) -> (Option<Expression<'a>>, Option<Statement<'a>>) {
    match (form, &mut statement) {
        (SuperForm::Return, Statement::ReturnStatement(ret)) => (ret.argument.take(), None),
        (SuperForm::Assign, Statement::ExpressionStatement(it)) => {
            let call = match &mut it.expression {
                Expression::AssignmentExpression(assign) => Some(std::mem::replace(
                    &mut assign.right,
                    Expression::new_null_literal(SPAN, cx),
                )),
                _ => None,
            };
            (call, None)
        }
        (SuperForm::Chained, _) => {
            let scoping = cx.scoping();
            let call = statement_expression_mut(&mut statement).and_then(|expression| {
                let wrapped = is_identifier_assignment(expression);
                if wrapped {
                    take_super(expression, cx)
                } else {
                    super_slot(expression, scoping).and_then(|slot| take_super(slot, cx))
                }
            });
            (call, Some(statement))
        }
        _ => (None, None),
    }
}

pub(super) fn super_statement_of<'a>(
    call: Option<Expression<'a>>,
    spread_concat: bool,
    cx: &Cx<'a, '_>,
) -> (Statement<'a>, bool) {
    let span = call.as_ref().map_or(SPAN, GetSpan::span);
    let arguments = call.map_or_else(
        || AVec::new_in(cx),
        |call| super_arguments(call, spread_concat, cx),
    );
    let forwards = matches!(arguments.as_slice(), [Argument::SpreadElement(spread)]
        if matches!(&spread.argument, Expression::Identifier(id) if id.name.as_str() == "arguments"));
    let callee = Expression::new_super(SPAN, cx);
    let call = Expression::new_call_expression(span, callee, None, arguments, false, cx);
    (
        Statement::new_expression_statement(span, call, cx),
        forwards,
    )
}
