pub mod assume;
pub mod build;
pub mod cluster;
pub mod notes;
pub mod scan;
pub mod shell;
pub mod wrappers;

use std::ops::Range;

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
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
use crate::specifier::ModulePaths;
use build::{assemble, links, Links};
use scan::{scan, Context};
use shell::{shell, wrapper, Shell};

type Plan = (Vec<Range<usize>>, Links);

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    wrapper(frame.top).is_some_and(|found| found.wrapper.body.statements.len() >= 2)
        || shell::module_list(frame.top)
}

pub fn assumed_probe(frame: &Frame<'_, '_>) -> bool {
    probe(frame) || shell::loose_wrapper(frame.top)
}

fn planned(scoping: &Scoping, items: &[Statement<'_>], context: &Context) -> Option<Plan> {
    let facts = scan(scoping, items, context)?;
    let groups = wrappers::groups(scoping, items, &facts);
    let segments = cluster::plan(&facts, &groups)?;
    let links = links(scoping, &facts, &segments, &groups);
    Some((segments, links))
}

fn on_source(source: &Program<'_>, found: &Shell<'_, '_>) -> Option<Plan> {
    let semantics = build_scoping(source);
    let context = Context {
        scope: found.wrapper.scope.get()?,
        arrow: found.wrapper.arrow,
        module: false,
        enclosing: None,
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
        module: false,
        enclosing: None,
    };
    let plan = planned(&semantics.scoping, &temporary.body, &context);
    let owned = std::mem::replace(&mut temporary.body, AVec::new_in(&builder));
    (plan, owned)
}

pub(super) fn give_back<'a>(
    bodies: &mut Bodies<'a>,
    found: &Shell<'_, 'a>,
    owned: AVec<'a, Statement<'a>>,
) {
    if let Bodies::Taken(pool) = bodies {
        pool.insert(body_key(found.wrapper.body), owned);
    }
}

fn top_plan(source: &Program<'_>, list: &[Statement<'_>]) -> Option<Plan> {
    let semantics = build_scoping(source);
    let context = Context {
        scope: semantics.scoping.root_scope_id(),
        arrow: false,
        module: true,
        enclosing: None,
    };
    let plan = planned(&semantics.scoping, list, &context)?;
    let imports = shell::imports_end(list);
    plan.0
        .first()
        .is_some_and(|first| first.end >= imports)
        .then_some(plan)
}

fn detect_top<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    list: &'k [Statement<'a>],
    bodies: &Bodies<'a>,
) -> Option<Bundle<'a>> {
    let (segments, links) = top_plan(source, list)?;
    if matches!(bodies, Bodies::Planned(_)) {
        return Some(Bundle::new(
            BundleKind::Rollup,
            Some(build::ENTRY_ID.to_string()),
        ));
    }
    let owned = AVec::from_iter_in(list.iter().map(|item| item.clone_in(alloc)), &alloc);
    assemble(alloc, source, None, (&segments, &links), owned).ok()
}

pub fn detect_assumed<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    _frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    match shell::wrapper(&source.body) {
        Some(_) => assume::detect(alloc, source, bodies),
        None => detect_with(alloc, source, _frame, bodies),
    }
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    _frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let Some(found) = shell(source) else {
        let list = shell::top(source)?;
        return detect_top(alloc, source, list, bodies);
    };
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
    match assemble(alloc, source, Some(&found), (&segments, &links), owned) {
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
    let paths = ModulePaths::of(bundle);
    for module in bundle.modules.values_mut() {
        let targets: FxHashMap<String, String> = module
            .dependencies
            .iter()
            .filter_map(|(id, _)| {
                Some((
                    id.clone(),
                    paths.specifier(module.path.as_str(), paths.get(id)?),
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
