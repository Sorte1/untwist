use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast::{Comment, CommentKind};
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::parse::parse_program;

use crate::esbuild::layout::{FactoryDecl, Host, Layout, Scope};
use crate::esbuild::mentions::{free_mentions, mentioned};
use crate::matcher::param_names;
use crate::model::{module_from_body_with, Bodies, Bundle, Module, Runtime};
use crate::path::PosixPath;
use crate::rename::{rename_parameters_uncaptured, rename_wrapper_bindings};
use crate::runtime::{declared_names, live_runtime_module, runtime_id};

pub const ENTRY_ID: &str = "index";
const MODULE_PARAMS: [&str; 2] = ["exports", "module"];
const USE_STRICT: &str = "use strict";

pub struct Plan<'k, 'a> {
    pub scope: Scope<'k, 'a>,
    pub layout: Layout<'k, 'a>,
    pub strict: bool,
    pub module: bool,
}

pub fn has_use_strict(directives: &[Directive<'_>]) -> bool {
    directives
        .iter()
        .any(|directive| directive.directive.as_str() == USE_STRICT)
}

fn use_strict<'a>(alloc: &'a Allocator) -> Option<Directive<'a>> {
    let parsed = parse_program(alloc, "\"use strict\";");
    parsed.program.directives.into_iter().next()
}

fn strict_directives<'a>(
    alloc: &'a Allocator,
    directives: &[Directive<'a>],
    strict: bool,
) -> AVec<'a, Directive<'a>> {
    let mut out = AVec::from_iter_in(directives.iter().map(|d| d.clone_in(alloc)), &alloc);
    if strict && !has_use_strict(directives) {
        out.extend(use_strict(alloc));
    }
    out
}

