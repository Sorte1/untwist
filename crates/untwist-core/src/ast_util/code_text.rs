use oxc_ast::ast::{
    Argument, CallExpression, Expression, MemberExpression, NewExpression, Program,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::BinaryOperator;

const CODE_RUNNERS: [&str; 3] = ["eval", "Function", "execScript"];
const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];
const CONSTRUCTOR: &str = "constructor";
const NAME: &str = "name";

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

fn literal_text(expression: &Expression<'_>) -> Option<String> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str())
            .map(str::to_string),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            Some(literal_text(&binary.left)? + &literal_text(&binary.right)?)
        }
        _ => None,
    }
}

fn texts_read_no_name(arguments: &[Argument<'_>]) -> bool {
    arguments.iter().all(|argument| {
        argument
            .as_expression()
            .and_then(literal_text)
            .is_some_and(|text| !text.contains(NAME) && !text.contains('\\'))
    })
}

impl CodeText<'_> {
    fn runner(&self, callee: &Expression<'_>) -> bool {
        let direct_eval = matches!(
            callee.without_parentheses(),
            Expression::Identifier(id) if id.name == "eval"
        );
        direct_eval || names_constructor(callee) || is_free(self.scoping, callee, &CODE_RUNNERS)
    }

    fn runs_text(&self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> bool {
        if self.runner(callee) {
            return !texts_read_no_name(arguments);
        }
        is_free(self.scoping, callee, &TIMERS)
            && !calls_back(arguments.first())
            && !texts_read_no_name(arguments.get(..1).unwrap_or_default())
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
        if self.runner(&it.callee) || is_free(self.scoping, &it.callee, &TIMERS) {
            self.visit_callee_object(&it.callee);
            self.visit_arguments(&it.arguments);
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        let runner =
            names_constructor(&it.callee) || is_free(self.scoping, &it.callee, &CODE_RUNNERS);
        if runner && !texts_read_no_name(&it.arguments) {
            self.found = true;
            return;
        }
        if runner {
            self.visit_callee_object(&it.callee);
            self.visit_arguments(&it.arguments);
            return;
        }
        walk_js::walk_new_expression(self, it);
    }
}

impl<'a> CodeText<'_> {
    fn visit_callee_object(&mut self, callee: &Expression<'a>) {
        if let Some(member) = callee.without_parentheses().as_member_expression() {
            self.visit_expression(member.object());
        }
    }
}
