pub mod build;
pub mod check;
pub mod late;
pub mod layout;
pub mod links;
pub mod live;
pub mod mentions;
pub mod notes;
pub mod rewrite;
pub mod shape;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::Note;
use untwist_core::semantic::build_scoping;

use crate::esbuild::build::{
    candidates, factory_programs, has_use_strict, scan, wanted_helpers, Plan,
};
use crate::esbuild::check::{entry_sound, factory_closed, Proof};
use crate::esbuild::layout::{
    layout, layout_linked, scopes, FactoryDecl, Host, Layout, Scope, INLINE_PREFIX,
};
use crate::esbuild::live::read_as_members;
use crate::esbuild::mentions::scoped_closure;
use crate::esbuild::shape::CallForm;
use crate::frame::Frame;
use crate::links::Links;
use crate::model::{body_key, Bodies, Bundle, BundleKind};
use crate::rolldown;
use crate::runtime::{first_unclosed_helper, helper_symbols};

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    scopes(frame, &[]).any(|scope| layout(scope.list).is_some()) || links::may_link(frame.top)
}

fn found<'k, 'a>(
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    view: Option<&FxHashMap<String, CallForm>>,
) -> Option<Plan<'k, 'a>> {
    let (scope, mut layout) = scopes(frame, &source.directives).find_map(|scope| {
        let view = view.filter(|_| scope.host == Host::Top);
        layout_linked(scope.list, view).map(|layout| (scope, layout))
    })?;
    if source.source_text.contains(INLINE_PREFIX) {
        layout.inline.clear();
    }
    let module_code = frame.top.iter().any(Statement::is_module_declaration);
    let strict =
        module_code || has_use_strict(&source.directives) || has_use_strict(scope.directives);
    Some(Plan {
        scope,
        layout,
        strict,
        module: module_code,
    })
}

fn helper_names(plan: &Plan<'_, '_>) -> (FxHashMap<String, usize>, FxHashMap<String, usize>) {
    let candidates = candidates(plan.scope.list, &plan.layout);
    let shim = plan.layout.shim.map(|shim| shim.name.as_str());
    let wanted = wanted_helpers(&candidates, shim);
    (candidates, wanted)
}

fn inside_factory(scoping: &Scoping, factories: &FxHashSet<ScopeId>, scope: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| factories.contains(&ancestor))
}

fn seeds_in(
    scoping: &Scoping,
    plan: &Plan<'_, '_>,
    wanted: &FxHashMap<String, usize>,
    list_scope: ScopeId,
) -> FxHashSet<usize> {
    let factories: FxHashSet<ScopeId> = plan
        .layout
        .all()
        .filter_map(|factory| factory.scope.get())
        .collect();
    let mut seeds = FxHashSet::default();
    let referenced_inside = |references: &[ReferenceId]| {
        references.iter().any(|&reference| {
            inside_factory(
                scoping,
                &factories,
                scoping.get_reference(reference).scope_id(),
            )
        })
    };
    for (name, &symbol) in scoping.get_bindings(list_scope) {
        if let Some(&index) = wanted.get(name.as_str()) {
            if referenced_inside(scoping.get_resolved_reference_ids(symbol)) {
                seeds.insert(index);
            }
        }
    }
    for (name, references) in scoping.root_unresolved_references().iter() {
        if let Some(&index) = wanted.get(name.as_str()) {
            if referenced_inside(references) {
                seeds.insert(index);
            }
        }
    }
    seeds
}

fn wrapper_scope(scope: &Scope<'_, '_>) -> Option<ScopeId> {
    let Statement::ExpressionStatement(statement) = scope.wrapper? else {
        return None;
    };
    match statement.expression.without_parentheses() {
        Expression::CallExpression(call) => match call.callee.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => arrow.scope_id.get(),
            Expression::FunctionExpression(function) => function.scope_id.get(),
            _ => None,
        },
        Expression::FunctionExpression(function) => function.scope_id.get(),
        _ => None,
    }
}

fn hosts(plan: &Plan<'_, '_>) -> FxHashSet<SymbolId> {
    let mut hosts = FxHashSet::default();
    if plan.scope.host == Host::Bun {
        hosts.extend(plan.scope.function.and_then(crate::bun::require_param));
    }
    if let Some(Statement::VariableDeclaration(banner)) = plan
        .layout
        .banner
        .and_then(|index| plan.scope.list.get(index))
    {
        if let Some(BindingPattern::BindingIdentifier(id)) =
            banner.declarations.first().map(|declarator| &declarator.id)
        {
            hosts.extend(id.symbol_id.get());
        }
    }
    hosts
}

