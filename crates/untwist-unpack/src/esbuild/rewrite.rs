use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::{Ident, Str};
use rustc_hash::FxHashMap;
use untwist_core::pass::Note;
use untwist_core::semantic::build_scoping;

use crate::esbuild::mentions::{binds_any_or, fresh_name, mentioned};
use crate::model::{Bundle, BundleKind, Module};
use crate::path::relative_path;
use crate::runtime::{declared_names, js_string, runtime_id, runtime_path, synthetic_statements};

const REQUIRE: &str = "require";
const CREATE_REQUIRE: &str = "createRequire";

struct Calls<'a, 'r> {
    builder: AstBuilder<'a>,
    scoping: Option<&'r Scoping>,
    specifiers: &'r FxHashMap<&'r str, String>,
    shim: Option<&'r str>,
    require: Ident<'a>,
}

impl Calls<'_, '_> {
    fn is_free(&self, id: &IdentifierReference<'_>) -> bool {
        let Some(scoping) = self.scoping else {
            return true;
        };
        id.reference_id
            .get()
            .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
    }
}

impl<'a> VisitMut<'a> for Calls<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if let Expression::Identifier(callee) = &mut it.callee {
            if !it.optional && self.is_free(callee) {
                if let Some(specifier) = self.specifiers.get(callee.name.as_str()) {
                    if it.arguments.is_empty() {
                        callee.name = self.require;
                        let span = callee.span();
                        it.arguments
                            .push(Argument::from(Expression::new_string_literal(
                                span,
                                Str::from_str_in(specifier, &self.builder),
                                None,
                                &self.builder,
                            )));
                    }
                } else if self.shim == Some(callee.name.as_str()) && it.arguments.len() == 1 {
                    callee.name = Ident::from_str_in(REQUIRE, &self.builder);
                }
            }
        }
        walk_mut::walk_call_expression(self, it);
    }
}

fn prepend<'a>(program: &mut Program<'a>, at: usize, statements: AVec<'a, Statement<'a>>) {
    let at = at.min(program.body.len());
    let tail: Vec<Statement<'a>> = program.body.drain(at..).collect();
    program.body.extend(statements);
    program.body.extend(tail);
}

fn declares_require(statement: &Statement<'_>, require: &str) -> bool {
    match statement {
        Statement::ImportDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => declaration.declarations.iter().any(
            |declarator| matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.name == require),
        ),
        _ => false,
    }
}

fn after_requires(program: &Program<'_>, require: &str) -> usize {
    program
        .body
        .iter()
        .take_while(|statement| declares_require(statement, require))
        .count()
}

fn binds_require_at_top(program: &Program<'_>) -> bool {
    program.body.iter().any(|statement| match statement {
        Statement::ImportDeclaration(import) => import.specifiers.iter().flatten().any(|specifier| specifier.local().name == REQUIRE),
        Statement::VariableDeclaration(declaration) => declaration.declarations.iter().any(
            |declarator| matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.name == REQUIRE),
        ),
        Statement::FunctionDeclaration(function) => function.id.as_ref().is_some_and(|id| id.name == REQUIRE),
        Statement::ClassDeclaration(class) => class.id.as_ref().is_some_and(|id| id.name == REQUIRE),
        _ => false,
    })
}

fn entry_require<'a>(alloc: &'a Allocator, program: &mut Program<'a>) -> String {
    if !program.source_type.is_module() || binds_require_at_top(program) {
        return REQUIRE.to_string();
    }
    let name = fresh_name(program, REQUIRE);
    let create = fresh_name(program, CREATE_REQUIRE);
    let text = format!(
        "import {{ createRequire as {create} }} from \"module\";\nconst {name} = {create}(import.meta.url);"
    );
    if let Some(prologue) = synthetic_statements(alloc, text) {
        let at = program
            .body
            .iter()
            .take_while(|statement| matches!(statement, Statement::ImportDeclaration(_)))
            .count();
        prepend(program, at, prologue);
    }
    name
}

