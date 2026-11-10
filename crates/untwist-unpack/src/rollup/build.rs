use std::ops::Range;

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::FxHashSet;

use super::scan::Facts;
use super::shell::Shell;
use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{js_string, synthetic_statements};

pub const ENTRY_ID: &str = "index";
pub const CHUNK_PREFIX: &str = "chunk_";
const REQUIRE_HOST: &str = "node:module";

pub struct Links {
    pub ids: Vec<String>,
    pub imports: Vec<Vec<(usize, Vec<String>)>>,
    pub exports: Vec<Vec<String>>,
    pub requires: Vec<bool>,
    pub dropped: Vec<bool>,
}

fn sanitized(name: &str) -> String {
    name.chars()
        .map(
            |ch| match ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
                true => ch,
                false => '_',
            },
        )
        .collect()
}

fn chunk_ids(scoping: &Scoping, facts: &Facts, segments: &[Range<usize>]) -> Vec<String> {
    let mut taken: FxHashSet<String> = FxHashSet::default();
    taken.insert(ENTRY_ID.to_string());
    let mut ids = Vec::with_capacity(segments.len());
    for segment in &segments[..segments.len() - 1] {
        let stem = facts.first_decl[segment.clone()]
            .iter()
            .flatten()
            .next()
            .map_or_else(
                || segment.start.to_string(),
                |&s| sanitized(scoping.symbol_name(s)),
            );
        let base = format!("{CHUNK_PREFIX}{stem}");
        let mut id = base.clone();
        let mut suffix = 2usize;
        while !taken.insert(id.to_ascii_lowercase()) {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        ids.push(id);
    }
    ids.push(ENTRY_ID.to_string());
    ids
}

fn segment_of(segments: &[Range<usize>], n: usize) -> Vec<usize> {
    let mut of = vec![0; n];
    for (index, segment) in segments.iter().enumerate() {
        of[segment.clone()].fill(index);
    }
    of
}

pub fn links(scoping: &Scoping, facts: &Facts, segments: &[Range<usize>]) -> Links {
    let count = segments.len();
    let of = segment_of(segments, facts.declares.len());
    let mut imports: Vec<Vec<Vec<String>>> = vec![vec![Vec::new(); count]; count];
    let mut exports: Vec<Vec<String>> = vec![Vec::new(); count];
    for &(item, symbol) in &facts.uses {
        let (to, from) = (of[item as usize], of[facts.owner[symbol.index()] as usize]);
        if from < to {
            let name = scoping.symbol_name(symbol).to_string();
            imports[to][from].push(name.clone());
            exports[from].push(name);
        }
    }
    let imports = imports
        .into_iter()
        .enumerate()
        .map(|(to, lists)| {
            lists
                .into_iter()
                .enumerate()
                .filter(|(from, names)| !names.is_empty() || (to + 1 == count && *from < to))
                .map(|(from, mut names)| {
                    names.sort_unstable();
                    names.dedup();
                    (from, names)
                })
                .collect()
        })
        .collect();
    for names in &mut exports {
        names.sort_unstable();
        names.dedup();
    }
    let requires = segments
        .iter()
        .map(|segment| facts.requires[segment.clone()].contains(&true))
        .collect();
    Links {
        ids: chunk_ids(scoping, facts, segments),
        imports,
        exports,
        requires,
        dropped: facts.dropped.clone(),
    }
}

fn fresh_creator(text: &str) -> String {
    let mut name = "createRequire".to_string();
    while text.contains(name.as_str()) {
        name.push('_');
    }
    name
}

fn head_text(links: &Links, at: usize, creator: &str) -> String {
    let mut text = String::new();
    for (from, names) in &links.imports[at] {
        let id = js_string(&links.ids[*from]);
        match names.is_empty() {
            true => text.push_str(&format!("import {id};\n")),
            false => text.push_str(&format!("import {{ {} }} from {id};\n", names.join(", "))),
        }
    }
    if links.requires[at] {
        let host = js_string(REQUIRE_HOST);
        text.push_str(&format!(
            "import {{ createRequire as {creator} }} from {host};\nconst require = {creator}(import.meta.url);\n"
        ));
    }
    text
}

fn tail_text(links: &Links, at: usize) -> String {
    match links.exports[at].is_empty() {
        true => String::new(),
        false => format!("export {{ {} }};", links.exports[at].join(", ")),
    }
}

fn parsed<'a>(alloc: &'a Allocator, text: String) -> Option<AVec<'a, Statement<'a>>> {
    match text.is_empty() {
        true => Some(AVec::new_in(&alloc)),
        false => synthetic_statements(alloc, text),
    }
}

