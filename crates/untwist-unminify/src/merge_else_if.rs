use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;
use untwist_core::pass::{Cx, Interest, Locality, Pass, StatementKind, StatementVariants};

#[derive(Default)]
pub struct MergeElseIf;

fn lone_nested_if(stmt: &Statement<'_>) -> bool {
    let Statement::BlockStatement(block) = stmt else {
        return false;
    };
    block.body.len() == 1 && matches!(block.body[0], Statement::IfStatement(_))
}

fn negated_test<'a>(statement: &IfStatement<'a>) -> bool {
    matches!(&statement.test, Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::LogicalNot)
}

fn slot_declares(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::FunctionDeclaration(_) | Statement::ClassDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => declaration.kind.is_lexical(),
        Statement::IfStatement(nested) => {
            slot_declares(&nested.consequent)
                || nested.alternate.as_ref().is_some_and(slot_declares)
        }
        Statement::LabeledStatement(labeled) => slot_declares(&labeled.body),
        _ => false,
    }
}

impl MergeElseIf {
    fn merge_else(&mut self, stmt: &mut Statement<'_>, cx: &Cx<'_, '_>) -> bool {
        let Statement::IfStatement(statement) = stmt else {
            return false;
        };
        let Some(alternate) = statement.alternate.as_mut() else {
            return false;
        };
        if !lone_nested_if(alternate) {
            return false;
        }
        let Statement::BlockStatement(block) = alternate else {
            return false;
        };
        let Some(nested) = block.body.pop() else {
            return false;
        };
        if slot_declares(&nested) {
            cx.binding_moved();
        }
        *alternate = nested;
        true
    }

    fn invert<'a>(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let Statement::IfStatement(statement) = stmt else {
            return false;
        };
        if !negated_test(statement) || !lone_nested_if(&statement.consequent) {
            return false;
        }
        let Some(alternate) = statement.alternate.take() else {
            return false;
        };
        let placeholder = Expression::new_null_literal(SPAN, &*cx);
        let unary = match std::mem::replace(&mut statement.test, placeholder) {
            Expression::UnaryExpression(unary) => unary,
            other => {
                statement.test = other;
                statement.alternate = Some(alternate);
                return false;
            }
        };
        statement.test = unary.unbox().argument;
        let consequent = std::mem::replace(&mut statement.consequent, alternate);
        statement.alternate = Some(consequent);
        true
    }
}

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[StatementKind::IfStatement]);

impl<'a> Pass<'a> for MergeElseIf {
    fn name(&self) -> &'static str {
        "merge-else-if"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT.on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if self.merge_else(stmt, cx) || (cx.opts.extended && self.invert(stmt, cx)) {
            cx.changed();
        }
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(MergeElseIf)
}
