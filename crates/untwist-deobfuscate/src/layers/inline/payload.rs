use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, BindingPattern, Class, Expression, Function,
    IdentifierReference, ImportExpression, ImportMeta, LabeledStatement, NewTarget, Program,
    RegExpLiteral, ReturnStatement, Statement, StaticBlock, Super, UnaryExpression,
    VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::SemanticBuilder;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::{has_error, parse_with, SourceType};

use super::super::model::Refusal;
use super::regex::holds_everywhere;

const STRICT_PREFIX: &str = "\"use strict\";\n";

const SOURCE_COMMENTS: [&str; 4] = [
    "# sourceURL=",
    "@ sourceURL=",
    "# sourceMappingURL=",
    "@ sourceMappingURL=",
];

#[derive(Debug, Default)]
pub(super) struct Facts {
    pub var_names: Vec<String>,
    pub lexical_names: Vec<String>,
    pub own_strict: bool,
    pub deletes_a_name: bool,
    pub calls_eval: bool,
    pub labels: bool,
    pub reserved_words: bool,
    pub expression: Option<(String, bool)>,
    pub statements: String,
}

pub(super) fn read(text: &str, strict: bool) -> Result<Facts, Refusal> {
    let alloc = Allocator::default();
    let source = match strict {
        true => format!("{STRICT_PREFIX}{text}"),
        false => text.to_string(),
    };
    let offset = if strict { STRICT_PREFIX.len() } else { 0 };
    let parsed = parse_with(&alloc, &source, SourceType::script());
    if has_error(&parsed.diagnostics) || parsed.program.hashbang.is_some() {
        return Err(Refusal::Unparsable);
    }
    let program = &parsed.program;
    let checked = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(program);
    if has_error(&checked.diagnostics) {
        return Err(Refusal::Unparsable);
    }
    let mut scan = Scan::default();
    scan.visit_program(program);
    if scan.refused {
        return Err(Refusal::Shape);
    }
    let own_strict = program
        .directives
        .iter()
        .skip(usize::from(strict))
        .any(|directive| directive.directive == "use strict");
    let expression = single_expression(program, strict, &source);
    Ok(Facts {
        var_names: scan.var_names,
        lexical_names: lexical_names(program),
        own_strict,
        deletes_a_name: scan.deletes_a_name,
        calls_eval: scan.calls_eval,
        labels: scan.labels,
        reserved_words: scan.reserved_words,
        expression,
        statements: without_source_comments(&source, offset, program),
    })
}

fn single_expression(program: &Program<'_>, strict: bool, source: &str) -> Option<(String, bool)> {
    let [Statement::ExpressionStatement(only)] = program.body.as_slice() else {
        return None;
    };
    if program.directives.len() != usize::from(strict) {
        return None;
    }
    let text = slice(source, only.span).trim_end();
    let text = text.strip_suffix(';').unwrap_or(text);
    if is_anonymous_definition(&only.expression) {
        return None;
    }
    Some((text.to_string(), keeps_its_meaning_bare(&only.expression)))
}

fn is_anonymous_definition(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::FunctionExpression(function) => function.id.is_none(),
        Expression::ClassExpression(class) => class.id.is_none(),
        Expression::ArrowFunctionExpression(_) => true,
        _ => false,
    }
}

fn keeps_its_meaning_bare(expression: &Expression<'_>) -> bool {
    !matches!(
        expression.without_parentheses(),
        Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_)
            | Expression::ChainExpression(_)
    )
}

fn slice(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}

fn is_source_comment(text: &str) -> bool {
    let body = text
        .strip_prefix("//")
        .or_else(|| text.strip_prefix("/*"))
        .unwrap_or_default();
    SOURCE_COMMENTS
        .iter()
        .any(|marker| body.starts_with(marker))
}

fn without_source_comments(source: &str, offset: usize, program: &Program<'_>) -> String {
    let mut out = String::with_capacity(source.len().saturating_sub(offset));
    let mut at = offset;
    for comment in &program.comments {
        let (start, end) = (comment.span.start as usize, comment.span.end as usize);
        if start < at {
            continue;
        }
        let text = source.get(start..end).unwrap_or_default();
        if !is_source_comment(text) {
            continue;
        }
        out.push_str(source.get(at..start).unwrap_or_default());
        let breaks = text.contains(['\n', '\r', '\u{2028}', '\u{2029}']);
        out.push(if breaks { '\n' } else { ' ' });
        at = end;
    }
    out.push_str(source.get(at..).unwrap_or_default());
    out.trim_end().to_string()
}

fn lexical_names(program: &Program<'_>) -> Vec<String> {
    let mut names = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::VariableDeclaration(decl) if decl.kind != VariableDeclarationKind::Var => {
                for declarator in &decl.declarations {
                    pattern_names(&declarator.id, &mut names);
                }
            }
            Statement::ClassDeclaration(class) => {
                names.extend(class.id.as_ref().map(|id| id.name.to_string()));
            }
            _ => {}
        }
    }
    names
}

fn pattern_names(pattern: &BindingPattern<'_>, names: &mut Vec<String>) {
    let mut collect = Bindings::default();
    collect.visit_binding_pattern(pattern);
    names.extend(collect.names);
}

#[derive(Default)]
struct Bindings {
    names: Vec<String>,
}

impl<'a> Visit<'a> for Bindings {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.push(it.name.to_string());
    }
}

#[derive(Default)]
struct Scan {
    depth: u32,
    var_names: Vec<String>,
    deletes_a_name: bool,
    calls_eval: bool,
    labels: bool,
    reserved_words: bool,
    refused: bool,
}

impl Scan {
    fn note_name(&mut self, name: &str) {
        self.reserved_words |= matches!(name, "await" | "yield");
        self.calls_eval |= name == "eval";
    }
}

impl<'a> Visit<'a> for Scan {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.depth == 0 && it.is_declaration() {
            self.var_names
                .extend(it.id.as_ref().map(|id| id.name.to_string()));
        }
        self.depth += 1;
        walk::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.depth += 1;
        walk::walk_class(self, it);
        self.depth -= 1;
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.depth += 1;
        walk::walk_static_block(self, it);
        self.depth -= 1;
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if self.depth == 0 && it.kind == VariableDeclarationKind::Var {
            for declarator in &it.declarations {
                pattern_names(&declarator.id, &mut self.var_names);
            }
        }
        walk::walk_variable_declaration(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.refused |= self.depth == 0;
        walk::walk_return_statement(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let named = matches!(it.argument.without_parentheses(), Expression::Identifier(_));
        self.deletes_a_name |= it.operator == UnaryOperator::Delete && named;
        walk::walk_unary_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.note_name(it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.note_name(it.name.as_str());
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.labels = true;
        self.note_name(it.label.name.as_str());
        walk::walk_labeled_statement(self, it);
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.refused = true;
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.refused = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.refused = true;
    }

    fn visit_import_expression(&mut self, _it: &ImportExpression<'a>) {
        self.refused = true;
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        let flags = it
            .raw
            .as_ref()
            .and_then(|raw| raw.as_str().rsplit_once('/'))
            .map(|(_, flags)| flags);
        self.refused |= !flags.is_some_and(|flags| holds_everywhere(&it.regex.pattern.text, flags));
    }
}
