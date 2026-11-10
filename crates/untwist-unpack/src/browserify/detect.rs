use indexmap::IndexMap;
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_core::ast_util::prop::prop_name;

use crate::browserify::tree::{dependency_tree, infer_index_directories, resolve_dependency_tree};
use crate::matcher::{
    anonymous_factory, argument_expression, call_parts, literal_id, param_names, Factory,
};
use crate::model::{module_from_body_with, Bodies, Bundle, BundleKind, Module, RequireParam};
use crate::path::PosixPath;
use crate::rename::rename_factory_parameters;
use crate::UnpackOptions;

pub const BROWSERIFY_PARAMS: [&str; 3] = ["require", "module", "exports"];
const REQUIRE_PARAM: usize = 0;

type Entries = (String, Vec<Option<String>>);

fn require_param(params: usize, fresh: bool) -> RequireParam {
    match (params > REQUIRE_PARAM, fresh) {
        (false, _) => RequireParam::Absent,
        (true, true) => RequireParam::Fresh,
        (true, false) => RequireParam::Default,
    }
}

fn three_identifier_params(params: &FormalParameters<'_>) -> bool {
    params.rest.is_none()
        && params.items.len() == 3
        && params
            .items
            .iter()
            .all(|item| matches!(item.pattern, BindingPattern::BindingIdentifier(_)))
}

fn is_prelude(expr: &Expression<'_>) -> bool {
    if let Expression::FunctionExpression(function) = expr {
        return function.body.is_some() && three_identifier_params(&function.params);
    }
    let Some((callee, _)) = call_parts(expr) else {
        return false;
    };
    let Some(factory) = anonymous_factory(callee) else {
        return false;
    };
    if !factory.params.items.is_empty() || factory.params.rest.is_some() {
        return false;
    }
    if factory.body.statements.len() != 2 {
        return false;
    }
    let init = matches!(
        factory.body.statements.first(),
        Some(Statement::FunctionDeclaration(function))
            if three_identifier_params(&function.params)
    );
    let returns = matches!(
        factory.body.statements.get(1),
        Some(Statement::ReturnStatement(statement))
            if matches!(statement.argument, Some(Expression::Identifier(_)))
    );
    init && returns
}

fn dependency_list(object: &ObjectExpression<'_>) -> Option<Vec<(String, String)>> {
    let mut dependencies = Vec::with_capacity(object.properties.len());
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.method {
            return None;
        }
        let request = match &property.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
            PropertyKey::StringLiteral(literal) => literal.value.as_str()?.to_string(),
            _ => return None,
        };
        if matches!(&property.value, Expression::Identifier(id) if id.name == "undefined") {
            continue;
        }
        dependencies.push((literal_id(&property.value)?, request));
    }
    Some(dependencies)
}

struct ModuleEntry<'k, 'a> {
    id: String,
    params: &'k FormalParameters<'a>,
    body: &'k FunctionBody<'a>,
    dependencies: Vec<(String, String)>,
}

fn module_entry<'k, 'a>(property: &'k ObjectPropertyKind<'a>) -> Option<ModuleEntry<'k, 'a>> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    if property.computed || property.method {
        return None;
    }
    let id = prop_name(&property.key)?.into_owned();
    let Expression::ArrayExpression(array) = &property.value else {
        return None;
    };
    if array.elements.len() != 2 {
        return None;
    }
    let Expression::FunctionExpression(function) = array.elements.first()?.as_expression()? else {
        return None;
    };
    let Expression::ObjectExpression(deps) = array.elements.get(1)?.as_expression()? else {
        return None;
    };
    Some(ModuleEntry {
        id,
        params: &function.params,
        body: function.body.as_deref()?,
        dependencies: dependency_list(deps)?,
    })
}

fn assign_resolved_paths(bundle: &mut Bundle<'_>, resolved: &IndexMap<String, String>) {
    for (id, path) in resolved {
        let Some(module) = bundle.modules.get_mut(id) else {
            continue;
        };
        match PosixPath::parse(path) {
            Some(parsed) => {
                module.path = parsed;
                module.unsafe_path_source = None;
            }
            None => module.unsafe_path_source = Some(path.clone()),
        }
    }
}

fn browserify<'k, 'a>(expr: &'k Expression<'a>) -> Option<(&'k ObjectExpression<'a>, Entries)> {
    let (callee, arguments) = call_parts(expr)?;
    if arguments.len() != 3 || !is_prelude(callee) {
        return None;
    }
    let Expression::ObjectExpression(files) = argument_expression(arguments.first()?)? else {
        return None;
    };
    if !matches!(
        argument_expression(arguments.get(1)?)?,
        Expression::ObjectExpression(_)
    ) {
        return None;
    }
    let Expression::ArrayExpression(entries) = argument_expression(arguments.get(2)?)? else {
        return None;
    };
    let entry_id = literal_id(entries.elements.first()?.as_expression()?)?;
    let later = entries
        .elements
        .iter()
        .skip(1)
        .map(|element| element.as_expression().and_then(literal_id))
        .collect();
    Some((files, (entry_id, later)))
}

pub fn is_browserify(expr: &Expression<'_>) -> bool {
    browserify(expr).is_some_and(|(files, _)| {
        !files.properties.is_empty()
            && files
                .properties
                .iter()
                .all(|property| module_entry(property).is_some())
    })
}

pub fn browserify_factories<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<Vec<(String, Factory<'k, 'a>)>> {
    let (files, _) = browserify(expr)?;
    files
        .properties
        .iter()
        .map(|property| {
            let entry = module_entry(property)?;
            let factory = Factory {
                params: entry.params,
                body: entry.body,
            };
            Some((entry.id, factory))
        })
        .collect()
}

pub fn detect_browserify<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
) -> Option<Bundle<'a>> {
    detect_browserify_with(
        alloc,
        source,
        expr,
        &mut Bodies::Cloned,
        UnpackOptions::default(),
    )
}

pub fn detect_browserify_with<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
) -> Option<Bundle<'a>> {
    let (files, (entry_id, later_entries)) = browserify(expr)?;
    let mut bundle = Bundle::new(BundleKind::Browserify, Some(entry_id.clone()));
    bundle.later_entries = later_entries;
    for property in &files.properties {
        let entry = module_entry(property)?;
        let mut program = module_from_body_with(alloc, source, entry.body, bodies);
        let params = param_names(entry.params);
        let fresh = rename_factory_parameters(
            alloc,
            &mut program,
            (&params, &BROWSERIFY_PARAMS),
            options.conservative,
        )
        .into_iter()
        .find_map(|(index, name)| (index == REQUIRE_PARAM).then_some(name));
        let is_entry = entry.id == entry_id;
        let mut module = Module::new(entry.id.clone(), program, is_entry);
        module.dependencies = entry.dependencies;
        module.runtime_require = require_param(params.len(), fresh.is_some());
        bundle
            .fresh_requires
            .extend(fresh.map(|name| (entry.id.clone(), name)));
        bundle.modules.insert(entry.id, module);
    }
    if bundle.modules.is_empty() {
        return None;
    }
    let tree = dependency_tree(&bundle);
    let index_dirs = infer_index_directories(&tree, &entry_id);
    let resolved = resolve_dependency_tree(&tree, &entry_id, &index_dirs);
    assign_resolved_paths(&mut bundle, &resolved);
    Some(bundle)
}
