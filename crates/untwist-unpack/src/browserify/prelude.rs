use oxc_ast::ast::*;

use crate::browserify::forms::{HEAD_FORMS, PRELUDE_FORMS};
use crate::canon::function_shape;
use crate::webpack::machinery::form_digest;

pub const NAME: &str = "%NAME%";
const CALLED: &str = "()";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unproven {
    Prelude,
    Placement,
    Head,
}

impl Unproven {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Prelude => "its prelude is not one of browser-pack's",
            Self::Placement => "it runs inside other code than browserify's own output",
            Self::Head => "its standalone wrapper is not one of browserify's",
        }
    }
}

pub fn prelude_text(callee: &Expression<'_>, source_text: &str) -> Option<String> {
    match callee.without_parentheses() {
        Expression::FunctionExpression(function) => function_shape(function, source_text),
        Expression::CallExpression(call) if call.arguments.is_empty() && !call.optional => {
            let Expression::FunctionExpression(factory) = call.callee.without_parentheses() else {
                return None;
            };
            function_shape(factory, source_text).map(|text| format!("{text}{CALLED}"))
        }
        _ => None,
    }
}

pub fn prelude_proven(callee: &Expression<'_>, source_text: &str) -> bool {
    prelude_text(callee, source_text)
        .is_some_and(|text| PRELUDE_FORMS.contains(&form_digest(&text).as_str()))
}

fn identifier_name(text: &str) -> bool {
    let mut chars = text.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$');
    starts && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn head_matches(text: &str, (prefix_len, prefix, suffix): (usize, &str, &str)) -> bool {
    let (Some(head), Some(rest)) = (text.get(..prefix_len), text.get(prefix_len..)) else {
        return false;
    };
    if form_digest(head) != prefix {
        return false;
    }
    let name_len = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(rest.len());
    let (name, tail) = rest.split_at(name_len);
    identifier_name(name) && form_digest(tail) == suffix
}

pub fn head_text(head: &Function<'_>, source_text: &str) -> Option<String> {
    function_shape(head, source_text)
}

pub fn head_proven(head: &Function<'_>, source_text: &str) -> bool {
    head_text(head, source_text)
        .is_some_and(|text| HEAD_FORMS.iter().any(|form| head_matches(&text, *form)))
}

fn statement_expression<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let mut expression = statement.expression.without_parentheses();
    while let Expression::UnaryExpression(unary) = expression {
        if !matches!(
            unary.operator,
            UnaryOperator::LogicalNot | UnaryOperator::Void
        ) {
            return None;
        }
        expression = unary.argument.without_parentheses();
    }
    Some(expression)
}

fn only_statement<'k, 'a>(program: &'k Program<'a>) -> Option<&'k Statement<'a>> {
    let mut statements = program
        .body
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)));
    let only = statements.next()?;
    statements.next().is_none().then_some(only)
}

fn same<'a>(left: &Expression<'a>, right: &Expression<'a>) -> bool {
    std::ptr::eq(left.without_parentheses(), right.without_parentheses())
}

pub enum Frame<'k, 'a> {
    Plain,
    Standalone {
        head: &'k Function<'a>,
        entry: &'k Expression<'a>,
    },
}

fn shadow_only(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.declarations.iter().all(|declarator| {
        declarator.init.is_none() && matches!(declarator.id, BindingPattern::BindingIdentifier(_))
    })
}

enum Body<'k, 'a> {
    Block(&'k FunctionBody<'a>),
    Expression(&'k Expression<'a>),
}

fn bare(params: &FormalParameters<'_>) -> bool {
    params.items.is_empty() && params.rest.is_none()
}

fn factory_body<'k, 'a>(factory: &'k Expression<'a>) -> Option<Body<'k, 'a>> {
    match factory.without_parentheses() {
        Expression::FunctionExpression(function) => {
            let plain = !function.r#async && !function.generator && bare(&function.params);
            plain.then_some(Body::Block(function.body.as_deref()?))
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async && bare(&arrow.params) => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(Body::Block(body)),
                body => body.as_expression().map(Body::Expression),
            }
        }
        _ => None,
    }
}

fn block_result<'k, 'a>(body: &'k FunctionBody<'a>) -> Option<&'k Expression<'a>> {
    if !body.directives.is_empty() {
        return None;
    }
    let (last, shadows) = body.statements.split_last()?;
    if !shadows.iter().all(shadow_only) {
        return None;
    }
    match last {
        Statement::ReturnStatement(returned) => returned.argument.as_ref(),
        _ => None,
    }
}

fn standalone_body<'k, 'a>(
    factory: &'k Expression<'a>,
    bundle: &Expression<'a>,
) -> Option<&'k Expression<'a>> {
    let result = match factory_body(factory)? {
        Body::Block(body) => block_result(body)?,
        Body::Expression(expression) => expression,
    };
    let Expression::CallExpression(call) = result.without_parentheses() else {
        return None;
    };
    let [argument] = call.arguments.as_slice() else {
        return None;
    };
    let entry = argument.as_expression()?;
    (!call.optional && same(&call.callee, bundle)).then_some(entry)
}

pub fn frame<'k, 'a>(
    program: &'k Program<'a>,
    bundle: &Expression<'a>,
) -> Result<Frame<'k, 'a>, Unproven> {
    if !program.directives.is_empty() {
        return Err(Unproven::Placement);
    }
    let expression = only_statement(program)
        .and_then(statement_expression)
        .ok_or(Unproven::Placement)?;
    if same(expression, bundle) {
        return Ok(Frame::Plain);
    }
    let Expression::CallExpression(call) = expression else {
        return Err(Unproven::Placement);
    };
    let [factory] = call.arguments.as_slice() else {
        return Err(Unproven::Placement);
    };
    let entry = factory
        .as_expression()
        .and_then(|factory| standalone_body(factory, bundle))
        .ok_or(Unproven::Placement)?;
    let Expression::FunctionExpression(head) = call.callee.without_parentheses() else {
        return Err(Unproven::Head);
    };
    if call.optional {
        return Err(Unproven::Placement);
    }
    Ok(Frame::Standalone { head, entry })
}
