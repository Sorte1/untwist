pub mod build;
pub mod cluster;
pub mod notes;
pub mod scan;
pub mod shell;

use std::ops::Range;

use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::{ImportDeclaration, Program, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::SourceType;
use oxc_str::Str;
use rustc_hash::FxHashMap;
use untwist_core::pass::Note;
use untwist_core::semantic::build_scoping;

use crate::frame::Frame;
use crate::model::{body_key, Bodies, Bundle, BundleKind};
use crate::path::relative_path;
use build::{assemble, links, Links};
use scan::{scan, Context};
use shell::{shell, wrapper, Shell};

type Plan = (Vec<Range<usize>>, Links);

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    wrapper(frame.top).is_some_and(|found| found.wrapper.body.statements.len() >= 2)
}

fn planned(scoping: &Scoping, items: &[Statement<'_>], context: &Context) -> Option<Plan> {
    let facts = scan(scoping, items, context)?;
    let segments = cluster::plan(&facts)?;
    let links = links(scoping, &facts, &segments);
    Some((segments, links))
}

fn on_source(source: &Program<'_>, found: &Shell<'_, '_>) -> Option<Plan> {
    let semantics = build_scoping(source);
    let context = Context {
        scope: found.wrapper.scope.get()?,
        arrow: found.wrapper.arrow,
    };
    planned(&semantics.scoping, &found.wrapper.body.statements, &context)
}

fn detached<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    found: &Shell<'_, 'a>,
    owned: AVec<'a, Statement<'a>>,
) -> (Option<Plan>, AVec<'a, Statement<'a>>) {
    let builder = AstBuilder::new(alloc);
    let mut temporary = Program::new(
        found.wrapper.body.span,
        SourceType::mjs(),
        source.source_text,
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        owned,
        &builder,
    );
    let semantics = build_scoping(&temporary);
    let context = Context {
        scope: semantics.scoping.root_scope_id(),
        arrow: found.wrapper.arrow,
    };
    let plan = planned(&semantics.scoping, &temporary.body, &context);
    let owned = std::mem::replace(&mut temporary.body, AVec::new_in(&builder));
    (plan, owned)
}

fn give_back<'a>(bodies: &mut Bodies<'a>, found: &Shell<'_, 'a>, owned: AVec<'a, Statement<'a>>) {
    if let Bodies::Taken(pool) = bodies {
        pool.insert(body_key(found.wrapper.body), owned);
    }
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    _frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let found = shell(source)?;
    let (plan, owned) = match bodies {
        Bodies::Taken(_) => {
            let owned = bodies.statements(alloc, found.wrapper.body);
            detached(alloc, source, &found, owned)
        }
        Bodies::Planned(_) | Bodies::Cloned => {
            let plan = on_source(source, &found)?;
            let owned = bodies.statements(alloc, found.wrapper.body);
            if matches!(bodies, Bodies::Planned(_)) {
                return Some(Bundle::new(
                    BundleKind::Rollup,
                    Some(build::ENTRY_ID.to_string()),
                ));
            }
            (Some(plan), owned)
        }
    };
    let Some((segments, links)) = plan else {
        give_back(bodies, &found, owned);
        return None;
    };
    match assemble(alloc, source, &found, (&segments, &links), owned) {
        Ok(bundle) => Some(bundle),
        Err(owned) => {
            give_back(bodies, &found, owned);
            None
        }
    }
}

fn relink<'a>(
    builder: &AstBuilder<'a>,
    declaration: &mut ImportDeclaration<'a>,
    targets: &FxHashMap<String, String>,
) {
    let path = declaration
        .source
        .value
        .as_str()
        .and_then(|source| targets.get(source));
    if let Some(path) = path {
        declaration.source.value = Str::from_str_in(path, builder).into();
        declaration.source.raw = None;
    }
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    let builder = AstBuilder::new(alloc);
    let paths: FxHashMap<String, String> = bundle
        .modules
        .values()
        .map(|module| (module.id.clone(), module.path.as_str().to_string()))
        .collect();
    for module in bundle.modules.values_mut() {
        let targets: FxHashMap<String, String> = module
            .dependencies
            .iter()
            .filter_map(|(id, _)| {
                Some((
                    id.clone(),
                    relative_path(module.path.as_str(), paths.get(id)?),
                ))
            })
            .collect();
        for statement in module.program_mut().body.iter_mut() {
            if let Statement::ImportDeclaration(declaration) = statement {
                relink(&builder, declaration, &targets);
            }
        }
    }
    Vec::new()
}
