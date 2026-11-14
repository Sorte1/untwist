use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_span::Span;

use crate::matcher::Factory;
use crate::model::{Bodies, Bundle, BundleKind};
use crate::webpack::container::{container_is_empty, is_module_container, module_factories};
use crate::webpack::entry::{attach, site};
use crate::webpack::extract::extract_modules_with;
use crate::webpack::gate::{routable, Prelude};
use crate::webpack::helpers::prelude_runtime_skipping;
use crate::webpack::machinery::{loading_of, refilled, settle};
use crate::webpack::proof::{deferred_entries, Proving, Shape};
use crate::webpack::require_fn::{find_assigned_entry_id, find_required_entry_id, Dispatchers};
use crate::webpack::require_form::{declared_record, Record};
use crate::webpack::startup::top_startup;
use crate::UnpackOptions;

struct Candidate<'k, 'a> {
    index: usize,
    name: &'k str,
    container: &'k Expression<'a>,
}

fn container_candidate<'k, 'a>(
    index: usize,
    stmt: &'k Statement<'a>,
) -> Option<(Candidate<'k, 'a>, bool)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let container = declarator.init.as_ref()?;
    if !is_module_container(container) {
        return None;
    }
    let candidate = Candidate {
        index,
        name: id.name.as_str(),
        container,
    };
    Some((candidate, container_is_empty(container)))
}

fn webpack5<'k, 'a>(body: &'k [Statement<'a>]) -> Option<(Candidate<'k, 'a>, &'k str)> {
    let dispatchers = Dispatchers::of(body);
    if dispatchers.is_empty() {
        return None;
    }
    body.iter().enumerate().find_map(|(index, stmt)| {
        let (candidate, empty) = container_candidate(index, stmt)?;
        let require_name = dispatchers.first_after(candidate.name, index)?;
        if empty
            && loading_of(body, require_name, &Bodies::Cloned)
                .installed
                .is_empty()
        {
            return None;
        }
        Some((candidate, require_name))
    })
}

pub fn is_webpack5(body: &[Statement<'_>]) -> bool {
    webpack5(body).is_some()
}

pub fn webpack5_record(body: &[Statement<'_>]) -> Record {
    webpack5(body).map_or_else(Record::default, |(candidate, require)| {
        declared_record(body, require, candidate.name)
    })
}

pub fn webpack5_factories<'k, 'a>(
    body: &'k [Statement<'a>],
) -> Option<Vec<(String, Factory<'k, 'a>)>> {
    webpack5(body).map(|(candidate, _)| module_factories(candidate.container))
}

pub fn detect_webpack5<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    body: &[Statement<'a>],
) -> Option<Bundle<'a>> {
    detect_webpack5_with(
        alloc,
        source,
        body,
        &mut Bodies::Cloned,
        UnpackOptions::default(),
    )
}

pub fn detect_webpack5_with<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    body: &[Statement<'a>],
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
) -> Option<Bundle<'a>> {
    detect_webpack5_noted(alloc, source, body, bodies, options, Proving::own(source))
}

pub fn detect_webpack5_noted<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    body: &[Statement<'a>],
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
    mut proving: Proving<'_>,
) -> Option<Bundle<'a>> {
    let (candidate, require_name) = webpack5(body)?;
    let loading = loading_of(body, require_name, bodies);
    let scratch = oxc_allocator::Allocator::default();
    let startup = match (
        options.conservative,
        refilled(&scratch, body, candidate.index, bodies),
    ) {
        (true, _) => None,
        (false, Some(copies)) => top_startup(&copies, require_name),
        (false, None) => top_startup(body, require_name),
    };
    let assigned = match &startup {
        Some(startup) => Some(startup.entry.clone()),
        None => find_assigned_entry_id(body, require_name, Some(candidate.index)),
    };
    let entry = match (&assigned, options.conservative) {
        (None, false) => site(source, (body, candidate.container)),
        _ => None,
    };
    if entry.as_ref().is_some_and(|entry| entry.unrecorded_head) {
        return proving.gate(Err(crate::notes::library_head_unproven()));
    }
    let deferred = deferred_entries(body, require_name);
    let started = !options.conservative && entry.is_none() && deferred.is_empty();
    let entry_id = assigned.or_else(|| {
        started
            .then(|| find_required_entry_id(body, require_name, Some(candidate.index)))
            .flatten()
    });
    let skip = entry.as_ref().map(|entry| entry.span);
    let returned: Vec<Span> = entry
        .iter()
        .filter_map(|entry| entry.returned)
        .map(|(_, span)| span)
        .collect();
    let checked = !matches!(bodies, Bodies::Taken(_)) && !options.conservative;
    if checked {
        let shape = Shape {
            params: None,
            statements: body,
            require: require_name,
            container: candidate.name,
            container_at: Some(candidate.index),
            skip,
            startup: startup.as_ref().map_or(&[], |startup| &startup.spans),
            proven: &returned,
        };
        proving.prove(source, &shape)?;
    }
    let mut bundle = Bundle::new(BundleKind::Webpack, entry_id);
    if !options.conservative {
        bundle.deferred = deferred;
    }
    bundle.runtime = prelude_runtime_skipping(alloc, (None, body), (require_name, skip), bodies);
    if let Some(runtime) = bundle.runtime.as_mut() {
        settle(alloc, runtime, &loading);
    }
    let loads_chunks = loading.settles_ensure();
    bundle.installed_chunks = loading.installed;
    bundle.startup_chunks = startup.map(|startup| startup.chunks).unwrap_or_default();
    if checked {
        let prelude = Prelude {
            runtime: bundle.runtime.as_ref(),
        };
        proving.gate(routable(
            source.source_text,
            candidate.container,
            &prelude,
            options,
        ))?;
    }
    let entry_id = bundle.entry_id.as_deref();
    (bundle.modules, bundle.fresh_requires) = extract_modules_with(
        alloc,
        source,
        (candidate.container, entry_id),
        bodies,
        options,
    );
    if let Some(entry) = &entry {
        let attached = attach(alloc, source, (entry, require_name), &mut bundle, bodies);
        if !attached && (loads_chunks || entry.returned.is_some()) {
            return proving.gate(Err(crate::notes::entry_not_shared()));
        }
    }
    Some(bundle)
}
