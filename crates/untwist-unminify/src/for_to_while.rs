use oxc_allocator::ReplaceWith;
use oxc_ast::ast::*;
use untwist_core::pass::{Cx, Interest, Pass, StatementKind, StatementVariants};

#[derive(Default)]
pub struct ForToWhile;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(ForToWhile)
}

const ACTED_ON_STATEMENTS: StatementVariants =
    StatementVariants::of(&[StatementKind::ForStatement]);

impl<'a> Pass<'a> for ForToWhile {
    fn name(&self) -> &'static str {
        "for-to-while"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_STATEMENT.on_statements(ACTED_ON_STATEMENTS)
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        let Statement::ForStatement(for_stmt) = &*stmt else {
            return;
        };
        if for_stmt.init.is_some() || for_stmt.update.is_some() {
            return;
        }
        let mut changed = false;
        stmt.replace_with(|old| {
            let for_stmt = match old {
                Statement::ForStatement(for_stmt) => for_stmt,
                other => return other,
            };
            let for_stmt = for_stmt.unbox();
            let span = for_stmt.span;
            let test = match for_stmt.test {
                Some(test) => test,
                None => Expression::new_boolean_literal(span, true, &*cx),
            };
            changed = true;
            Statement::new_while_statement(span, test, for_stmt.body, &*cx)
        });
        if changed {
            cx.changed();
        }
    }
}