fn runtime_names(bundle: &Bundle<'_>) -> Vec<String> {
    bundle
        .modules
        .get(&runtime_id(BundleKind::Esbuild))
        .map(|module| declared_names(&module.program().body))
        .unwrap_or_default()
}

fn runtime_import<'a>(
    alloc: &'a Allocator,
    module: &Module<'a>,
    names: &[String],
    require: &str,
) -> Option<AVec<'a, Statement<'a>>> {
    let specifier = relative_path(module.path.as_str(), &runtime_path(BundleKind::Esbuild));
    let text = format!(
        "var {{ {} }} = {require}({});",
        names.join(", "),
        js_string(&specifier)
    );
    synthetic_statements(alloc, text)
}

fn used_runtime(
    program: &Program<'_>,
    (runtime, wanted): (&[String], &FxHashMap<String, usize>),
    is_entry: bool,
    scoping: Option<&Scoping>,
) -> Vec<String> {
    if is_entry || runtime.is_empty() {
        return runtime.to_vec();
    }
    if let Some(scoping) = scoping {
        let free = scoping.root_unresolved_references();
        return runtime
            .iter()
            .filter(|name| free.contains_key(name.as_str()))
            .cloned()
            .collect();
    }
    let mut found: Vec<usize> = mentioned(wanted, program.body.iter()).into_iter().collect();
    found.sort_unstable();
    found
        .into_iter()
        .map(|index| runtime[index].clone())
        .collect()
}

fn rewrite_module<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    paths: &FxHashMap<String, String>,
    shim: Option<&str>,
    (runtime, wanted): (&[String], &FxHashMap<String, usize>),
) -> Option<()> {
    let from = module.path.as_str().to_string();
    let dependencies = std::mem::take(&mut module.dependencies);
    let specifiers: FxHashMap<&str, String> = dependencies
        .iter()
        .filter_map(|(id, name)| Some((name.as_str(), relative_path(&from, paths.get(id)?))))
        .collect();
    let is_entry = module.is_entry;
    let shim = shim.filter(|_| !is_entry);
    let program = module.program_mut();
    let watched = specifiers.keys().copied().chain(shim);
    let rebound = binds_any_or(program, watched, (!is_entry).then_some(wanted));
    let scoping = rebound.then(|| build_scoping(program).scoping);
    let needs_require = !specifiers.is_empty() || !runtime.is_empty();
    let require = if is_entry && needs_require {
        entry_require(alloc, program)
    } else {
        REQUIRE.to_string()
    };
    let mut calls = Calls {
        builder: AstBuilder::new(alloc),
        scoping: scoping.as_ref(),
        specifiers: &specifiers,
        shim,
        require: Ident::from_str_in(&require, &alloc),
    };
    calls.visit_program(program);
    drop(specifiers);
    let used = used_runtime(program, (runtime, wanted), is_entry, scoping.as_ref());
    module.dependencies = dependencies;
    import_runtime(alloc, module, &used, &require)
}

fn import_runtime<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    used: &[String],
    require: &str,
) -> Option<()> {
    if used.is_empty() {
        return Some(());
    }
    let import = runtime_import(alloc, module, used, require)?;
    let is_entry = module.is_entry;
    let program = module.program_mut();
    let at = if is_entry {
        after_requires(program, require)
    } else {
        0
    };
    prepend(program, at, import);
    Some(())
}

pub fn rewrite<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    let paths: FxHashMap<String, String> = bundle
        .modules
        .iter()
        .map(|(id, module)| (id.clone(), module.path.as_str().to_string()))
        .collect();
    let shim = bundle
        .runtime
        .as_ref()
        .map(|runtime| runtime.require_name.clone());
    let runtime = runtime_names(bundle);
    let wanted: FxHashMap<String, usize> = runtime
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect();
    let runtime_key = runtime_id(BundleKind::Esbuild);
    for (id, module) in bundle.modules.iter_mut() {
        if *id != runtime_key {
            let _ = rewrite_module(alloc, module, &paths, shim.as_deref(), (&runtime, &wanted));
        }
    }
    Vec::new()
}