fn host_params(plan: &Plan<'_, '_>) -> FxHashSet<SymbolId> {
    let Some(wrapper) = plan.scope.function.filter(|_| plan.scope.host == Host::Bun) else {
        return FxHashSet::default();
    };
    wrapper
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn shim_init(plan: &Plan<'_, '_>) -> Option<oxc_span::Span> {
    let shim = plan.layout.shim?;
    plan.layout.prefix.iter().find_map(|&index| {
        let Statement::VariableDeclaration(declaration) = plan.scope.list.get(index)? else {
            return None;
        };
        declaration
            .declarations
            .iter()
            .find_map(|declarator| match &declarator.id {
                BindingPattern::BindingIdentifier(id) if id.span == shim.span => {
                    declarator.init.as_ref().map(oxc_span::GetSpan::span)
                }
                _ => None,
            })
    })
}

fn proof<'s>(scoping: &'s Scoping, plan: &Plan<'_, '_>, moved: &[usize]) -> Proof<'s> {
    let list = plan.scope.list;
    let requires: FxHashMap<SymbolId, oxc_span::Span> = plan
        .layout
        .factories
        .iter()
        .filter_map(|factory| {
            let span = match &list[factory.statement] {
                Statement::VariableDeclaration(declaration) => {
                    declaration.declarations[factory.declarator].span
                }
                other => oxc_span::GetSpan::span(other),
            };
            factory
                .binding?
                .symbol_id
                .get()
                .map(|symbol| (symbol, span))
        })
        .collect();
    Proof {
        scoping,
        requires,
        shim: plan.layout.shim.and_then(|shim| shim.symbol_id.get()),
        shim_loads: !plan.module || plan.layout.banner.is_some() || plan.layout.created_shim,
        hosts: hosts(plan),
        host_params: host_params(plan),
        shim_init: shim_init(plan),
        helpers: helper_symbols(list, moved),
    }
}

fn entry_proven(proof: &Proof<'_>, plan: &Plan<'_, '_>, moved: &[usize]) -> bool {
    let list = plan.scope.list;
    let layout: &Layout = &plan.layout;
    let statements = list
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            moved.binary_search(index).is_err()
                && !layout.is_factory_statement(*index)
                && !layout.is_map_statement(*index)
        })
        .map(|(_, statement)| statement);
    let declarators = list
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| match statement {
            Statement::VariableDeclaration(declaration) => {
                let factories = layout.factories_at(index);
                (!factories.is_empty()).then_some((factories, declaration))
            }
            _ => None,
        })
        .flat_map(|(factories, declaration)| {
            declaration
                .declarations
                .iter()
                .enumerate()
                .filter(move |(at, _)| {
                    factories
                        .binary_search_by_key(at, |factory| factory.declarator)
                        .is_err()
                })
                .map(|(_, declarator)| declarator)
        });
    let unwrapped = plan.scope.host != Host::Function
        || rolldown::receiver::receiver_free(proof.scoping, statements.clone());
    unwrapped && entry_sound(proof, statements, declarators)
}

fn unchanged(scoping: &Scoping, binding: &BindingIdentifier<'_>) -> bool {
    binding.symbol_id.get().is_some_and(|symbol| {
        scoping.symbol_redeclarations(symbol).is_empty()
            && !scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
    })
}

fn helpers_unchanged(scoping: &Scoping, layout: &Layout<'_, '_>) -> bool {
    layout
        .trusted
        .iter()
        .copied()
        .chain(layout.shim)
        .all(|binding| unchanged(scoping, binding))
}

fn maps_used_once(scoping: &Scoping, layout: &Layout<'_, '_>) -> bool {
    layout.maps().all(|map| {
        unchanged(scoping, map.binding)
            && map
                .binding
                .symbol_id
                .get()
                .is_some_and(|symbol| scoping.get_resolved_reference_ids(symbol).len() == 1)
    })
}

fn list_scope(scoping: &Scoping, plan: &Plan<'_, '_>) -> Option<ScopeId> {
    match plan.scope.host {
        Host::Top => Some(scoping.root_scope_id()),
        Host::Iife | Host::Function | Host::Bun => wrapper_scope(&plan.scope),
    }
}

enum Round {
    Proven(Vec<usize>),
    Keep(Vec<bool>),
}