pub fn candidates(list: &[Statement<'_>], layout: &Layout<'_, '_>) -> FxHashMap<String, usize> {
    let mut names = FxHashMap::default();
    for &index in &layout.prefix {
        for name in declared_names(std::slice::from_ref(&list[index])) {
            names.insert(name, index);
        }
    }
    names
}

pub fn wanted_helpers(
    candidates: &FxHashMap<String, usize>,
    shim: Option<&str>,
) -> FxHashMap<String, usize> {
    let mut wanted = candidates.clone();
    if let Some(shim) = shim {
        wanted.remove(shim);
    }
    wanted
}

pub struct Scan {
    pub seeds: FxHashSet<usize>,
    pub dependencies: Vec<Vec<usize>>,
}

pub fn scan(
    plan: &Plan<'_, '_>,
    helpers: &FxHashMap<String, usize>,
    programs: &[Program<'_>],
) -> Scan {
    let offset = plan.scope.list.len();
    let mut wanted = helpers.clone();
    wanted.extend(
        factory_names(&plan.layout)
            .into_iter()
            .map(|(name, index)| (name, offset + index)),
    );
    let mut seeds = FxHashSet::default();
    let dependencies = programs
        .iter()
        .zip(plan.layout.all())
        .map(|(program, factory)| {
            let mut found: Vec<usize> = Vec::new();
            let params = param_names(factory.function.params);
            for index in free_mentions(&wanted, program, &params) {
                match index.checked_sub(offset) {
                    Some(factory) => found.push(factory),
                    None => {
                        seeds.insert(index);
                    }
                }
            }
            found.sort_unstable();
            found
        })
        .collect();
    Scan {
        seeds,
        dependencies,
    }
}

fn factory_names(layout: &Layout<'_, '_>) -> FxHashMap<String, usize> {
    layout
        .all()
        .enumerate()
        .map(|(index, factory)| (factory.name.to_string(), index))
        .collect()
}

fn line_comments(source: &Program<'_>) -> FxHashMap<u32, Comment> {
    source
        .comments
        .iter()
        .filter(|comment| comment.kind == CommentKind::Line)
        .map(|comment| (comment.attached_to, comment.clone()))
        .collect()
}

fn comment_path(
    source: &Program<'_>,
    (list, attached): (&[Statement<'_>], &FxHashMap<u32, Comment>),
    factory: &FactoryDecl,
) -> Option<String> {
    let statement = list.get(factory.statement)?;
    if let Statement::VariableDeclaration(declaration) = statement {
        if declaration.declarations.len() != 1 {
            return None;
        }
    }
    let comment = attached.get(&statement.span().start)?;
    let text = comment.span.source_text(source.source_text);
    let path = crate::rolldown::shape::region_path(text.strip_prefix("//")?.trim());
    let plain = !path.is_empty() && !path.chars().any(char::is_whitespace);
    (plain && PosixPath::parse(path).is_some()).then(|| path.to_string())
}

fn unique_id(taken: &mut FxHashSet<String>, preferred: Option<String>, name: &str) -> String {
    if let Some(id) = preferred.filter(|id| !taken.contains(&id.to_ascii_lowercase())) {
        taken.insert(id.to_ascii_lowercase());
        return id;
    }
    let mut id = name.to_string();
    let mut suffix = 2;
    while !taken.insert(id.to_ascii_lowercase()) {
        id = format!("{name}-{suffix}");
        suffix += 1;
    }
    id
}

fn factory_ids(
    source: &Program<'_>,
    list: &[Statement<'_>],
    layout: &Layout<'_, '_>,
    has_runtime: bool,
) -> Vec<String> {
    let mut taken: FxHashSet<String> = FxHashSet::default();
    taken.insert(ENTRY_ID.to_string());
    if has_runtime {
        taken.insert(runtime_id(layout.kind));
    }
    let keyless = layout.all().any(|factory| factory.key.is_none());
    let attached = if keyless {
        line_comments(source)
    } else {
        FxHashMap::default()
    };
    layout
        .all()
        .map(|factory| {
            let preferred = factory.key.clone().or_else(|| {
                factory
                    .inline
                    .is_none()
                    .then(|| comment_path(source, (list, &attached), factory))
                    .flatten()
            });
            unique_id(&mut taken, preferred, &factory.name)
        })
        .collect()
}

pub fn factory_programs<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    plan: &Plan<'_, 'a>,
    bodies: &mut Bodies<'a>,
) -> Vec<Program<'a>> {
    plan.layout
        .all()
        .map(|factory| {
            let mut program = module_from_body_with(alloc, source, factory.function.body, bodies);
            program.directives =
                strict_directives(alloc, &factory.function.body.directives, plan.strict);
            program
        })
        .collect()
}

fn named(found: &[usize], names: &[&str], ids: &[String]) -> Vec<(String, String)> {
    found
        .iter()
        .map(|&index| (ids[index].clone(), names[index].to_string()))
        .collect()
}

fn kept_statement<'a>(
    alloc: &'a Allocator,
    statement: &Statement<'a>,
    factories: &[FactoryDecl<'_, '_>],
) -> Option<Statement<'a>> {
    if factories.is_empty() {
        return Some(statement.clone_in(alloc));
    }
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    if declaration.declarations.len() == factories.len() {
        return None;
    }
    let mut kept = declaration.clone_in(alloc);
    let mut index = 0;
    kept.declarations.retain(|_| {
        let keep = factories
            .binary_search_by_key(&index, |factory| factory.declarator)
            .is_err();
        index += 1;
        keep
    });
    Some(Statement::VariableDeclaration(kept))
}

fn removed_spans(plan: &Plan<'_, '_>, moved: &[usize]) -> Vec<Span> {
    let list = plan.scope.list;
    let mut spans: Vec<Span> = moved.iter().map(|&index| list[index].span()).collect();
    spans.extend(plan.layout.maps().map(|map| list[map.statement].span()));
    for factory in &plan.layout.factories {
        let statement = &list[factory.statement];
        let span = match statement {
            Statement::VariableDeclaration(declaration) if declaration.declarations.len() > 1 => {
                declaration.declarations[factory.declarator].span
            }
            other => other.span(),
        };
        spans.push(span);
    }
    for factory in &plan.layout.inline {
        let function = factory.function;
        spans.push(Span::new(
            function.params.span.start,
            function.body.span.end,
        ));
    }
    spans.sort_unstable_by_key(|span| span.start);
    spans
}

fn covered(sorted: &[Span], inner: Span) -> bool {
    let after = sorted.partition_point(|span| span.start <= inner.start);
    after > 0 && inner.end <= sorted[after - 1].end
}

fn entry_comments<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    plan: &Plan<'_, 'a>,
    moved: &[usize],
) -> AVec<'a, Comment> {
    let removed = removed_spans(plan, moved);
    let starts: FxHashSet<u32> = removed.iter().map(|span| span.start).collect();
    let region = plan
        .scope
        .function
        .map_or(source.span, |function| function.body.span);
    let kept = source.comments.iter().filter(|comment| {
        let inside = region.start <= comment.span.start && comment.span.end <= region.end;
        inside && !starts.contains(&comment.attached_to) && !covered(&removed, comment.span)
    });
    AVec::from_iter_in(kept.cloned(), &alloc)
}