fn wrapper_body<'e, 'a>(
    statements: &'e mut AVec<'a, Statement<'a>>,
) -> Option<&'e mut FunctionBody<'a>> {
    let Some(Statement::ExpressionStatement(statement)) = statements.first_mut() else {
        return None;
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        return None;
    };
    match &mut call.callee {
        Expression::FunctionExpression(function) => function.body.as_deref_mut(),
        Expression::ArrowFunctionExpression(arrow) => match &mut arrow.body {
            ArrowFunctionBody::FunctionBody(body) => Some(&mut **body),
            _ => None,
        },
        _ => None,
    }
}

struct Parts<'a> {
    heads: Vec<AVec<'a, Statement<'a>>>,
    tails: Vec<AVec<'a, Statement<'a>>>,
    wrapper: AVec<'a, Statement<'a>>,
}

fn parts<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    links: &Links,
) -> Option<Parts<'a>> {
    let creator = fresh_creator(source.source_text);
    let count = links.ids.len();
    let mut heads = Vec::with_capacity(count);
    let mut tails = Vec::with_capacity(count);
    for at in 0..count {
        heads.push(parsed(alloc, head_text(links, at, &creator))?);
        tails.push(parsed(alloc, tail_text(links, at))?);
    }
    let text = match shell.wrapper.arrow {
        true => "(() => {})();",
        false => "(function () {})();",
    };
    let mut wrapper = synthetic_statements(alloc, text.to_string())?;
    let body = wrapper_body(&mut wrapper)?;
    body.directives = shell.wrapper.body.directives.clone_in(alloc);
    Some(Parts {
        heads,
        tails,
        wrapper,
    })
}

fn comments_of(source: &Program<'_>, items: &[Statement<'_>]) -> Vec<Comment> {
    let mut comments = Vec::new();
    for item in items {
        let span = item.span();
        let from = source
            .comments
            .partition_point(|c| c.span.start < span.start);
        comments.extend(
            source.comments[from..]
                .iter()
                .take_while(|c| c.span.end <= span.end)
                .cloned(),
        );
    }
    comments
}

fn program<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    comments: Vec<Comment>,
    directives: AVec<'a, Directive<'a>>,
    statements: AVec<'a, Statement<'a>>,
) -> Program<'a> {
    let builder = AstBuilder::new(alloc);
    Program::new(
        Span::default(),
        SourceType::mjs(),
        source.source_text,
        AVec::from_iter_in(comments, &builder),
        None,
        directives,
        statements,
        &builder,
    )
}

fn entry_statements<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    head: AVec<'a, Statement<'a>>,
    mut wrapper: AVec<'a, Statement<'a>>,
) -> AVec<'a, Statement<'a>> {
    let mut statements = head;
    for (index, statement) in source.body.iter().enumerate() {
        if index == shell.wrapper.index {
            statements.extend(wrapper.drain(..));
        } else {
            statements.push(statement.clone_in(alloc));
        }
    }
    statements
}

pub fn assemble<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    shell: &Shell<'_, 'a>,
    (segments, links): (&[Range<usize>], &Links),
    owned: AVec<'a, Statement<'a>>,
) -> Result<Bundle<'a>, AVec<'a, Statement<'a>>> {
    let Some(parts) = parts(alloc, source, shell, links) else {
        return Err(owned);
    };
    let Parts {
        heads,
        tails,
        mut wrapper,
    } = parts;
    let mut items = owned.into_iter().zip(&links.dropped);
    let mut bundle = Bundle::new(BundleKind::Rollup, Some(ENTRY_ID.to_string()));
    let last = segments.len() - 1;
    for (at, (head, tail)) in heads.into_iter().zip(tails).enumerate() {
        let body: Vec<Statement<'a>> = items
            .by_ref()
            .take(segments[at].len())
            .filter(|(_, dropped)| !**dropped)
            .map(|(statement, _)| statement)
            .collect();
        let comments = comments_of(source, &body);
        let id = links.ids[at].clone();
        let statements = if at == last {
            let mut wrapper = std::mem::replace(&mut wrapper, AVec::new_in(&alloc));
            if let Some(inner) = wrapper_body(&mut wrapper) {
                inner.statements.extend(body);
            }
            entry_statements(alloc, source, shell, head, wrapper)
        } else {
            let mut statements = head;
            statements.extend(body);
            statements.extend(tail);
            statements
        };
        let directives = match at == last {
            true => source.directives.clone_in(alloc),
            false => AVec::new_in(&alloc),
        };
        let program = program(alloc, source, comments, directives, statements);
        let mut module = Module::new(id.clone(), program, at == last);
        module.dependencies = links.imports[at]
            .iter()
            .map(|(from, _)| (links.ids[*from].clone(), links.ids[*from].clone()))
            .collect();
        bundle.modules.insert(id, module);
    }
    Ok(bundle)
}
