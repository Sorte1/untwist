use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use untwist_core::ast_util::js_text::js_text;

const CHAIN_NAMES: [&str; 8] = [
    "__proto__",
    "prototype",
    "setPrototypeOf",
    "getPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "__lookupSetter__",
    "__defineSetter__",
];

const RUNNER_NAMES: [&str; 8] = [
    "eval",
    "Function",
    "constructor",
    "execScript",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "importScripts",
];

const FUNCTION_FORM: (&str, &str) = ("(function () {\n", "\n});");

pub(super) fn text_spares_chains(text: &str, depth: u8) -> bool {
    let Some(next) = depth.checked_sub(1) else {
        return false;
    };
    let wrapped = format!("{}{text}{}", FUNCTION_FORM.0, FUNCTION_FORM.1);
    [text, wrapped.as_str()].iter().any(|form| {
        crate::proof::with_parsed_text(form, |program| {
            let mut scan = ChainText::default();
            scan.visit_program(program);
            (!scan.open).then_some(scan.nested)
        })
        .flatten()
        .flatten()
        .is_some_and(|nested| nested.iter().all(|inner| text_spares_chains(inner, next)))
    })
}

#[derive(Default)]
struct ChainText {
    open: bool,
    nested: Vec<String>,
}

impl ChainText {
    fn name(&mut self, name: &str) {
        self.open |= CHAIN_NAMES.contains(&name);
    }

    fn runner_callee(callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(id) => RUNNER_NAMES.contains(&id.name.as_str()),
            Expression::StaticMemberExpression(member) => {
                RUNNER_NAMES.contains(&member.property.name.as_str())
            }
            _ => false,
        }
    }

    fn texts(&mut self, arguments: &[Argument<'_>]) {
        for argument in arguments {
            match argument {
                Argument::StringLiteral(text) => self.nested.push(js_text(text.value).into_owned()),
                Argument::TemplateLiteral(template) if template.expressions.is_empty() => {
                    match template.quasis.first().and_then(|quasi| quasi.value.cooked) {
                        Some(cooked) => self.nested.push(js_text(cooked).into_owned()),
                        None => self.open = true,
                    }
                }
                Argument::NumericLiteral(_)
                | Argument::FunctionExpression(_)
                | Argument::ArrowFunctionExpression(_) => {}
                _ => self.open = true,
            }
        }
    }
}

impl<'a> VisitJs<'a> for ChainText {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.name(&it.name);
        self.open |= RUNNER_NAMES.contains(&it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.name(&it.name);
    }

    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.name(&it.name);
        self.open |= RUNNER_NAMES.contains(&it.name.as_str());
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.name(&js_text(it.value));
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        match it.value.cooked {
            Some(cooked) => self.name(&js_text(cooked)),
            None => self.open = true,
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let literal = matches!(
            it.expression.without_parentheses(),
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        );
        self.open |= !literal;
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.open = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.open = true;
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.open = true;
        walk_js::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if Self::runner_callee(&it.callee) {
            self.texts(&it.arguments);
            for argument in &it.arguments {
                if let Some(expr) = argument.as_expression() {
                    if matches!(
                        expr,
                        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                    ) {
                        self.visit_expression(expr);
                    }
                }
            }
            self.visit_runner_callee(&it.callee);
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if Self::runner_callee(&it.callee) {
            self.texts(&it.arguments);
            self.visit_runner_callee(&it.callee);
            return;
        }
        walk_js::walk_new_expression(self, it);
    }
}

impl ChainText {
    fn visit_runner_callee(&mut self, callee: &Expression<'_>) {
        if let Expression::StaticMemberExpression(member) = callee.without_parentheses() {
            self.visit_expression(&member.object);
        }
    }
}
