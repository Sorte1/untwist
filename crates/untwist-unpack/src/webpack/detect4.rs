use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::matcher::{argument_expression, call_parts, single_param_name, Factory};
use crate::model::{Bodies, Bundle, BundleKind};
use crate::webpack::container::{is_module_container, module_factories};
use crate::webpack::extract::{extract_modules_with, strip_trailing_marker_comment};
use crate::webpack::gate::{routable, Prelude};
use crate::webpack::helpers::prelude_runtime;
use crate::webpack::machinery::{installed_declared, jsonp_block, loading_of, settle};
use crate::webpack::proof::{deferred_entries, Proving, Shape};
use crate::webpack::require_fn::{
    count_required_entry_ids, find_assigned_entry_id, find_required_entry_id, require_function,
};
use crate::webpack::startup::webpack4_deferred;
use crate::UnpackOptions;
use oxc_span::Span;

struct Webpack4<'k, 'a> {
    container: &'k Expression<'a>,
    params: &'k FormalParameters<'a>,
    body: &'k FunctionBody<'a>,
    require_name: &'k str,
}

fn webpack4<'k, 'a>(expr: &'k Expression<'a>) -> Option<Webpack4<'k, 'a>> {
    let (callee, arguments) = call_parts(expr)?;
    if arguments.len() != 1 {
        return None;
    }
    let container = argument_expression(arguments.first()?)?;
    if !is_module_container(container) {
        return None;
    }
    let Expression::FunctionExpression(bootstrap) = callee else {
        return None;
    };
    if bootstrap.id.is_some() || bootstrap.generator || bootstrap.r#async {
        return None;
    }
    let container_name = single_param_name(&bootstrap.params)?;
    let body = bootstrap.body.as_deref()?;
    let require_name = body
        .statements
        .iter()
        .find_map(|stmt| require_function(stmt, container_name))?
        .name;
    Some(Webpack4 {
        container,
        params: &bootstrap.params,
        body,
        require_name,
    })
}

fn deferred_check<'k>(statements: &'k [Statement<'_>]) -> Option<&'k str> {
    let call = match statements.last()? {
        Statement::ExpressionStatement(statement) => &statement.expression,
        Statement::ReturnStatement(ret) => ret.argument.as_ref()?,
        _ => return None,
    };
    let Expression::CallExpression(call) = call.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.without_parentheses() else {
        return None;
    };
    let declared = statements.iter().any(|statement| {
        matches!(statement, Statement::FunctionDeclaration(function)
            if function.id.as_ref().is_some_and(|id| id.name == callee.name))
    });
    declared.then(|| callee.name.as_str())
}

pub fn is_webpack4(expr: &Expression<'_>) -> bool {
    webpack4(expr).is_some()
}

pub fn webpack4_factories<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<Vec<(String, Factory<'k, 'a>)>> {
    webpack4(expr).map(|found| module_factories(found.container))
}

pub fn detect_webpack4<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
) -> Option<Bundle<'a>> {
    detect_webpack4_with(
        alloc,
        source,
        expr,
        &mut Bodies::Cloned,
        UnpackOptions::default(),
    )
}

pub fn detect_webpack4_with<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
) -> Option<Bundle<'a>> {
    detect_webpack4_noted(alloc, source, expr, bodies, options, Proving::own(source))
}

pub fn detect_webpack4_noted<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
    mut proving: Proving<'_>,
) -> Option<Bundle<'a>> {
    let Webpack4 {
        container,
        params,
        body,
        require_name,
    } = webpack4(expr)?;
    let loading = loading_of(&body.statements, require_name, bodies);
    let block = jsonp_block(source.source_text, &body.statements, require_name);
    let deferred = block
        .as_ref()
        .and_then(|_| deferred_check(&body.statements))
        .and_then(|check| webpack4_deferred(&body.statements, check));
    let proven: Vec<Span> = block
        .iter()
        .flatten()
        .chain(deferred.iter().flat_map(|deferred| &deferred.spans))
        .copied()
        .collect();
    let checked = !matches!(bodies, Bodies::Taken(_)) && !options.conservative;
    if checked {
        let container_name = single_param_name(params)?;
        let shape = Shape {
            params: Some(params),
            statements: &body.statements,
            require: require_name,
            container: container_name,
            container_at: None,
            skip: None,
            startup: &[],
            proven: &proven,
        };
        proving.prove(source, &shape)?;
    }
    let assigned = find_assigned_entry_id(&body.statements, require_name, None);
    let started = deferred
        .as_ref()
        .and_then(|deferred| deferred.entry.clone());
    let entry_id = match &started {
        Some((entry, _)) => Some(entry.clone()),
        None => assigned
            .clone()
            .or_else(|| find_required_entry_id(&body.statements, require_name, None)),
    };

    let mut bundle = Bundle::new(BundleKind::Webpack, entry_id);
    if !options.conservative {
        bundle.deferred = deferred_entries(&body.statements, require_name);
    }
    if assigned.is_none() {
        bundle.entry_candidates = count_required_entry_ids(&body.statements, require_name);
    }
    bundle.runtime = prelude_runtime(
        alloc,
        (Some(params), &body.statements),
        require_name,
        bodies,
    );
    if let Some(runtime) = bundle.runtime.as_mut() {
        settle(alloc, runtime, &loading);
    }
    let settled = loading.settles_ensure();
    bundle.installed_chunks = loading.installed;
    if block.is_some() || settled {
        bundle
            .installed_chunks
            .extend(installed_declared(&body.statements));
    }
    bundle.runs_deferred = deferred.is_some();
    if let Some((_, chunks)) = started {
        bundle.startup_chunks = chunks;
    }
    if checked {
        let prelude = Prelude {
            runtime: bundle.runtime.as_ref(),
        };
        proving.gate(routable(source.source_text, container, &prelude, options))?;
    }
    let entry_id = bundle.entry_id.as_deref();
    (bundle.modules, bundle.fresh_requires) =
        extract_modules_with(alloc, source, (container, entry_id), bodies, options);
    for module in bundle.modules.values_mut() {
        strip_trailing_marker_comment(module.program_mut());
    }
    Some(bundle)
}
