use oxc_ast::ast::{
    AccessorProperty, BinaryExpression, Decorator, Expression, ImportDeclaration, ImportExpression,
    Program, RegExpLiteral, UnaryExpression, UpdateExpression, VariableDeclaration, WithClause,
    WithClauseKeyword,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::OxcDiagnostic;
use oxc_regular_expression::ast::{CapturingGroup, IgnoreGroup};
use oxc_regular_expression::visit::{walk, Visit};
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator, UpdateOperator};

const NODE: &str = "node 22 does not read";

#[derive(Default)]
struct Groups {
    names: Vec<String>,
    modifiers: bool,
}

impl<'a> Visit<'a> for Groups {
    fn visit_capturing_group(&mut self, it: &CapturingGroup<'a>) {
        if let Some(name) = &it.name {
            self.names.push(name.to_string());
        }
        walk::walk_capturing_group(self, it);
    }

    fn visit_ignore_group(&mut self, it: &IgnoreGroup<'a>) {
        self.modifiers |= it.modifiers.is_some();
        walk::walk_ignore_group(self, it);
    }
}

fn pattern_problem(literal: &RegExpLiteral<'_>) -> Option<&'static str> {
    let pattern = literal.regex.pattern.pattern.as_ref()?;
    let mut groups = Groups::default();
    groups.visit_pattern(pattern);
    if groups.modifiers {
        return Some("regular expression modifiers");
    }
    let mut names = groups.names;
    names.sort_unstable();
    let duplicated = names.windows(2).any(|pair| pair[0] == pair[1]);
    duplicated.then_some("a capture group name used twice in a regular expression")
}

struct Walk<'s> {
    source: &'s str,
    module: bool,
    found: Vec<OxcDiagnostic>,
}

impl Walk<'_> {
    fn unread(&mut self, what: &str, span: Span) {
        self.found
            .push(OxcDiagnostic::error(format!("{NODE} {what}")).with_label(span));
    }

    fn opens_html_comment(&self, binary: &BinaryExpression<'_>) -> bool {
        if binary.operator != BinaryOperator::LessThan {
            return false;
        }
        let Expression::UnaryExpression(not) = &binary.right else {
            return false;
        };
        let decremented = matches!(
            &not.argument,
            Expression::UpdateExpression(update) if is_prefix_decrement(update)
        );
        let start = not.span.start as usize;
        decremented
            && is_logical_not(not)
            && start > 0
            && self.source.as_bytes().get(start - 1) == Some(&b'<')
            && self
                .source
                .get(start..)
                .is_some_and(|rest| rest.starts_with("!--"))
    }
}

fn is_logical_not(unary: &UnaryExpression<'_>) -> bool {
    unary.operator == UnaryOperator::LogicalNot
}

fn is_prefix_decrement(update: &UpdateExpression<'_>) -> bool {
    update.prefix && update.operator == UpdateOperator::Decrement
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
        if let Some(problem) = pattern_problem(it) {
            self.unread(problem, it.span);
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if self.module && self.opens_html_comment(it) {
            self.found.push(
                OxcDiagnostic::error(
                    "node reads `<!--` as an HTML comment opener, which a module may not hold",
                )
                .with_label(it.span),
            );
        }
        walk_js::walk_binary_expression(self, it);
    }
}

pub fn unread_by_node(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut walk = Walk {
        source: program.source_text,
        module: program.source_type.is_module(),
        found: Vec::new(),
    };
    walk.visit_program(program);
    walk.found
}
