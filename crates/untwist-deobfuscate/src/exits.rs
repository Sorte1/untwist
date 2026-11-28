use oxc_ast::ast::{BreakStatement, ContinueStatement, Function, LabeledStatement, Statement};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

pub(crate) fn always_exits(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ReturnStatement(_) | Statement::ThrowStatement(_) => true,
        Statement::BlockStatement(block) => {
            block.body.iter().any(always_exits) && !block.body.iter().any(jumps)
        }
        Statement::IfStatement(branch) => {
            always_exits(&branch.consequent) && branch.alternate.as_ref().is_some_and(always_exits)
        }
        _ => false,
    }
}

fn jumps(statement: &Statement<'_>) -> bool {
    let mut scan = Jumps { found: false };
    scan.visit_statement(statement);
    scan.found
}

struct Jumps {
    found: bool,
}

impl<'a> VisitJs<'a> for Jumps {
    fn visit_break_statement(&mut self, _it: &BreakStatement<'a>) {
        self.found = true;
    }

    fn visit_continue_statement(&mut self, _it: &ContinueStatement<'a>) {
        self.found = true;
    }

    fn visit_labeled_statement(&mut self, _it: &LabeledStatement<'a>) {
        self.found = true;
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.found {
            walk_js::walk_statement(self, it);
        }
    }
}
