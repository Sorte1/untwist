use oxc_ast::ast::{
    AccessorProperty, Decorator, ImportDeclaration, ImportExpression, JSXText, Program,
    RegExpLiteral, StringLiteral, TemplateElement, VariableDeclaration, WithClause,
    WithClauseKeyword,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;

use super::regex;

const NODE: &str = "node 22 does not read";

struct Walk<'s> {
    source: &'s str,
    module: bool,
    found: Vec<OxcDiagnostic>,
    tokens: Vec<Span>,
}

impl Walk<'_> {
    fn unread(&mut self, what: &str, span: Span) {
        self.found
            .push(OxcDiagnostic::error(format!("{NODE} {what}")).with_label(span));
    }

    fn token(&mut self, span: Span) {
        if self.module {
            self.tokens.push(span);
        }
    }
}

impl<'a> VisitJs<'a> for Walk<'_> {
    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind.is_using() || it.kind.is_await_using() {
            self.unread("`using` declarations", it.span);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_decorator(&mut self, it: &Decorator<'a>) {
        self.unread("decorators", it.span);
        walk_js::walk_decorator(self, it);
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        self.unread("`accessor` fields", it.span);
        walk_js::walk_accessor_property(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        if it.phase.is_some() {
            self.unread("`import defer` or `import source`", it.span);
        }
        walk_js::walk_import_declaration(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if it.phase.is_some() {
            self.unread("`import.defer()` or `import.source()`", it.span);
        }
        walk_js::walk_import_expression(self, it);
    }

    fn visit_with_clause(&mut self, it: &WithClause<'a>) {
        if it.keyword == WithClauseKeyword::Assert {
            self.unread("`assert` import attributes", it.span);
        }
        walk_js::walk_with_clause(self, it);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        self.token(it.span);
        let flags = self
            .source
            .get(it.span.start as usize..it.span.end as usize)
            .and_then(|text| text.rsplit_once('/'))
            .map_or("", |(_, flags)| flags);
        if let Some(problem) = regex::problem(it.regex.pattern.text.as_str(), flags) {
            self.found
                .push(OxcDiagnostic::error(problem).with_label(it.span));
        }
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.token(it.span);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        self.token(it.span);
    }

    fn visit_jsx_text(&mut self, it: &JSXText<'a>) {
        self.token(it.span);
    }
}

fn html_comment_opener(program: &Program<'_>, tokens: &[Span]) -> Option<Span> {
    let source = program.source_text;
    let skipped = |at: u32| {
        program
            .comments
            .iter()
            .map(|comment| comment.span)
            .chain(program.hashbang.as_ref().map(|hashbang| hashbang.span))
            .chain(tokens.iter().copied())
            .any(|span| span.start <= at && at < span.end)
    };
    source
        .match_indices("<!--")
        .filter_map(|(at, _)| u32::try_from(at).ok())
        .find(|at| !skipped(*at))
        .map(|at| Span::sized(at, 4))
}

pub fn unread_by_node(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let module = program.source_type.is_module();
    let mut walk = Walk {
        source: program.source_text,
        module: module && program.source_text.contains("<!--"),
        found: Vec::new(),
        tokens: Vec::new(),
    };
    walk.visit_program(program);
    let opener = match walk.module {
        true => html_comment_opener(program, &walk.tokens),
        false => None,
    };
    if let Some(span) = opener {
        walk.found.push(
            OxcDiagnostic::error(
                "node reads `<!--` as an HTML comment opener, which a module may not hold",
            )
            .with_label(span),
        );
    }
    walk.found
}
