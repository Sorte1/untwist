pub mod build;
pub mod check;
pub mod layout;
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
use crate::esbuild::layout::{layout, scopes, Host, Layout, Scope};
use crate::esbuild::live::read_as_members;
use crate::esbuild::mentions::closure;
use crate::frame::Frame;
use crate::model::{Bodies, Bundle, BundleKind};
use crate::runtime::{first_unclosed_helper, helper_symbols};

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    scopes(frame, &[]).any(|scope| layout(scope.list).is_some())
}

fn found<'k, 'a>(source: &'k Program<'a>, frame: &Frame<'k, 'a>) -> Option<Plan<'k, 'a>> {
    let (scope, layout) = scopes(frame, &source.directives)
        .find_map(|scope| layout(scope.list).map(|layout| (scope, layout)))?;
    let module_code = frame.top.iter().any(Statement::is_module_declaration);
    let strict =
        module_code || has_use_strict(&source.directives) || has_use_strict(scope.directives);
    Some(Plan {
        scope,
        layout,
        strict,
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
) -> FxHashSet<usize> {
    let factories: FxHashSet<ScopeId> = plan
        .layout
        .factories
        .iter()
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
    for symbol in scoping.symbol_ids() {
        if let Some(&index) = wanted.get(scoping.symbol_name(symbol)) {
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
            let span = oxc_span::GetSpan::span(&list[factory.statement]);
            factory.binding.symbol_id.get().map(|symbol| (symbol, span))
        })
        .collect();
    Proof {
        scoping,
        requires,
        shim: plan.layout.shim.and_then(|shim| shim.symbol_id.get()),
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
            moved.binary_search(index).is_err() && !layout.is_factory_statement(*index)
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
    entry_sound(proof, statements, declarators)
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

fn proven(source: &Program<'_>, plan: &Plan<'_, '_>) -> Option<Vec<usize>> {
    if plan.layout.unsound {
        return None;
    }
    let scoping = build_scoping(source).scoping;
    if !helpers_unchanged(&scoping, &plan.layout) {
        return None;
    }
    let (candidates, wanted) = helper_names(plan);
    let moved = closure(
        plan.scope.list,
        &candidates,
        seeds_in(&scoping, plan, &wanted),
    );
    let proof = proof(&scoping, plan, &moved);
    let list_scope = match plan.scope.host {
        Host::Top => Some(scoping.root_scope_id()),
        Host::Iife | Host::Bun => wrapper_scope(&plan.scope),
    };
    let Some(list_scope) = list_scope else {
        return None;
    };
    let free_object = scoping
        .find_binding(list_scope, oxc_str::Ident::from("Object"))
        .is_none();
    if !free_object || !proof.requires_host_at(list_scope) {
        return None;
    }
    if first_unclosed_helper(&scoping, plan.scope.list, &moved).is_some() {
        return None;
    }
    let declared_once = proof
        .requires
        .keys()
        .chain(&proof.helpers)
        .all(|&symbol| scoping.symbol_redeclarations(symbol).is_empty());
    if !declared_once || !read_as_members(&scoping, plan.scope.list, &moved, &proof.helpers) {
        return None;
    }
    let factories = plan
        .layout
        .factories
        .iter()
        .all(|factory| factory_closed(&proof, factory));
    (factories && entry_proven(&proof, plan, &moved)).then_some(moved)
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let plan = found(source, frame)?;
    let taken = matches!(bodies, Bodies::Taken(_));
    let proven_moved = if taken {
        None
    } else {
        Some(proven(source, &plan)?)
    };
    let programs = factory_programs(alloc, source, &plan, bodies);
    if matches!(bodies, Bodies::Planned(_)) {
        return Some(Bundle::new(BundleKind::Esbuild, None));
    }
    let (candidates, wanted) = helper_names(&plan);
    let scanned = scan(&plan, &wanted, &programs);
    let moved =
        proven_moved.unwrap_or_else(|| closure(plan.scope.list, &candidates, scanned.seeds));
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
