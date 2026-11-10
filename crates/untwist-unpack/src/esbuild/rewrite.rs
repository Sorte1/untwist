use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::{Ident, Str};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::Note;
use untwist_core::semantic::build_scoping;

use crate::esbuild::live::read_through_runtime;
use crate::esbuild::mentions::{binds_any_or, fresh_name, mentioned};
use crate::model::{Bundle, BundleKind, Module};
use crate::runtime::{
    declared_names, js_string, live_exports, runtime_id, runtime_specifier, synthetic_statements,
};
use crate::specifier::ModulePaths;

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

struct Shared {
    names: Vec<String>,
    wanted: FxHashMap<String, usize>,
    live: FxHashMap<String, usize>,
}

fn indexed(names: &[String]) -> FxHashMap<String, usize> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect()
}

fn shared(bundle: &Bundle<'_>) -> Shared {
    let Some(runtime) = bundle.modules.get(&runtime_id(BundleKind::Esbuild)) else {
        return Shared {
            names: Vec::new(),
            wanted: FxHashMap::default(),
            live: FxHashMap::default(),
        };
    };
    let body = &runtime.program().body;
    let live = live_exports(body);
    let read_live: FxHashSet<&str> = live.iter().map(String::as_str).collect();
    let names: Vec<String> = declared_names(body)
        .into_iter()
        .filter(|name| !read_live.contains(name.as_str()))
        .collect();
    Shared {
        wanted: indexed(&names),
        live: indexed(&live),
        names,
    }
}

fn runtime_import<'a>(
    alloc: &'a Allocator,
    module: &Module<'a>,
    (binding, names): (Option<&str>, &[String]),
    require: &str,
) -> Option<AVec<'a, Statement<'a>>> {
    let specifier = runtime_specifier(module.path.as_str(), BundleKind::Esbuild);
    let specifier = js_string(&specifier);
    let whole = binding.map(|binding| format!("var {binding} = {require}({specifier});\n"));
    let named = (!names.is_empty())
        .then(|| format!("var {{ {} }} = {require}({specifier});", names.join(", ")));
    let text = whole.into_iter().chain(named).collect::<String>();
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
    paths: &ModulePaths,
    shim: Option<&str>,
    shared: &Shared,
) -> Option<()> {
    let from = module.path.as_str().to_string();
    let dependencies = std::mem::take(&mut module.dependencies);
    let specifiers: FxHashMap<&str, String> = dependencies
        .iter()
        .filter_map(|(id, name)| Some((name.as_str(), paths.specifier(&from, paths.get(id)?))))
        .collect();
    let is_entry = module.is_entry;
    let shim = shim.filter(|_| !is_entry);
    let program = module.program_mut();
    let live = shared.live.keys().map(String::as_str);
    let watched = specifiers.keys().copied().chain(shim).chain(live);
    let rebound = binds_any_or(program, watched, (!is_entry).then_some(&shared.wanted));
    let scoping = rebound.then(|| build_scoping(program).scoping);
    let needs_require =
        !specifiers.is_empty() || !shared.names.is_empty() || !shared.live.is_empty();
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
    let names = (shared.names.as_slice(), &shared.wanted);
    let used = used_runtime(program, names, is_entry, scoping.as_ref());
    let binding = read_through_runtime(alloc, program, &shared.live, scoping.as_ref());
    module.dependencies = dependencies;
    import_runtime(alloc, module, (binding.as_deref(), &used), &require)
}

fn import_runtime<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    (binding, used): (Option<&str>, &[String]),
    require: &str,
) -> Option<()> {
    if binding.is_none() && used.is_empty() {
        return Some(());
    }
    let import = runtime_import(alloc, module, (binding, used), require)?;
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
    let paths = ModulePaths::of(bundle);
    let shim = bundle
        .runtime
        .as_ref()
        .map(|runtime| runtime.require_name.clone());
    let shared = shared(bundle);
    let runtime_key = runtime_id(BundleKind::Esbuild);
    for (id, module) in bundle.modules.iter_mut() {
        if *id != runtime_key {
            let _ = rewrite_module(alloc, module, &paths, shim.as_deref(), &shared);
        }
    }
    Vec::new()
}
