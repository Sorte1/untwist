use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitJs;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::FxHashSet;

use crate::closure::roles::{Role, ENTRY_ID};
use crate::closure::shell::Shell;
use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{declared_names, module_of, synthetic_statements};

pub const NAMESPACE_EXPORT: &str = "namespace";
pub const VARS_EXPORT: &str = "vars";

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
    let Statement::ExpressionStatement(statement) = statements.get_mut(0)? else {
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

struct Mentions<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Mentions<'a> {
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        self.names.insert(it.name.as_str());
    }
}

pub fn mentioned<'a>(statements: &[Statement<'a>]) -> FxHashSet<&'a str> {
    let mut mentions = Mentions {
        names: FxHashSet::default(),
    };
    for statement in statements {
        mentions.visit_statement(statement);
    }
    mentions.names
}

struct Prelude<'a> {
    statements: Vec<(Vec<String>, Statement<'a>)>,
}

impl<'a> Prelude<'a> {
    fn for_segment(&self, alloc: &'a Allocator, segment: &[Statement<'a>]) -> Vec<Statement<'a>> {
        let names = mentioned(segment);
        self.statements
            .iter()
            .filter(|(declared, _)| declared.iter().any(|name| names.contains(name.as_str())))
            .map(|(_, statement)| statement.clone_in(alloc))
            .collect()
    }
}

fn module_program<'a>(
    alloc: &'a Allocator,
    (source, shell, namespace): (&Program<'a>, &Shell<'_, 'a>, &str),
    inner: Vec<Statement<'a>>,
    spans: &[Span],
) -> Option<Program<'a>> {
    let p = shell.param.name.as_str();
    let text = format!("(function ({p}) {{}}).call(globalThis, {namespace});");
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

fn runtime_module<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
) -> Option<Module<'a>> {
    let x = shell.namespace;
    let text = format!(
        "globalThis.{x} = globalThis.{x} || {{}};\nmodule.exports = {{ {NAMESPACE_EXPORT}: globalThis.{x}, {VARS_EXPORT}: Object.create(null) }};"
    );
    let statements = synthetic_statements(alloc, text)?;
    module_of(alloc, source.source_text, BundleKind::Closure, statements)
}

pub fn assemble<'a>(
    alloc: &'a Allocator,
    (source, shell, namespace): (&Program<'a>, &Shell<'_, 'a>, &str),
    plan: &[Role],
    owned: AVec<'a, Statement<'a>>,
) -> Option<Bundle<'a>> {
    let mut statements = owned.into_iter();
    let mut prelude = Prelude {
        statements: Vec::new(),
    };
    let mut entry = Vec::new();
    let mut entry_spans = Vec::new();
    let mut modules = Vec::new();
    for role in plan {
        let statement = statements.next();
        let span = statement.as_ref().map(GetSpan::span);
        match (role, statement) {
            (Role::Prelude, Some(statement)) => {
                let declared = declared_names(std::slice::from_ref(&statement));
                prelude.statements.push((declared, statement));
            }
            (Role::Setup, Some(statement)) => {
                entry_spans.extend(span);
                entry.push(statement);
            }
            (Role::Module(id), statement) => modules.push((id.clone(), statement)),
            _ => {}
        }
    }
    let context = (source, shell, namespace);
    let mut bundle = Bundle::new(BundleKind::Closure, Some(ENTRY_ID.to_string()));
    for (id, statement) in modules {
        let spans: Vec<Span> = statement.iter().map(GetSpan::span).collect();
        let segment: Vec<Statement<'a>> = statement.into_iter().collect();
        let mut inner = prelude.for_segment(alloc, &segment);
        inner.extend(segment);
        let program = module_program(alloc, context, inner, &spans)?;
        bundle
            .modules
            .insert(id.clone(), Module::new(id, program, false));
    }
    let mut inner = prelude.for_segment(alloc, &entry);
    inner.extend(entry);
    let program = module_program(alloc, context, inner, &entry_spans)?;
    bundle.modules.insert(
        ENTRY_ID.to_string(),
        Module::new(ENTRY_ID.to_string(), program, true),
    );
    let module = runtime_module(alloc, source, shell)?;
    bundle.modules.insert(module.id.clone(), module);
    Some(bundle)
}
