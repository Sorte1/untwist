use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, CallExpression, Expression, Function, Program, Statement};
use oxc_ast_visit::Visit;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::GetSpan;
use untwist_core::parse::{has_error, parse_with, SourceType};

use super::decode::{
    argument_refusal, fold_texts, outer_call, plain_arguments, refuse, sloppy_script, FreeNames,
};
use super::model::{DecodeCx, Decoded, Layer, LayerForm, Refusal, Refused};
use super::probe::Probe;

const HEAD: &str = "(function anonymous(";

const NAME: &str = "anonymous";

pub fn decode(
    program: &Program<'_>,
    scoping: &Scoping,
    probe: Probe<'_, '_>,
    cx: &DecodeCx,
) -> Decoded {
    let call = outer_call(&probe)?;
    Some(decode_call(program, scoping, call, cx))
}

fn decode_call(
    program: &Program<'_>,
    scoping: &Scoping,
    call: &CallExpression<'_>,
    cx: &DecodeCx,
) -> Result<Layer, Refused> {
    let form = LayerForm::Function;
    let Some(inner) = constructor_arguments(&call.callee) else {
        return refuse(form, Refusal::Shape);
    };
    let exprs = match plain_arguments(inner) {
        Some(exprs) if !call.optional && sloppy_script(program) => exprs,
        _ => return refuse(form, Refusal::Shape),
    };
    if let Some(reason) = argument_refusal(scoping, &["Function"], &exprs) {
        return refuse(form, reason);
    }
    let (texts, steps, heap) = match fold_texts(&exprs, cx) {
        Ok(folded) => folded,
        Err(reason) => return refuse(form, reason),
    };
    let args = arguments_text(program.source_text, &call.arguments);
    let Some(text) = joined(&texts, args, call.arguments.len()) else {
        return refuse(form, Refusal::Injection);
    };
    Ok(Layer {
        form,
        text,
        steps,
        heap,
    })
}

fn constructor_arguments<'p, 'a>(callee: &'p Expression<'a>) -> Option<&'p [Argument<'a>]> {
    let (constructor, arguments) = match callee.without_parentheses() {
        Expression::CallExpression(inner) if !inner.optional => (&inner.callee, &inner.arguments),
        Expression::NewExpression(inner) => (&inner.callee, &inner.arguments),
        _ => return None,
    };
    matches!(constructor.without_parentheses(), Expression::Identifier(id) if id.name == "Function")
        .then_some(arguments.as_slice())
}

fn arguments_text<'s>(source: &'s str, arguments: &[Argument<'_>]) -> &'s str {
    let (Some(first), Some(last)) = (arguments.first(), arguments.last()) else {
        return "";
    };
    let start = first.span().start as usize;
    let end = last.span().end as usize;
    source.get(start..end).unwrap_or("")
}

fn joined(texts: &[String], args: &str, arity: usize) -> Option<String> {
    let (body, params) = texts.split_last()?;
    let params = params.join(",");
    let text = format!("{HEAD}{params}\n) {{\n{body}\n}})({args});");
    let shape = Shape {
        params: params.len(),
        body: body.len(),
        arity,
    };
    shape.holds(&text).then_some(text)
}

struct Shape {
    params: usize,
    body: usize,
    arity: usize,
}

impl Shape {
    fn holds(&self, text: &str) -> bool {
        let alloc = Allocator::default();
        let parsed = parse_with(&alloc, text, SourceType::script());
        if has_error(&parsed.diagnostics) || !parsed.program.directives.is_empty() {
            return false;
        }
        let [Statement::ExpressionStatement(only)] = parsed.program.body.as_slice() else {
            return false;
        };
        let Expression::CallExpression(call) = &only.expression else {
            return false;
        };
        let Expression::FunctionExpression(function) = &call.callee else {
            return false;
        };
        call.arguments.len() == self.arity
            && self.spans_hold(function)
            && !names_itself(function)
            && SemanticBuilder::new()
                .with_check_syntax_error(true)
                .build(&parsed.program)
                .diagnostics
                .is_empty()
    }

    fn spans_hold(&self, function: &Function<'_>) -> bool {
        let open = HEAD.len() - 1;
        let params_end = HEAD.len() + self.params + 2;
        let body_start = params_end + 1;
        let body_end = body_start + self.body + 4;
        let Some(body) = &function.body else {
            return false;
        };
        let named = function.id.as_ref().is_some_and(|id| id.name == NAME);
        named
            && !function.r#async
            && !function.generator
            && span_is(function.params.span(), open, params_end)
            && span_is(body.span(), body_start, body_end)
    }
}

fn span_is(span: oxc_span::Span, start: usize, end: usize) -> bool {
    span.start as usize == start && span.end as usize == end
}

fn names_itself(function: &Function<'_>) -> bool {
    let mut names = FreeNames::default();
    names.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        names.visit_function_body(body);
    }
    names.names.contains(&NAME)
}
