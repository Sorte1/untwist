use oxc_allocator::{ReplaceWith, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::pass::{Cx, Interest, Locality, Pass, StatementKind, StatementVariants};

#[derive(Default)]
pub struct UnaryExpressions;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UnaryExpressions)
}

fn is_discardable_unary(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    let Expression::UnaryExpression(unary) = expr else {
        return false;
    };
    match unary.operator {
        UnaryOperator::Void | UnaryOperator::LogicalNot => true,
        UnaryOperator::Typeof => !is_unresolved_identifier(&unary.argument, cx),
        _ => false,
    }
}

fn is_unresolved_identifier(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    id.reference_id
        .get()
        .and_then(|reference| cx.scoping().get_reference(reference).symbol_id())
        .is_none()
}

fn is_void_return(stmt: &Statement<'_>) -> bool {
    let Statement::ReturnStatement(ret) = stmt else {
        return false;
    };
    matches!(
        &ret.argument,
        Some(Expression::UnaryExpression(unary)) if unary.operator == UnaryOperator::Void
    )
}

fn split_void_return<'a>(
    stmt: Statement<'a>,
    cx: &mut Cx<'a, '_>,
) -> Result<(Statement<'a>, Statement<'a>), Statement<'a>> {
    let ret = match stmt {
        Statement::ReturnStatement(ret) => ret.unbox(),
        other => return Err(other),
    };
    let span = ret.span;
    let argument = match ret.argument {
        Some(Expression::UnaryExpression(unary)) if unary.operator == UnaryOperator::Void => {
            unary.unbox().argument
        }
        argument => return Err(Statement::new_return_statement(span, argument, cx)),
    };
    let argument_span = argument.span();
    Ok((
        Statement::new_expression_statement(argument_span, argument, cx),
        Statement::new_return_statement(span, None, cx),
    ))
}

const ACTED_ON_STATEMENTS: StatementVariants =
    StatementVariants::of(&[StatementKind::ExpressionStatement]);

impl<'a> Pass<'a> for UnaryExpressions {
    fn name(&self) -> &'static str {
        "unary-expressions"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        (Interest::EXIT_STATEMENT | Interest::EXIT_STATEMENTS).on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        let Statement::ExpressionStatement(statement) = stmt else {
            return;
        };
        if !is_discardable_unary(&statement.expression, cx) {
            return;
        }
        statement.expression.replace_with(|old| match old {
            Expression::UnaryExpression(unary) => unary.unbox().argument,
            other => other,
        });
        cx.changed();
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if !stmts.iter().any(is_void_return) {
            return;
        }
        let old = std::mem::replace(stmts, AVec::new_in(cx));
        for stmt in old {
            match split_void_return(stmt, cx) {
                Ok((expression, ret)) => {
                    stmts.push(expression);
                    stmts.push(ret);
                    cx.changed();
                }
                Err(unchanged) => stmts.push(unchanged),
            }
        }
    }
}
