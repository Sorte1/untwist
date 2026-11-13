pub mod build;
pub mod outer;
pub mod plan;
pub mod shared;

use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::{Program, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::SourceType;
use untwist_core::semantic::build_scoping;

use super::scan::{scan, Context, Enclosing};
use super::shell::{assumed, Shell};
use super::wrappers;
use crate::model::{Bodies, Bundle, BundleKind};
use build::{assemble, layout, Layout};
use outer::Outer;
use plan::{boundaries, Boundary, Plan};

type Planned = (Plan, Layout);

fn library_boundaries() -> Vec<Boundary> {
    Vec::new()
}

fn outer_of(program: &Program<'_>, found: &Shell<'_, '_>) -> Option<Option<Outer>> {
    let Some(outer) = &found.outer else {
        return Some(None);
    };
    let strict = outer.body.has_use_strict_directive() || program.has_use_strict_directive();
    outer::analyse(outer, found.wrapper.index, strict).map(Some)
}

fn enclosing(found: &Shell<'_, '_>, outer: Option<&Outer>, on_source: bool) -> Option<Enclosing> {
    let outer = outer?;
    let scope = match on_source {
        true => found.outer.as_ref().and_then(|wrapper| wrapper.scope.get()),
        false => None,
    };
    Some(Enclosing {
        scope,
        names: outer.names.clone(),
    })
}

fn planned(
    scoping: &Scoping,
    items: &[Statement<'_>],
    context: &Context,
    (outer, text): (Option<&Outer>, &str),
) -> Option<Planned> {
    let facts = scan(scoping, items, context)?;
    let groups = wrappers::groups_with(scoping, items, &facts, true);
    let found = boundaries(groups, library_boundaries());
    let plan = plan::plan(scoping, items, &facts, found, outer)?;
    let layout = layout(scoping, &facts, &plan, text);
    Some((plan, layout))
}

fn on_source(
    source: &Program<'_>,
    found: &Shell<'_, '_>,
    outer: Option<&Outer>,
) -> Option<Planned> {
    let semantics = build_scoping(source);
    let context = Context {
        scope: found.wrapper.scope.get()?,
        arrow: found.wrapper.arrow,
        module: false,
        enclosing: enclosing(found, outer, true),
    };
    let items = &found.wrapper.body.statements;
    planned(
        &semantics.scoping,
        items,
        &context,
        (outer, source.source_text),
    )
}

fn detached<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    found: &Shell<'_, 'a>,
    outer: Option<&Outer>,
    owned: AVec<'a, Statement<'a>>,
) -> (Option<Planned>, AVec<'a, Statement<'a>>) {
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
        enclosing: enclosing(found, outer, false),
    };
    let plan = planned(
        &semantics.scoping,
        &temporary.body,
        &context,
        (outer, source.source_text),
    );
    let owned = std::mem::replace(&mut temporary.body, AVec::new_in(&builder));
    (plan, owned)
}

fn attempt<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    found: &Shell<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let outer = outer_of(source, found)?;
    let (plan, owned) = match bodies {
        Bodies::Taken(_) => {
            let owned = bodies.statements(alloc, found.wrapper.body);
            detached(alloc, source, found, outer.as_ref(), owned)
        }
        Bodies::Planned(_) | Bodies::Cloned => {
            let plan = on_source(source, found, outer.as_ref())?;
            let owned = bodies.statements(alloc, found.wrapper.body);
            if matches!(bodies, Bodies::Planned(_)) {
                return Some(Bundle::new(
                    BundleKind::Rollup,
                    Some(super::build::ENTRY_ID.to_string()),
                ));
            }
            (Some(plan), owned)
        }
    };
    let Some((plan, layout)) = plan else {
        super::give_back(bodies, found, owned);
        return None;
    };
    match assemble(alloc, source, found, (&plan, &layout), owned) {
        Ok(bundle) => Some(bundle),
        Err(owned) => {
            super::give_back(bodies, found, owned);
            None
        }
    }
}

pub fn detect<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    assumed(source)
        .iter()
        .find_map(|found| attempt(alloc, source, found, bodies))
}
