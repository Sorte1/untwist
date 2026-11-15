use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use untwist_core::ast_util::js_text::js_text;

use super::roles::reflective_access;

const REFLECTIVE_ROOTS: [&str; 3] = ["Reflect", "Object", "Proxy"];

const RUNNER_KEYS: [&str; 10] = [
    "eval",
    "Function",
    "execScript",
    "setTimeout",
    "setInterval",
    "constructor",
    "runInThisContext",
    "runInNewContext",
    "runInContext",
    "compileFunction",
];

pub(super) fn may_hand_back_a_runner(program: &Program<'_>) -> bool {
    if untwist_core::ast_util::code_text::may_run_code_text(program, &Scoping::default()) {
        return true;
    }
    let mut route = RunnerRoute::default();
    route.visit_program(program);
    route.found
}

fn names_a_route(name: &str) -> bool {
    REFLECTIVE_ROOTS.contains(&name) || RUNNER_KEYS.contains(&name) || reflective_access(name)
}

#[derive(Default)]
struct RunnerRoute {
    found: bool,
}

impl RunnerRoute {
    fn note_piece(&mut self, text: &str) {
        let piece = !text.is_empty() && RUNNER_KEYS.iter().any(|key| key.contains(text));
        self.found |= piece || names_a_route(text);
    }
}

impl<'a> VisitJs<'a> for RunnerRoute {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= names_a_route(&it.name);
    }

    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.found |= names_a_route(&it.name);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.note_piece(&js_text(it.value));
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.found |= !it.expressions.is_empty();
        walk_js::walk_template_literal(self, it);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        if let Some(cooked) = it.value.cooked {
            self.note_piece(&js_text(cooked));
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let literal_key = matches!(
            it.expression.without_parentheses(),
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        );
        self.found |= !literal_key;
        walk_js::walk_computed_member_expression(self, it);
    }
}
