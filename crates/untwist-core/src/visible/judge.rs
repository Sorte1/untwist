use oxc_ast::ast::{
    JSXAttributeValue, JSXText, Program, RegExpLiteral, StringLiteral, TaggedTemplateExpression,
    TemplateLiteral,
};
use oxc_ast_visit::{walk, Visit};

use super::is_hidden;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Respelling {
    #[default]
    Plain,
    Text,
    Parse,
}

fn odd(text: &str) -> bool {
    text.chars().any(|ch| is_hidden(ch) || ch == '\t')
}

fn breaks_lines(text: &str) -> bool {
    text.contains(['\r', '\u{2028}', '\u{2029}'])
}

pub(super) fn judged(program: &Program<'_>, hidden: bool) -> Respelling {
    let mut judge = Judge {
        parse: false,
        strings: false,
    };
    judge.visit_program(program);
    let source = program.source_text;
    let commented = program
        .comments
        .iter()
        .filter_map(|comment| source.get(comment.span.start as usize..comment.span.end as usize))
        .any(odd);
    let banged = program
        .hashbang
        .as_ref()
        .is_some_and(|hashbang| odd(&hashbang.value));
    match (hidden || judge.strings, judge.parse || commented || banged) {
        (false, _) => Respelling::Plain,
        (true, true) => Respelling::Parse,
        (true, false) => Respelling::Text,
    }
}

struct Judge {
    parse: bool,
    strings: bool,
}

impl<'a> Visit<'a> for Judge {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.strings |= it.value.as_str().is_some_and(odd);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.parse |= it.quasis.iter().any(|quasi| {
            let raw = quasi.value.raw.as_str();
            raw.contains('\t') || breaks_lines(raw)
        });
        self.visit_expressions(&it.expressions);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.parse |= it
            .quasi
            .quasis
            .iter()
            .any(|quasi| odd(quasi.value.raw.as_str()));
        self.visit_expression(&it.tag);
        self.visit_expressions(&it.quasi.expressions);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        self.parse |= it.regex.pattern.text.contains('\t');
    }

    fn visit_jsx_text(&mut self, it: &JSXText<'a>) {
        self.parse |= odd(&it.value);
    }

    fn visit_jsx_attribute_value(&mut self, it: &JSXAttributeValue<'a>) {
        match it {
            JSXAttributeValue::StringLiteral(literal) => {
                self.parse |= literal.value.as_str().is_none_or(odd);
            }
            other => walk::walk_jsx_attribute_value(self, other),
        }
    }
}
