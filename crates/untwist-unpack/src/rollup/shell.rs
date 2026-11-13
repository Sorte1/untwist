use oxc_ast::ast::*;
use oxc_syntax::operator::UnaryOperator;
use std::cell::Cell;

use oxc_syntax::scope::ScopeId;

pub struct Wrapper<'k, 'a> {
    pub index: usize,
    pub body: &'k FunctionBody<'a>,
    pub arrow: bool,
    pub scope: &'k Cell<Option<ScopeId>>,
}

pub struct Shell<'k, 'a> {
    pub wrapper: Wrapper<'k, 'a>,
    pub outer: Option<Wrapper<'k, 'a>>,
}

const SCRIPT_ONLY: &str = "currentScript";

fn discarded<'k, 'a>(expr: &'k Expression<'a>) -> &'k Expression<'a> {
    match expr {
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::LogicalNot | UnaryOperator::Void
            ) =>
        {
            discarded(&unary.argument)
        }
        other => other,
    }
}

fn called<'k, 'a>(
    callee: &'k Expression<'a>,
) -> Option<(&'k FunctionBody<'a>, bool, &'k Cell<Option<ScopeId>>)> {
    match callee {
        Expression::FunctionExpression(function) => {
            let plain = function.id.is_none() && !function.r#async && !function.generator;
            let bare = function.params.items.is_empty() && function.params.rest.is_none();
            (plain && bare).then_some(())?;
            Some((function.body.as_deref()?, false, &function.scope_id))
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let bare = arrow.params.items.is_empty() && arrow.params.rest.is_none();
            (bare && !arrow.r#async).then_some(())?;
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return None;
            };
            Some((body, true, &arrow.scope_id))
        }
        _ => None,
    }
}

fn wrapper_of<'k, 'a>(index: usize, statement: &'k Statement<'a>) -> Option<Wrapper<'k, 'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = discarded(&statement.expression) else {
        return None;
    };
    if call.optional || !call.arguments.is_empty() {
        return None;
    }
    let (body, arrow, scope) = called(&call.callee)?;
    Some(Wrapper {
        index,
        body,
        arrow,
        scope,
    })
}

pub fn wrapper<'k, 'a>(top: &'k [Statement<'a>]) -> Option<Shell<'k, 'a>> {
    let mut found = None;
    for (index, statement) in top.iter().enumerate() {
        if matches!(statement, Statement::EmptyStatement(_)) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(wrapper_of(index, statement)?);
    }
    Some(Shell {
        wrapper: found?,
        outer: None,
    })
}

fn reads_current_script(program: &Program<'_>, body: &FunctionBody<'_>) -> bool {
    let span = body.span;
    program
        .source_text
        .get(span.start as usize..span.end as usize)
        .is_none_or(|text| text.contains(SCRIPT_ONLY))
}

fn module_item(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ImportDeclaration(_)
            | Statement::ExportDeclaration(_)
            | Statement::ExportNamedDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
    )
}

fn reexports(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ExportAllDeclaration(_) | Statement::ExportFromDeclaration(_)
    )
}

pub fn module_list(top: &[Statement<'_>]) -> bool {
    let imports = top
        .iter()
        .take_while(|statement| {
            matches!(
                statement,
                Statement::ImportDeclaration(_) | Statement::EmptyStatement(_)
            )
        })
        .count();
    let imports_lead = top[imports..]
        .iter()
        .all(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    top.len() >= 2 && top.iter().any(module_item) && !top.iter().any(reexports) && imports_lead
}

pub fn imports_end(top: &[Statement<'_>]) -> usize {
    top.iter()
        .rposition(|statement| matches!(statement, Statement::ImportDeclaration(_)))
        .map_or(0, |at| at + 1)
}

pub fn top<'k, 'a>(program: &'k Program<'a>) -> Option<&'k [Statement<'a>]> {
    let module = program.source_type.is_module() && module_list(&program.body);
    let span = program.span;
    let current = program
        .source_text
        .get(span.start as usize..span.end as usize)
        .is_none_or(|text| text.contains(SCRIPT_ONLY));
    (module && !current).then_some(&program.body[..])
}

pub fn shell<'k, 'a>(program: &'k Program<'a>) -> Option<Shell<'k, 'a>> {
    let found = wrapper(&program.body)?;
    let strict =
        found.wrapper.body.has_use_strict_directive() || program.has_use_strict_directive();
    (strict && !reads_current_script(program, found.wrapper.body)).then_some(found)
}

fn strict_body(program: &Program<'_>, body: &FunctionBody<'_>) -> bool {
    body.has_use_strict_directive() || program.has_use_strict_directive()
}

fn nested<'k, 'a>(program: &Program<'a>, outer: &Wrapper<'k, 'a>) -> Option<Wrapper<'k, 'a>> {
    let mut found = None;
    for (index, statement) in outer.body.statements.iter().enumerate() {
        let Some(inner) = wrapper_of(index, statement) else {
            continue;
        };
        if found.is_some() {
            return None;
        }
        found = Some(inner);
    }
    let inner = found?;
    let strict = strict_body(program, outer.body) || inner.body.has_use_strict_directive();
    strict.then_some(inner)
}

pub fn assumed<'k, 'a>(program: &'k Program<'a>) -> Vec<Shell<'k, 'a>> {
    let Some(found) = wrapper(&program.body) else {
        return Vec::new();
    };
    let mut shells = Vec::new();
    if let Some(inner) = nested(program, &found.wrapper) {
        let outer = wrapper_of(found.wrapper.index, &program.body[found.wrapper.index]);
        shells.push(Shell {
            wrapper: inner,
            outer,
        });
    }
    if strict_body(program, found.wrapper.body) {
        shells.push(found);
    }
    shells
}

pub fn loose_wrapper(top: &[Statement<'_>]) -> bool {
    let mut statements = top
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)));
    let (Some(Statement::ExpressionStatement(statement)), None) =
        (statements.next(), statements.next())
    else {
        return false;
    };
    let Expression::CallExpression(call) =
        discarded(statement.expression.without_parentheses()).without_parentheses()
    else {
        return false;
    };
    !call.optional
        && call.arguments.is_empty()
        && called(call.callee.without_parentheses()).is_some()
}
