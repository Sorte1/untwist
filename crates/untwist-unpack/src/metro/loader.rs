use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;

use super::layout::Layout;
use crate::canon::statement_shape;

const KNOWN: [&str; 2] = [
    include_str!("runtimes/require.txt"),
    include_str!("runtimes/require-default.txt"),
];
const DEFINE_SUFFIX: &str = "__d";
const RUN: &str = "__r";
const PREFIX_NAME: &str = "__METRO_GLOBAL_PREFIX__";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loader {
    Known,
    Foreign,
    Absent,
}

fn names_loader(text: &str) -> bool {
    text.ends_with(DEFINE_SUFFIX) || text.ends_with(RUN)
}

struct Mentions {
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= names_loader(it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= names_loader(it.name.as_str());
    }

    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.found |= names_loader(it.name.as_str());
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.found |= it.value.as_str().is_none_or(names_loader);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        self.found |= names_loader(it.value.raw.as_str());
    }
}

fn mentions(statement: &Statement<'_>) -> bool {
    let mut mentions = Mentions { found: false };
    mentions.visit_statement(statement);
    mentions.found
}

fn is_known(statement: &Statement<'_>, source_text: &str) -> bool {
    let Statement::ExpressionStatement(expression) = statement else {
        return false;
    };
    let Expression::CallExpression(call) = &expression.expression else {
        return false;
    };
    matches!(&call.callee, Expression::FunctionExpression(_))
        && KNOWN.contains(&statement_shape(statement, source_text).as_str())
}

fn declares_prefix(statement: &Statement<'_>, prefix: &str) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.declarations.iter().any(|declarator| {
        let named = matches!(
            &declarator.id,
            BindingPattern::BindingIdentifier(id) if id.name == PREFIX_NAME
        );
        let bound = matches!(
            &declarator.init,
            Some(Expression::StringLiteral(text)) if text.value.as_str() == Some(prefix)
        );
        named && bound
    })
}

fn prefix_bound(scoping: &Scoping, before: &[Statement<'_>], prefix: &str) -> bool {
    let Some(symbol) = scoping.get_root_binding(PREFIX_NAME.into()) else {
        return false;
    };
    !scoping.symbol_is_mutated(symbol)
        && scoping.symbol_redeclarations(symbol).is_empty()
        && before
            .iter()
            .any(|statement| declares_prefix(statement, prefix))
}

fn calls_only(statements: &[Statement<'_>], define: &str) -> bool {
    statements.iter().all(|statement| {
        let Statement::ExpressionStatement(expression) = statement else {
            return true;
        };
        let Expression::CallExpression(call) = &expression.expression else {
            return true;
        };
        matches!(
            &call.callee,
            Expression::Identifier(callee) if callee.name == define || callee.name == RUN
        )
    })
}

pub fn loader(scoping: &Scoping, source: &Program<'_>, layout: &Layout<'_, '_>) -> Loader {
    let Some((prelude, rest)) = source.body.split_at_checked(layout.prelude) else {
        return Loader::Foreign;
    };
    let known: Vec<usize> = (0..prelude.len())
        .filter(|index| is_known(&prelude[*index], source.source_text))
        .collect();
    let foreign =
        (0..prelude.len()).any(|index| !known.contains(&index) && mentions(&prelude[index]));
    let prefix = layout.define.strip_suffix(DEFINE_SUFFIX).unwrap_or_default();
    match (known.as_slice(), foreign) {
        ([], false) => Loader::Absent,
        ([at], false)
            if prefix_bound(scoping, &prelude[..*at], prefix)
                && calls_only(rest, layout.define) =>
        {
            Loader::Known
        }
        _ => Loader::Foreign,
    }
}
