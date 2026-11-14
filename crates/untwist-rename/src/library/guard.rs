use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

pub struct InlinedWrapper<'s, 'a> {
    pub guard: &'s IfStatement<'a>,
    pub flag: SymbolId,
    pub exports: SymbolId,
    pub tail: &'s [Statement<'a>],
}

fn symbol_of(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return None;
    };
    crate::collect::reference_symbol(id, scoping)
}

fn returned(statement: &Statement<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match statement {
        Statement::ReturnStatement(ret) => symbol_of(ret.argument.as_ref()?, scoping),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => returned(only, scoping),
            _ => None,
        },
        _ => None,
    }
}

fn sets_flag(statement: &Statement<'_>, flag: SymbolId, scoping: &Scoping) -> bool {
    let Statement::ExpressionStatement(expression) = statement else {
        return false;
    };
    let first = match expression.expression.without_parentheses() {
        Expression::SequenceExpression(sequence) => &sequence.expressions[0],
        other => other,
    };
    let Expression::AssignmentExpression(assignment) = first.without_parentheses() else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return false;
    };
    assignment.operator == AssignmentOperator::Assign
        && crate::collect::reference_symbol(target, scoping) == Some(flag)
}

fn guarded_tail<'s, 'a>(
    guard: &'s IfStatement<'a>,
    rest: &'s [Statement<'a>],
) -> Option<&'s [Statement<'a>]> {
    match (&guard.alternate, rest) {
        (None, rest) => Some(rest),
        (Some(Statement::BlockStatement(block)), []) => Some(block.body.as_slice()),
        (Some(alternate), []) => Some(std::slice::from_ref(alternate)),
        _ => None,
    }
}

pub fn wrapper_guard<'s, 'a>(
    statements: &'s [Statement<'a>],
    scoping: &Scoping,
) -> Option<InlinedWrapper<'s, 'a>> {
    let (Statement::IfStatement(guard), rest) = statements.split_first()? else {
        return None;
    };
    let flag = symbol_of(&guard.test, scoping)?;
    let exports = returned(&guard.consequent, scoping)?;
    let tail = guarded_tail(guard, rest)?;
    if !sets_flag(tail.first()?, flag, scoping) {
        return None;
    }
    Some(InlinedWrapper {
        guard,
        flag,
        exports,
        tail,
    })
}

pub fn inlined_wrapper<'s, 'a>(
    statements: &'s [Statement<'a>],
    scoping: &Scoping,
) -> Option<InlinedWrapper<'s, 'a>> {
    let wrapper = wrapper_guard(statements, scoping)?;
    match wrapper.tail.last()? {
        Statement::ReturnStatement(_) => None,
        _ => Some(wrapper),
    }
}

pub fn assigned_to(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    match assignment.operator {
        AssignmentOperator::Assign => crate::collect::reference_symbol(target, scoping),
        _ => None,
    }
}

pub fn called_wrapper<'c, 'a>(
    call: &'c CallExpression<'a>,
    scoping: &Scoping,
) -> Option<(SymbolId, &'c Function<'a>)> {
    if !call.arguments.is_empty() || call.optional {
        return None;
    }
    let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
        return None;
    };
    if function.r#async
        || function.generator
        || !function.params.items.is_empty()
        || function.params.rest.is_some()
    {
        return None;
    }
    let body = function.body.as_ref()?;
    let wrapper = wrapper_guard(&body.statements, scoping)?;
    Some((wrapper.flag, function))
}
