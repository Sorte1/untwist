use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::{Ident, Str};
use rustc_hash::FxHashMap;
use untwist_core::semantic::build_scoping;

use super::links::{loader, Form};
use super::{resolve, single_string};
use crate::esbuild::mentions::{binds_any, mentioned};
use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{
    declared_names, js_string, runtime_id, runtime_specifier, synthetic_statements,
};
use crate::specifier::ModulePaths;

const REQUIRE: &str = "require";

struct Rewrite<'a, 'r> {
    builder: AstBuilder<'a>,
    scoping: Option<&'r Scoping>,
    callee: &'r str,
    hoisted: bool,
    deps: &'r [(String, String)],
    paths: &'r ModulePaths,
    from: &'r str,
}

impl<'a> Rewrite<'a, '_> {
    fn free(&self, id: &IdentifierReference<'_>) -> bool {
        crate::scan::is_free(self.scoping, id)
    }

    fn specifier(&self, target: &str) -> Option<String> {
        match self.paths.get(target) {
            Some(path) => Some(self.paths.specifier(self.from, path)),
            None => (!self.hoisted).then(|| target.to_string()),
        }
    }

    fn required(&self, call: &CallExpression<'_>) -> Option<String> {
        let Expression::Identifier(callee) = &call.callee else {
            return None;
        };
        if call.optional || callee.name != self.callee || !self.free(callee) {
            return None;
        }
        let spec = single_string(call)?;
        let target = if self.hoisted {
            spec
        } else {
            resolve(self.deps, spec)
        };
        self.specifier(target)
    }

    fn rooted(&self, call: &CallExpression<'_>) -> Option<String> {
        let Expression::StaticMemberExpression(root) = &call.callee else {
            return None;
        };
        let Expression::StaticMemberExpression(bundle) = &root.object else {
            return None;
        };
        let Expression::Identifier(module) = &bundle.object else {
            return None;
        };
        let fits = !self.hoisted
            && root.property.name == "root"
            && bundle.property.name == "bundle"
            && module.name == "module"
            && self.free(module);
        let id = single_string(call).filter(|_| fits)?;
        self.paths
            .get(id)
            .map(|path| self.paths.specifier(self.from, path))
    }
}

impl<'a> VisitMut<'a> for Rewrite<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        let Some(specifier) = self.required(it).or_else(|| self.rooted(it)) else {
            walk_mut::walk_call_expression(self, it);
            return;
        };
        let span = it.callee.span();
        it.callee = Expression::new_identifier(
            span,
            Ident::from_str_in(REQUIRE, &self.builder),
            &self.builder,
        );
        if let Some(argument) = it.arguments.first_mut() {
            let span = argument.span();
            *argument = Argument::from(Expression::new_string_literal(
                span,
                Str::from_str_in(&specifier, &self.builder),
                None,
                &self.builder,
            ));
        }
    }
}

pub(super) fn loaded_call<'a, 'k>(
    program: &'k mut Program<'a>,
) -> Option<&'k mut CallExpression<'a>> {
    let [Statement::ExpressionStatement(statement)] = program.body.as_mut_slice() else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &mut statement.expression else {
        return None;
    };
    let Expression::CallExpression(then) = assign.right.without_parentheses_mut() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = &mut then.callee else {
        return None;
    };
    let Expression::CallExpression(resolved) = member.object.without_parentheses_mut() else {
        return None;
    };
    Some(resolved)
}

fn rewrite_loader<'a>(
    builder: AstBuilder<'a>,
    program: &mut Program<'a>,
    hoisted: Option<&str>,
    paths: &ModulePaths,
    from: &str,
) {
    let found = loader(&program.body, hoisted, "module", None)
        .map(|found| (found.form, found.id.to_string()));
    let Some((form, id)) = found else {
        return;
    };
    let Some(path) = paths.get(&id) else {
        return;
    };
    let Some(resolved) = loaded_call(program) else {
        return;
    };
    if form == Form::Hoisted {
        resolved.arguments.clear();
        return;
    }
    let Some(Argument::CallExpression(load)) = resolved.arguments.first_mut() else {
        return;
    };
    let span = load.callee.span();
    load.callee = Expression::new_identifier(span, Ident::from_str_in(REQUIRE, &builder), &builder);
    if let Some(argument) = load.arguments.first_mut() {
        let span = argument.span();
        let specifier = paths.specifier(from, path);
        *argument = Argument::from(Expression::new_string_literal(
            span,
            Str::from_str_in(&specifier, &builder),
            None,
            &builder,
        ));
    }
}

fn runtime_import<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    names: &[String],
) -> Option<()> {
    if names.is_empty() {
        return Some(());
    }
    let specifier = runtime_specifier(module.path.as_str(), BundleKind::Parcel);
    let text = format!(
        "var {{ {} }} = require({});",
        names.join(", "),
        js_string(&specifier)
    );
    let import = synthetic_statements(alloc, text)?;
    let program = module.program_mut();
    let tail: Vec<Statement<'a>> = program.body.drain(..).collect();
    program.body.extend(import);
    program.body.extend(tail);
    Some(())
}

struct Helpers {
    names: Vec<String>,
    index: FxHashMap<String, usize>,
}

fn used(program: &Program<'_>, runtime: &Helpers, is_entry: bool) -> Vec<String> {
    if is_entry {
        return runtime.names.clone();
    }
    let mut found: Vec<usize> = mentioned(&runtime.index, program.body.iter())
        .into_iter()
        .collect();
    found.sort_unstable();
    found
        .into_iter()
        .map(|index| runtime.names[index].clone())
        .collect()
}

fn link_module<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    (callee, hoisted): (&str, bool),
    paths: &ModulePaths,
    runtime: &Helpers,
) -> Option<()> {
    let from = module.path.as_str().to_string();
    let deps = std::mem::take(&mut module.dependencies);
    let is_entry = module.is_entry;
    let program = module.program_mut();
    let builder = AstBuilder::new(alloc);
    rewrite_loader(builder, program, hoisted.then_some(callee), paths, &from);
    let scoping = (!hoisted && binds_any(program, [callee, "module"].into_iter()))
        .then(|| build_scoping(program).scoping);
    if !callee.is_empty() {
        let mut rewrite = Rewrite {
            builder: AstBuilder::new(alloc),
            scoping: scoping.as_ref(),
            callee,
            hoisted,
            deps: &deps,
            paths,
            from: &from,
        };
        rewrite.visit_program(program);
    }
    let names = used(program, runtime, is_entry);
    module.dependencies = deps;
    runtime_import(alloc, module, &names)
}

pub fn link<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) {
    let paths = ModulePaths::of(bundle);
    let hoisted = bundle
        .runtime
        .as_ref()
        .map(|runtime| runtime.require_name.clone())
        .filter(|name| !name.is_empty());
    let requires: FxHashMap<String, String> = std::mem::take(&mut bundle.fresh_requires)
        .into_iter()
        .collect();
    let runtime_key = runtime_id(BundleKind::Parcel);
    let names = bundle
        .modules
        .get(&runtime_key)
        .map(|module| declared_names(&module.program().body))
        .unwrap_or_default();
    let index = names
        .iter()
        .enumerate()
        .map(|(at, name)| (name.clone(), at))
        .collect();
    let runtime = Helpers { names, index };
    for (id, module) in bundle.modules.iter_mut() {
        if *id == runtime_key {
            continue;
        }
        let callee = match &hoisted {
            Some(name) => (name.as_str(), true),
            None => (requires.get(id).map_or("", String::as_str), false),
        };
        let _ = link_module(alloc, module, callee, &paths, &runtime);
    }
}