fn entry_statements<'a>(
    alloc: &'a Allocator,
    plan: &Plan<'_, 'a>,
    moved: &[usize],
) -> AVec<'a, Statement<'a>> {
    let list = plan.scope.list;
    let mut out = AVec::with_capacity_in(list.len(), &alloc);
    for (index, statement) in list.iter().enumerate() {
        if moved.binary_search(&index).is_err() && !plan.layout.is_map_statement(index) {
            let kept = kept_statement(alloc, statement, plan.layout.factories_at(index));
            out.extend(kept.map(|mut kept| {
                let sites: Vec<(Span, &str)> = plan
                    .layout
                    .inline
                    .iter()
                    .filter(|factory| factory.statement == index)
                    .filter_map(|factory| Some((factory.inline?, factory.name.as_str())))
                    .collect();
                crate::rolldown::inline::replace_inline(alloc, &mut kept, &sites);
                kept
            }));
        }
    }
    out
}

fn wrapper_params<'k>(plan: &Plan<'k, '_>) -> Vec<&'k str> {
    match plan.scope.function.filter(|_| plan.scope.host == Host::Bun) {
        Some(wrapper) => param_names(wrapper.params),
        None => Vec::new(),
    }
}

fn entry_program<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    plan: &Plan<'_, 'a>,
    moved: &[usize],
) -> Program<'a> {
    let builder = AstBuilder::new(alloc);
    let body = entry_statements(alloc, plan, moved);
    let is_module = body.iter().any(Statement::is_module_declaration);
    let mut directives = strict_directives(alloc, plan.scope.directives, false);
    if plan.scope.host != Host::Top && !is_module {
        directives = strict_directives(alloc, &directives, plan.strict);
    }
    let source_type = if is_module {
        SourceType::mjs()
    } else {
        SourceType::cjs()
    };
    Program::new(
        source.span,
        source_type,
        source.source_text,
        entry_comments(alloc, source, plan, moved),
        source.hashbang.clone_in(alloc),
        directives,
        body,
        &builder,
    )
}

fn runtime_of<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    plan: &Plan<'_, 'a>,
    moved: &[usize],
) -> Option<Module<'a>> {
    let mut statements = AVec::with_capacity_in(moved.len(), &alloc);
    for &index in moved {
        statements.push(plan.scope.list[index].clone_in(alloc));
    }
    let mut module = live_runtime_module(alloc, source, plan.layout.kind, statements)?;
    let program = module.program_mut();
    program.directives = strict_directives(alloc, &program.directives, plan.strict);
    Some(module)
}

pub fn bundle<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    plan: &Plan<'_, 'a>,
    (programs, dependencies): (Vec<Program<'a>>, Vec<Vec<usize>>),
    moved: &[usize],
) -> Option<Bundle<'a>> {
    let list = plan.scope.list;
    let ids = factory_ids(source, list, &plan.layout, !moved.is_empty());
    let names = factory_names(&plan.layout);
    let mut bundle = Bundle::new(plan.layout.kind, Some(ENTRY_ID.to_string()));
    let mut entry = entry_program(alloc, source, plan, moved);
    rename_wrapper_bindings(alloc, &mut entry, &wrapper_params(plan));
    let mut entry = Module::new(ENTRY_ID.to_string(), entry, true);
    let mut found: Vec<usize> = mentioned(&names, entry.program().body.iter())
        .into_iter()
        .collect();
    found.sort_unstable();
    let listed: Vec<&str> = plan
        .layout
        .all()
        .map(|factory| factory.name.as_str())
        .collect();
    entry.dependencies = named(&found, &listed, &ids);
    bundle.modules.insert(ENTRY_ID.to_string(), entry);
    if !moved.is_empty() {
        let runtime = runtime_of(alloc, source, plan, moved)?;
        bundle.modules.insert(runtime.id.clone(), runtime);
    }
    let factories = plan.layout.all().zip(programs).zip(&ids);
    for (((factory, mut program), id), found) in factories.zip(&dependencies) {
        let params = param_names(factory.function.params);
        if params
            .iter()
            .zip(MODULE_PARAMS)
            .any(|(from, to)| *from != to)
        {
            rename_parameters_uncaptured(alloc, &mut program, &params, &MODULE_PARAMS);
        }
        rename_wrapper_bindings(alloc, &mut program, &params);
        let mut module = Module::new(id.clone(), program, false);
        module.dependencies = named(found, &listed, &ids);
        bundle.modules.insert(id.clone(), module);
    }
    bundle.runtime = plan.layout.shim.map(|shim| Runtime {
        require_name: shim.name.to_string(),
        helpers: Vec::new(),
    });
    Some(bundle)
}