fn round(scoping: &Scoping, plan: &Plan<'_, '_>) -> Option<Round> {
    let ordered = !plan.layout.early
        || late::first_call_follows_definition(scoping, plan.scope.list, &plan.layout);
    if !ordered {
        return None;
    }
    let (candidates, wanted) = helper_names(plan);
    let list_scope = list_scope(scoping, plan)?;
    let moved = scoped_closure(
        scoping,
        list_scope,
        plan.scope.list,
        &candidates,
        seeds_in(scoping, plan, &wanted, list_scope),
    );
    let proof = proof(scoping, plan, &moved);
    let free_object = scoping
        .find_binding(list_scope, oxc_str::Ident::from("Object"))
        .is_none();
    if !free_object || !proof.requires_host_at(list_scope) {
        return None;
    }
    if first_unclosed_helper(scoping, plan.scope.list, &moved).is_some() {
        return None;
    }
    let declared_once = proof
        .requires
        .keys()
        .chain(&proof.helpers)
        .all(|&symbol| scoping.symbol_redeclarations(symbol).is_empty());
    if !declared_once || !read_as_members(scoping, plan.scope.list, &moved, &proof.helpers) {
        return None;
    }
    let closed: Vec<bool> = plan
        .layout
        .all()
        .map(|factory| factory_closed(&proof, factory))
        .collect();
    if closed.contains(&false) {
        let closed = kept_users(scoping, plan, closed);
        if !closed.contains(&true) {
            return None;
        }
        return Some(Round::Keep(closed));
    }
    entry_proven(&proof, plan, &moved).then_some(Round::Proven(moved))
}

fn kept_users(scoping: &Scoping, plan: &Plan<'_, '_>, mut closed: Vec<bool>) -> Vec<bool> {
    let all: Vec<&FactoryDecl<'_, '_>> = plan.layout.all().collect();
    let owners: FxHashMap<ScopeId, usize> = all
        .iter()
        .enumerate()
        .filter_map(|(index, factory)| Some((factory.scope.get()?, index)))
        .collect();
    let mut users: Vec<Vec<usize>> = vec![Vec::new(); all.len()];
    for (index, factory) in all.iter().enumerate() {
        let Some(symbol) = factory.binding.and_then(|binding| binding.symbol_id.get()) else {
            continue;
        };
        for reference in scoping.get_resolved_references(symbol) {
            let user = scoping
                .scope_ancestors(reference.scope_id())
                .find_map(|scope| owners.get(&scope).copied());
            users[index].extend(user.filter(|&user| user != index));
        }
    }
    let mut pending: Vec<usize> = (0..all.len()).filter(|&index| !closed[index]).collect();
    while let Some(kept) = pending.pop() {
        for &user in &users[kept] {
            if closed[user] {
                closed[user] = false;
                pending.push(user);
            }
        }
    }
    closed
}

fn retain<T>(items: &mut Vec<T>, keep: &[bool]) {
    let mut at = 0;
    items.retain(|_| {
        at += 1;
        keep[at - 1]
    });
}

fn proven(source: &Program<'_>, plan: &mut Plan<'_, '_>) -> Option<Vec<usize>> {
    if plan.layout.unsound {
        return None;
    }
    let scoping = build_scoping(source).scoping;
    if !helpers_unchanged(&scoping, &plan.layout) || !maps_used_once(&scoping, &plan.layout) {
        return None;
    }
    loop {
        match round(&scoping, plan)? {
            Round::Proven(moved) => return Some(moved),
            Round::Keep(closed) => {
                let (declared, inline) = closed.split_at(plan.layout.factories.len());
                retain(&mut plan.layout.factories, declared);
                retain(&mut plan.layout.inline, inline);
            }
        }
    }
}

fn taken_only(plan: &mut Plan<'_, '_>, bodies: &Bodies<'_>) {
    let Bodies::Taken(pool) = bodies else {
        return;
    };
    let taken = |factory: &FactoryDecl<'_, '_>| pool.contains_key(&body_key(factory.function.body));
    plan.layout.factories.retain(taken);
    plan.layout.inline.retain(taken);
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
    links: &Links,
) -> Option<Bundle<'a>> {
    let view = links.esbuild.as_ref().map(|view| &view.helpers);
    let mut plan = found(source, frame, view)?;
    if plan.layout.kind == BundleKind::Esbuild && rolldown::marked(source) {
        plan.layout.kind = BundleKind::Rolldown;
    }
    detect_plan(alloc, source, plan, bodies)
}

fn detect_plan<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    mut plan: Plan<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let taken = matches!(bodies, Bodies::Taken(_));
    let proven_moved = if taken {
        taken_only(&mut plan, bodies);
        None
    } else {
        Some(proven(source, &mut plan)?)
    };
    let programs = factory_programs(alloc, source, &plan, bodies);
    if matches!(bodies, Bodies::Planned(_)) {
        return Some(Bundle::new(plan.layout.kind, None));
    }
    let (candidates, wanted) = helper_names(&plan);
    let scanned = scan(&plan, &wanted, &programs);
    let moved = match proven_moved {
        Some(moved) => moved,
        None => {
            let scoping = build_scoping(source).scoping;
            let scope = list_scope(&scoping, &plan)?;
            scoped_closure(&scoping, scope, plan.scope.list, &candidates, scanned.seeds)
        }
    };
    build::bundle(
        alloc,
        source,
        &plan,
        (programs, scanned.dependencies),
        &moved,
    )
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    rewrite::rewrite(alloc, bundle)
}
