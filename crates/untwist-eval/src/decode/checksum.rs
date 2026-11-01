use oxc_ast::ast::{BindingPattern, CallExpression, Expression, Statement};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::bail::{AbruptKind, EvalResult};
use crate::template::identifier_name;
use crate::value::Eval;

pub(super) fn addition_terms<'a>(expr: &'a Expression<'a>, out: &mut Vec<&'a Expression<'a>>) {
    let mut current = expr;
    loop {
        match current {
            Expression::BinaryExpression(node) if node.operator == BinaryOperator::Addition => {
                out.push(&node.right);
                current = &node.left;
            }
            Expression::ParenthesizedExpression(node) => current = &node.expression,
            other => {
                out.push(other);
                break;
            }
        }
    }
    out.reverse();
}

pub struct Checksum<'a> {
    pub body: &'a [Statement<'a>],
    pub prelude: Vec<&'a Statement<'a>>,
    pub expression: &'a Expression<'a>,
    pub array_argument: &'a Expression<'a>,
    pub expected: &'a Expression<'a>,
    pub array_param: &'a str,
    pub expected_param: &'a str,
}

pub fn checksum_from_rotator<'a>(iife: &'a Expression<'a>) -> Option<Checksum<'a>> {
    let call = unwrap_iife(iife)?;
    let Expression::FunctionExpression(function) = &call.callee else {
        return None;
    };
    if function.generator || function.r#async {
        return None;
    }
    let body = function.body.as_ref()?;
    let mut params = function
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        });
    let array_param = params.next()?;
    let expected_param = params.next()?;
    let array_argument = call.arguments.first()?.as_expression()?;
    let expected = call.arguments.get(1)?.as_expression()?;
    if !crate::template::template_exact(&body.statements, array_param, expected_param) {
        return None;
    }

    let mut prelude = Vec::new();
    let mut expression = None;
    for statement in &body.statements {
        if let Some(found) = checksum_in_loop(statement, expected_param) {
            expression = Some(found);
            break;
        }
        prelude.push(statement);
    }
    Some(Checksum {
        body: &body.statements,
        prelude,
        expression: expression?,
        array_argument,
        expected,
        array_param,
        expected_param,
    })
}

pub(super) fn rotator_body_is_strict(iife: &Expression<'_>) -> bool {
    unwrap_iife(iife).is_some_and(|call| {
        matches!(&call.callee, Expression::FunctionExpression(function) if function.has_use_strict_directive())
    })
}

fn unwrap_iife<'a>(expr: &'a Expression<'a>) -> Option<&'a CallExpression<'a>> {
    match expr {
        Expression::CallExpression(call) => Some(call.as_ref()),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unwrap_iife(&unary.argument)
        }
        Expression::ParenthesizedExpression(inner) => unwrap_iife(&inner.expression),
        _ => None,
    }
}

fn checksum_in_loop<'a>(
    statement: &'a Statement<'a>,
    expected_param: &str,
) -> Option<&'a Expression<'a>> {
    let body = match statement {
        Statement::WhileStatement(node) => &node.body,
        Statement::ForStatement(node) => &node.body,
        _ => return None,
    };
    let Statement::BlockStatement(block) = body else {
        return None;
    };
    let Some(Statement::TryStatement(node)) = block.body.first() else {
        return None;
    };
    checksum_in_try(&node.block.body, expected_param)
}

fn checksum_in_try<'a>(
    statements: &'a [Statement<'a>],
    expected_param: &str,
) -> Option<&'a Expression<'a>> {
    let mut candidate = None;
    let mut accumulator = None;
    for statement in statements {
        if let Statement::VariableDeclaration(declaration) = statement {
            let declarator = declaration.declarations.first()?;
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                accumulator = Some(id.name.as_str());
                candidate = declarator.init.as_ref();
            }
            continue;
        }
        let Statement::IfStatement(branch) = statement else {
            continue;
        };
        let Expression::BinaryExpression(test) = &branch.test else {
            continue;
        };
        let left = identifier_name(&test.left);
        let right = identifier_name(&test.right);
        if left == accumulator && right == Some(expected_param) {
            return candidate;
        }
    }
    None
}

impl<'a> Checksum<'a> {
    pub(super) fn dead_tail_provable(&self, machinery: &[&str]) -> bool {
        let mut terms = Vec::new();
        addition_terms(self.expression, &mut terms);
        let tail = terms.get(1..).unwrap_or_default();
        crate::template::dead_tail_provable(self.body, tail, machinery)
    }
}

pub(super) const NATIVE_ROTATE_STEP_COST: u64 = 64;

pub(super) fn checksum_step<'a, T>(outcome: Eval<'a, T>) -> EvalResult<Option<T>> {
    match outcome {
        Ok(value) => Ok(Some(value)),
        Err(abrupt) => match abrupt.into_kind() {
            AbruptKind::Bail(bail) => Err(bail),
            AbruptKind::Throw(_) => Ok(None),
        },
    }
}
