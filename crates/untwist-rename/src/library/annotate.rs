use oxc_allocator::Allocator;
use oxc_ast::ast::CommentNewlines;
use oxc_ast::ast::*;
use oxc_ast::{Comment, CommentKind, CommentPosition};
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;

use super::shape::UnitKind;
use super::table::Role;
use super::Hit;

fn module_list(modules: &[String]) -> String {
    modules
        .iter()
        .map(|module| format!("{module}.js"))
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn comment_text(hit: &Hit) -> String {
    let mut text = format!(
        "// {} {}: {}",
        hit.label,
        hit.range(),
        module_list(&hit.modules)
    );
    for same in &hit.also {
        text.push_str(&format!(
            ", same code in {} {}: {}",
            same.label,
            same.range(),
            module_list(&same.modules)
        ));
    }
    text
}

fn function_span(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

fn carried_by_expression(expr: &Expression<'_>, out: &mut Vec<Span>) {
    if let Some(span) = function_span(expr) {
        out.push(span);
        return;
    }
    match expr.without_parentheses() {
        Expression::AssignmentExpression(assignment) => {
            carried_by_expression(&assignment.right, out)
        }
        Expression::LogicalExpression(logical) => {
            carried_by_expression(&logical.left, out);
            carried_by_expression(&logical.right, out);
        }
        Expression::ConditionalExpression(conditional) => {
            carried_by_expression(&conditional.consequent, out);
            carried_by_expression(&conditional.alternate, out);
        }
        Expression::SequenceExpression(sequence) => {
            for part in &sequence.expressions {
                carried_by_expression(part, out);
            }
        }
        Expression::CallExpression(call) => {
            if let Some(span) = function_span(&call.callee) {
                out.push(span);
            }
            for argument in &call.arguments {
                if let Some(argument) = argument.as_expression() {
                    carried_by_argument(argument, out);
                }
            }
        }
        _ => {}
    }
}

fn carried_by_argument(expr: &Expression<'_>, out: &mut Vec<Span>) {
    if let Some(span) = function_span(expr) {
        out.push(span);
        return;
    }
    if let Expression::ObjectExpression(object) = expr.without_parentheses() {
        if object.properties.len() == 1 {
            if let Some(ObjectPropertyKind::ObjectProperty(property)) = object.properties.first() {
                if let Some(span) = function_span(&property.value) {
                    out.push(span);
                }
            }
        }
    }
}

fn carried(statement: &Statement<'_>) -> Vec<Span> {
    let mut out = Vec::new();
    match statement {
        Statement::FunctionDeclaration(function) => out.push(function.span),
        Statement::VariableDeclaration(declaration) if declaration.declarations.len() == 1 => {
            if let Some(init) = &declaration.declarations[0].init {
                carried_by_expression(init, &mut out);
            }
        }
        Statement::ExpressionStatement(expression) => {
            carried_by_expression(&expression.expression, &mut out);
        }
        Statement::ReturnStatement(ret) => {
            if let Some(argument) = &ret.argument {
                carried_by_expression(argument, &mut out);
            }
        }
        _ => {}
    }
    out
}

#[derive(Default)]
struct Anchors {
    starts: FxHashMap<u32, u32>,
    carriers: FxHashMap<Span, u32>,
    first_in_body: FxHashMap<Span, u32>,
}

impl<'a> Visit<'a> for Anchors {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let start = it.span().start;
        *self.starts.entry(start).or_default() += 1;
        for span in carried(it) {
            self.carriers.entry(span).or_insert(start);
        }
        walk::walk_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(first) = it.body.as_ref().and_then(|body| body.statements.first()) {
            self.first_in_body.insert(it.span, first.span().start);
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if let ArrowFunctionBody::FunctionBody(body) = &it.body {
            if let Some(first) = body.statements.first() {
                self.first_in_body.insert(it.span, first.span().start);
            }
        }
        walk::walk_arrow_function_expression(self, it);
    }
}

impl Anchors {
    fn unique(&self, start: u32) -> Option<u32> {
        (self.starts.get(&start) == Some(&1)).then_some(start)
    }

    fn anchor(&self, program: &Program<'_>, hit: &Hit) -> Option<u32> {
        if hit.kind == UnitKind::Program {
            return program
                .body
                .first()
                .and_then(|first| self.unique(first.span().start));
        }
        let carried = self
            .carriers
            .get(&hit.span)
            .and_then(|&start| self.unique(start));
        if carried.is_some() || hit.role == Role::Function {
            return carried;
        }
        self.first_in_body
            .get(&hit.span)
            .and_then(|&start| self.unique(start))
    }
}

fn already_there(program: &Program<'_>, start: u32, text: &str) -> bool {
    program.comments.iter().any(|comment| {
        comment.attached_to == start
            && program
                .source_text
                .get(comment.span.start as usize..comment.span.end as usize)
                == Some(text)
    })
}

pub fn annotate<'a>(program: &mut Program<'a>, hits: &[&Hit], alloc: &'a Allocator) -> usize {
    let mut anchors = Anchors::default();
    anchors.visit_program(program);
    let mut planned: Vec<(u32, String)> = Vec::new();
    for hit in hits {
        let Some(start) = anchors.anchor(program, hit) else {
            continue;
        };
        let text = comment_text(hit);
        if already_there(program, start, &text) || planned.iter().any(|(at, _)| *at == start) {
            continue;
        }
        planned.push((start, text));
    }
    if planned.is_empty() {
        return 0;
    }
    let mut source = String::from(program.source_text);
    let mut comments = Vec::with_capacity(planned.len());
    for (start, text) in &planned {
        source.push('\n');
        let begin = u32::try_from(source.len()).unwrap_or(u32::MAX);
        source.push_str(text);
        let end = u32::try_from(source.len()).unwrap_or(u32::MAX);
        let mut comment = Comment::new(begin, end, CommentKind::Line);
        comment.attached_to = *start;
        comment.position = CommentPosition::Leading;
        comment.newlines = CommentNewlines::Leading | CommentNewlines::Trailing;
        comments.push(comment);
    }
    program.source_text = alloc.alloc_str(&source);
    program.comments.extend(comments);
    planned.len()
}
