use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::model::{Bodies, Bundle, BundleKind};
use crate::webpack::container::{container_is_empty, is_module_container};
use crate::webpack::entry::{attach, site};
use crate::webpack::extract::extract_modules_with;
use crate::webpack::gate::{routable, Prelude};
use crate::webpack::helpers::prelude_runtime_skipping;
use crate::webpack::require_fn::{find_assigned_entry_id, Dispatchers};
use crate::webpack::runtime_module::member_names_in;
use crate::UnpackOptions;

struct Candidate<'k, 'a> {
    index: usize,
    name: &'k str,
    container: &'k Expression<'a>,
}

fn container_candidate<'k, 'a>(index: usize, stmt: &'k Statement<'a>) -> Option<Candidate<'k, 'a>> {
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
    if !is_module_container(container) || container_is_empty(container) {
        return None;
    }
    Some(Candidate {
        index,
        name: id.name.as_str(),
        container,
    })
}

fn webpack5<'k, 'a>(body: &'k [Statement<'a>]) -> Option<(Candidate<'k, 'a>, &'k str)> {
    let dispatchers = Dispatchers::of(body);
    if dispatchers.is_empty() {
        return None;
    }
    body.iter().enumerate().find_map(|(index, stmt)| {
        let candidate = container_candidate(index, stmt)?;
        let require_name = dispatchers.first_after(candidate.name, index)?;
        Some((candidate, require_name))
    })
}

pub fn is_webpack5(body: &[Statement<'_>]) -> bool {
    webpack5(body).is_some()
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
    let (candidate, require_name) = webpack5(body)?;
    let entry_id = find_assigned_entry_id(body, require_name, Some(candidate.index));
    let entry = match (&entry_id, options.conservative) {
        (None, false) => site(source, (body, candidate.container)),
        _ => None,
    };
    let mut bundle = Bundle::new(BundleKind::Webpack, entry_id);
    bundle.runtime = prelude_runtime_skipping(
        alloc,
        (None, body),
        (require_name, entry.as_ref().map(|entry| entry.span)),
        bodies,
    );
    if !matches!(bodies, Bodies::Taken(_))
        && !routable(
            source.source_text,
            candidate.container,
            &Prelude {
                runtime: bundle.runtime.as_ref(),
                mentioned: member_names_in(
                    require_name,
                    body.iter()
                        .enumerate()
                        .filter(|(index, _)| *index != candidate.index)
                        .map(|(_, statement)| statement),
                ),
            },
            options,
        )
    {
        return None;
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
        attach(alloc, source, (entry, require_name), &mut bundle, bodies);
    }
    Some(bundle)
}
