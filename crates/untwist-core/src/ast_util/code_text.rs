use oxc_ast::ast::{
    Argument, CallExpression, Expression, MemberExpression, NewExpression, Program,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;

const CODE_RUNNERS: [&str; 3] = ["eval", "Function", "execScript"];
const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];
const CONSTRUCTOR: &str = "constructor";

struct CodeText<'s> {
    scoping: &'s Scoping,
    found: bool,
}

pub fn may_run_code_text(program: &Program<'_>, scoping: &Scoping) -> bool {
    let mut scan = CodeText {
        scoping,
        found: false,
    };
    scan.visit_program(program);
    scan.found
}

fn is_free(scoping: &Scoping, expression: &Expression<'_>, names: &[&str]) -> bool {
    let Expression::Identifier(id) = expression.without_parentheses() else {
        return false;
    };
    names.contains(&id.name.as_str())
        && id
            .reference_id
            .get()
            .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

fn names_constructor(expression: &Expression<'_>) -> bool {
    let Some(member) = expression.without_parentheses().as_member_expression() else {
        return false;
    };
    match member {
        MemberExpression::StaticMemberExpression(it) => it.property.name == CONSTRUCTOR,
        MemberExpression::ComputedMemberExpression(it) => match &it.expression {
            Expression::StringLiteral(literal) => literal.value.as_str() == Some(CONSTRUCTOR),
            _ => false,
        },
        MemberExpression::PrivateFieldExpression(_) => false,
    }
}

fn calls_back(first: Option<&Argument<'_>>) -> bool {
    matches!(
        first
            .and_then(Argument::as_expression)
            .map(Expression::without_parentheses),
        Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_))
    )
}

impl CodeText<'_> {
    fn runs_text(&self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        let direct_eval = matches!(
            callee.without_parentheses(),
            Expression::Identifier(id) if id.name == "eval"
        );
        direct_eval
            || names_constructor(callee)
            || (is_free(self.scoping, callee, &TIMERS) && !calls_back(arguments.first()))
    }
}

impl<'a> VisitJs<'a> for CodeText<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.found {
            return;
        }
        if is_free(self.scoping, it, &CODE_RUNNERS) {
            self.found = true;
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.runs_text(&it.callee, &it.arguments) {
            self.found = true;
            return;
        }
        if is_free(self.scoping, &it.callee, &TIMERS) {
            self.visit_arguments(&it.arguments);
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if names_constructor(&it.callee) {
            self.found = true;
            return;
        }
        walk_js::walk_new_expression(self, it);
    }
}
