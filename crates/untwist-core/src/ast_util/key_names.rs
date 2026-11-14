use oxc_ast::ast::{
    Argument, CallExpression, Expression, IdentifierName, ImportExpression, Program, StringLiteral,
    TemplateLiteral,
};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use rustc_hash::FxHashSet;

const REQUIRE: &str = "require";

pub fn key_names(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<String> {
    let mut keys = KeyNames {
        scoping,
        names: FxHashSet::default(),
    };
    keys.visit_program(program);
    keys.names
}

struct KeyNames<'s> {
    scoping: &'s Scoping,
    names: FxHashSet<String>,
}

impl KeyNames<'_> {
    fn free_require(&self, callee: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = callee else {
            return false;
        };
        id.name == REQUIRE
            && id.reference_id.get().is_some_and(|reference| {
                self.scoping.get_reference(reference).symbol_id().is_none()
            })
    }
}

impl<'a> Visit<'a> for KeyNames<'_> {
    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.names.insert(it.name.as_str().to_string());
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if let Some(value) = it.value.as_str() {
            self.names.insert(value.to_string());
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let required = self.free_require(&it.callee)
            && matches!(it.arguments.as_slice(), [Argument::StringLiteral(_)]);
        if required {
            return;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if matches!(&it.source, Expression::StringLiteral(_)) && it.options.is_none() {
            return;
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        for quasi in &it.quasis {
            if let Some(cooked) = quasi
                .value
                .cooked
                .as_ref()
                .and_then(|cooked| cooked.as_str())
            {
                self.names.insert(cooked.to_string());
            }
        }
        walk::walk_template_literal(self, it);
    }
}
