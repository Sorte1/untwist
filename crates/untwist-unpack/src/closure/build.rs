use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, SourceType, Span};

use crate::closure::roles::{Role, ENTRY_ID};
use crate::closure::shell::Shell;
use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{module_of, synthetic_statements};

const HOLDER_MODULE: &str = "module.exports = Object.create(null);";

fn function_of<'e, 'a>(expr: &'e mut Expression<'a>) -> Option<&'e mut Function<'a>> {
    match expr {
        Expression::ParenthesizedExpression(inner) => function_of(&mut inner.expression),
        Expression::FunctionExpression(function) => Some(function),
        _ => None,
    }
}

fn wrapper_body<'e, 'a>(
    statements: &'e mut AVec<'a, Statement<'a>>,
) -> Option<&'e mut FunctionBody<'a>> {
    let Statement::ExpressionStatement(statement) = statements.get_mut(1)? else {
        return None;
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        return None;
    };
    let Expression::StaticMemberExpression(callee) = &mut call.callee else {
        return None;
    };
    function_of(&mut callee.object)?.body.as_deref_mut()
}

fn comments_of<'a>(source: &Program<'a>, spans: &[Span]) -> Vec<Comment> {
    let mut comments = Vec::new();
    for span in spans {
        let from = source
            .comments
            .partition_point(|comment| comment.span.start < span.start);
        comments.extend(
            source.comments[from..]
                .iter()
                .take_while(|comment| comment.span.end <= span.end)
                .cloned(),
        );
    }
    comments
}

fn module_program<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    inner: Vec<Statement<'a>>,
    spans: &[Span],
) -> Option<Program<'a>> {
    let (x, p) = (shell.namespace, shell.param.name.as_str());
    let text = format!(
        "globalThis.{x} = globalThis.{x} || {{}};\n(function ({p}) {{}}).call(globalThis, globalThis.{x});"
    );
    let mut statements = synthetic_statements(alloc, text)?;
    let body = wrapper_body(&mut statements)?;
    body.directives = shell.body.directives.clone_in(alloc);
    body.statements.extend(inner);
    let builder = AstBuilder::new(alloc);
    let comments = AVec::from_iter_in(comments_of(source, spans), &builder);
    Some(Program::new(
        Span::default(),
        SourceType::cjs(),
        source.source_text,
        comments,
        None,
        source.directives.clone_in(alloc),
        statements,
        &builder,
    ))
}

pub fn assemble<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    (plan, held): (&[Role], bool),
    owned: AVec<'a, Statement<'a>>,
) -> Option<Bundle<'a>> {
    let mut statements = owned.into_iter();
    let mut prelude = Vec::new();
    let mut entry = Vec::new();
    let mut entry_spans = Vec::new();
    let mut modules = Vec::new();
    for role in plan {
        let statement = statements.next();
        let span = statement.as_ref().map(GetSpan::span);
        match (role, statement) {
            (Role::Prelude, Some(statement)) => prelude.push(statement),
            (Role::Setup, Some(statement)) => {
                entry_spans.extend(span);
                entry.push(statement);
            }
            (Role::Module(id), statement) => modules.push((id.clone(), statement)),
            _ => {}
        }
    }
    let mut bundle = Bundle::new(BundleKind::Closure, Some(ENTRY_ID.to_string()));
    for (id, statement) in modules {
        let spans: Vec<Span> = statement.iter().map(GetSpan::span).collect();
        let mut inner: Vec<Statement<'a>> = prelude.iter().map(|s| s.clone_in(alloc)).collect();
        inner.extend(statement);
        let program = module_program(alloc, source, shell, inner, &spans)?;
        bundle
            .modules
            .insert(id.clone(), Module::new(id, program, false));
    }
    let mut inner = prelude;
    inner.extend(entry);
    let program = module_program(alloc, source, shell, inner, &entry_spans)?;
    bundle.modules.insert(
        ENTRY_ID.to_string(),
        Module::new(ENTRY_ID.to_string(), program, true),
    );
    if held {
        let statements = synthetic_statements(alloc, HOLDER_MODULE.to_string())?;
        let module = module_of(alloc, source.source_text, BundleKind::Closure, statements)?;
        bundle.modules.insert(module.id.clone(), module);
    }
    Some(bundle)
}
