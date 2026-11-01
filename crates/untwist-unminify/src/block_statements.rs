use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use untwist_core::pass::{Cx, Interest, Pass, StatementKind, StatementVariants};

#[derive(Default)]
pub struct BlockStatements;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(BlockStatements)
}

fn needs_block(stmt: &Statement<'_>) -> bool {
    !matches!(
        stmt,
        Statement::BlockStatement(_) | Statement::EmptyStatement(_)
    )
}

fn loop_body<'a, 'b>(stmt: &'b mut Statement<'a>) -> Option<&'b mut Statement<'a>> {
    match stmt {
        Statement::DoWhileStatement(node) => Some(&mut node.body),
        Statement::ForStatement(node) => Some(&mut node.body),
        Statement::ForInStatement(node) => Some(&mut node.body),
        Statement::ForOfStatement(node) => Some(&mut node.body),
        Statement::WhileStatement(node) => Some(&mut node.body),
        _ => None,
    }
}

fn loop_scope_id(stmt: &Statement<'_>) -> Option<ScopeId> {
    match stmt {
        Statement::ForStatement(node) => node.scope_id.get(),
        Statement::ForInStatement(node) => node.scope_id.get(),
        Statement::ForOfStatement(node) => node.scope_id.get(),
        _ => None,
    }
}

fn wrap_in_block<'a>(slot: &mut Statement<'a>, parent_scope: ScopeId, cx: &mut Cx<'a, '_>) {
    let scope_id = cx.tcx.insert_scope_below_statement_from_scope_id(
        &*slot,
        parent_scope,
        ScopeFlags::empty(),
    );
    let placeholder = Statement::new_empty_statement(SPAN, &*cx);
    let inner = std::mem::replace(slot, placeholder);
    let span = inner.span();
    let body = AVec::from_iter_in([inner], &*cx);
    *slot = Statement::new_block_statement_with_scope_id(span, body, scope_id, &*cx);
}

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[
    StatementKind::IfStatement,
    StatementKind::DoWhileStatement,
    StatementKind::ForStatement,
    StatementKind::ForInStatement,
    StatementKind::ForOfStatement,
    StatementKind::WhileStatement,
]);

impl<'a> Pass<'a> for BlockStatements {
    fn name(&self) -> &'static str {
        "block-statements"
    }

    fn interest(&self) -> Interest {
        (Interest::EXIT_STATEMENT | Interest::EXIT_ARROW_FUNCTION_EXPRESSION)
            .on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        let current_scope = cx.current_scope_id();
        if let Statement::IfStatement(if_stmt) = stmt {
            if needs_block(&if_stmt.consequent) {
                wrap_in_block(&mut if_stmt.consequent, current_scope, cx);
                cx.changed();
            }
            if let Some(alternate) = if_stmt.alternate.as_mut() {
                if !matches!(
                    alternate,
                    Statement::BlockStatement(_) | Statement::IfStatement(_)
                ) {
                    wrap_in_block(alternate, current_scope, cx);
                    cx.changed();
                }
            }
            return;
        }
        let parent_scope = loop_scope_id(stmt).unwrap_or(current_scope);
        let Some(body) = loop_body(stmt) else {
            return;
        };
        if !needs_block(body) {
            return;
        }
        wrap_in_block(body, parent_scope, cx);
        cx.changed();
    }

    fn exit_arrow_function_expression(
        &mut self,
        arrow: &mut ArrowFunctionExpression<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        if !matches!(arrow.body, ArrowFunctionBody::SequenceExpression(_)) {
            return;
        }
        let span = arrow.body.span();
        let placeholder = ArrowFunctionBody::new_null_literal(SPAN, &*cx);
        let old_body = std::mem::replace(&mut arrow.body, placeholder);
        let ArrowFunctionBody::SequenceExpression(seq) = old_body else {
            unreachable!()
        };
        let argument = Expression::SequenceExpression(seq);
        let return_stmt = Statement::new_return_statement(span, Some(argument), &*cx);
        let statements = AVec::from_iter_in([return_stmt], &*cx);
        arrow.body =
            ArrowFunctionBody::new_function_body(span, AVec::new_in(&*cx), statements, &*cx);
        cx.changed();
    }
}
