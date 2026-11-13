use std::sync::atomic::{AtomicBool, Ordering};

use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};

mod controller;
mod cookie;
mod helper;
mod reader;
mod state;

pub use reader::{judge_readers, source_readers, SourceReaders};

const FOLD_DEPTH: u32 = 16;
const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";

static NARROWED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn narrow_guard_shapes_for_tests(on: bool) {
    NARROWED.store(on, Ordering::SeqCst);
}

fn narrowed() -> bool {
    NARROWED.load(Ordering::Relaxed)
}

#[inline]
pub fn may_be_a_guard(function: &Function<'_>) -> bool {
    let params = function.params.items.len();
    if params > 2 {
        return false;
    }
    let Some(body) = &function.body else {
        return false;
    };
    match (params, body.statements.as_slice()) {
        (0, [.., Statement::VariableDeclaration(_), Statement::ReturnStatement(handoff)]) => {
            matches!(handoff.argument, Some(Expression::FunctionExpression(_)))
        }
        (0, [only]) => !narrowed() && returns_a_string(only),
        (0, statements) if cookie::opens_like_a_cookie(statements) => {
            !narrowed() && cookie::cookie_target(statements).is_some()
        }
        (2, statements) => {
            !narrowed()
                && controller::opens_with_a_choice(statements)
                && controller::handed_out_target(function).is_some()
        }
        (1, [Statement::ExpressionStatement(only)]) => {
            matches!(only.expression, Expression::SequenceExpression(_))
        }
        (1, [Statement::ExpressionStatement(first), _, _, _, _]) => {
            matches!(first.expression, Expression::AssignmentExpression(_))
        }
        (1, [Statement::VariableDeclaration(first), _, _, ..]) => {
            opens_with_an_alphabet(first)
                || helper::helper_target(function, &body.statements).is_some()
        }
        _ => false,
    }
}

const BAIT_WORDS: [&str; 3] = ["dev", "window", "newState"];

fn returns_a_string(statement: &Statement<'_>) -> bool {
    matches!(statement, Statement::ReturnStatement(ret)
        if matches!(&ret.argument, Some(Expression::StringLiteral(text))
            if BAIT_WORDS.iter().any(|word| text.value == *word)))
}

fn opens_with_an_alphabet(declaration: &VariableDeclaration<'_>) -> bool {
    matches!(declaration.declarations.as_slice(),
        [only] if matches!(&only.init, Some(Expression::StringLiteral(text)) if text.value.len() == ALPHABET.len()))
}

pub fn read_target<'k, 'a>(function: &'k Function<'a>) -> Option<&'k Function<'a>> {
    let statements = &function.body.as_ref()?.statements;
    if function.params.rest.is_some() {
        return None;
    }
    match (function.params.items.len(), statements.as_slice()) {
        (0, [only]) if !narrowed() && returns_a_string(only) => Some(function),
        (0, [_, _, ..]) => controller::controller_target(statements)
            .or_else(|| cookie::cookie_target(statements).filter(|_| !narrowed())),
        (1, _) => state::state_target(function, statements)
            .or_else(|| helper::helper_target(function, statements)),
        (2, _) if !narrowed() => controller::handed_out_target(function),
        _ => None,
    }
}

pub fn holder_read_target<'k, 'a>(object: &'k ObjectExpression<'a>) -> Option<&'k Function<'a>> {
    if narrowed() || object.properties.len() < 2 {
        return None;
    }
    cookie::holder_target(object)
}

fn binding_name<'k>(pattern: &'k BindingPattern<'_>) -> Option<&'k str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn param_name<'k>(function: &'k Function<'_>, index: usize) -> Option<&'k str> {
    binding_name(&function.params.items.get(index)?.pattern)
}

fn has_no_params(function: &Function<'_>) -> bool {
    function.params.items.is_empty() && function.params.rest.is_none()
}

fn statements_of<'k, 'a>(function: &'k Function<'a>) -> Option<&'k [Statement<'a>]> {
    Some(function.body.as_ref()?.statements.as_slice())
}

type Declared<'k, 'a> = (&'k str, Option<&'k Expression<'a>>);

fn declared<'k, 'a>(declarator: &'k VariableDeclarator<'a>) -> Option<Declared<'k, 'a>> {
    Some((binding_name(&declarator.id)?, declarator.init.as_ref()))
}

pub(super) fn var_declaration<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<&'k VariableDeclaration<'a>> {
    match statement {
        Statement::VariableDeclaration(declaration)
            if matches!(
                declaration.kind,
                VariableDeclarationKind::Var
                    | VariableDeclarationKind::Let
                    | VariableDeclarationKind::Const
            ) =>
        {
            Some(declaration)
        }
        _ => None,
    }
}

fn single_var<'k, 'a>(statement: &'k Statement<'a>) -> Option<Declared<'k, 'a>> {
    let declaration = var_declaration(statement)?;
    match declaration.declarations.as_slice() {
        [declarator] => declared(declarator),
        _ => None,
    }
}

fn is_named(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name == name)
}

fn folded_number(expr: &Expression<'_>, depth: u32) -> Option<f64> {
    if depth == 0 {
        return None;
    }
    match expr.without_parentheses() {
        Expression::NumericLiteral(literal) => Some(literal.value),
        Expression::UnaryExpression(unary) => {
            let value = folded_number(&unary.argument, depth - 1)?;
            match unary.operator {
                UnaryOperator::UnaryNegation => Some(-value),
                UnaryOperator::UnaryPlus => Some(value),
                _ => None,
            }
        }
        Expression::BinaryExpression(binary) => {
            let left = folded_number(&binary.left, depth - 1)?;
            let right = folded_number(&binary.right, depth - 1)?;
            match binary.operator {
                BinaryOperator::Addition => Some(left + right),
                BinaryOperator::Subtraction => Some(left - right),
                BinaryOperator::Multiplication => Some(left * right),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_number(expr: &Expression<'_>, value: f64) -> bool {
    folded_number(expr, FOLD_DEPTH) == Some(value)
}

fn is_string(expr: &Expression<'_>, value: &str) -> bool {
    matches!(expr, Expression::StringLiteral(literal) if literal.value == value)
}

fn returned<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ReturnStatement(ret) => ret.argument.as_ref(),
        _ => None,
    }
}

pub(super) fn is_reset(
    expr: &Expression<'_>,
    name: &str,
    value: fn(&Expression<'_>) -> bool,
) -> bool {
    let Expression::AssignmentExpression(assign) = expr else {
        return false;
    };
    assign.operator == AssignmentOperator::Assign
        && matches!(&assign.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == name)
        && value(&assign.right)
}

fn is_null(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::NullLiteral(_))
}
