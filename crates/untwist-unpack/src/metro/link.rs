use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitMut;
use oxc_semantic::Scoping;
use oxc_str::Ident;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Note, NoteKind};
use untwist_core::semantic::build_scoping;

use super::edit::{Edit, Used};
use super::{
    DEPENDENCY_MAP_NAME, GLOBAL_NAME, IMPORT_ALL_NAME, IMPORT_DEFAULT_NAME, IMPORT_PREFIX,
};
use crate::model::{Bundle, BundleKind, Module};
use crate::path::relative_path;
use crate::rename::generate_uid;
use crate::runtime::{js_string, module_of, runtime_path, synthetic_statements};
use crate::webpack::runtime_module::prepend;
use crate::PASS_NAME;

const HELPERS: &str = r#"var metroHasOwn = Object.prototype.hasOwnProperty;
var metroEmpty = {};
var metroImports = new Map();
function metroRequire(moduleId) {
  return require(metroModulePaths[moduleId]);
}
function metroRecord(moduleId) {
  var record = metroImports.get(moduleId);
  if (record === undefined) {
    record = { importedDefault: metroEmpty, importedAll: metroEmpty };
    metroImports.set(moduleId, record);
  }
  return record;
}
function metroImportDefault(moduleId) {
  var record = metroRecord(moduleId);
  if (record.importedDefault !== metroEmpty) {
    return record.importedDefault;
  }
  var value = metroRequire(moduleId);
  return record.importedDefault = value && value.__esModule ? value.default : value;
}
function metroImportAll(moduleId) {
  var record = metroRecord(moduleId);
  if (record.importedAll !== metroEmpty) {
    return record.importedAll;
  }
  var value = metroRequire(moduleId);
  var importedAll;
  if (value && value.__esModule) {
    importedAll = value;
  } else {
    importedAll = {};
    if (value) {
      for (var key in value) {
        if (metroHasOwn.call(value, key)) {
          importedAll[key] = value[key];
        }
      }
    }
    importedAll.default = value;
  }
  return record.importedAll = importedAll;
}
metroRequire.importDefault = metroImportDefault;
metroRequire.importAll = metroImportAll;
"#;

fn prologue_text(module: &Module<'_>, used: &Used) -> String {
    let mut text = String::new();
    if used.global {
        text.push_str(&format!("var {GLOBAL_NAME} = globalThis;\n"));
    }
    if used.dependency_map {
        let elements: Vec<&str> = module
            .dependencies
            .iter()
            .map(|(t, _)| t.as_str())
            .collect();
        text.push_str(&format!(
            "var {DEPENDENCY_MAP_NAME} = [{}];",
            elements.join(", ")
        ));
    }
    text
}

type Context<'r> = (
    &'r FxHashMap<String, String>,
    &'r [String],
    &'r [(String, String)],
);

const SHADOWABLE: [&str; 2] = [GLOBAL_NAME, IMPORT_PREFIX];

fn scoped_fresh(scoping: &Scoping) -> String {
    let taken: FxHashSet<String> = scoping
        .root_unresolved_references()
        .keys()
        .map(|name| name.to_string())
        .collect();
    generate_uid("require", scoping, &taken)
}

fn body_text<'t>(module: &Module<'t>) -> &'t str {
    let program = module.program();
    let span = program.span;
    program
        .source_text
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}

fn edit_with<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    (paths, prelude, deps): Context<'_>,
    (scoping, checking, fresh): (Option<&Scoping>, bool, Option<&str>),
) -> (Used, Vec<String>, bool) {
    let from = module.path.as_str().to_string();
    let mut edit = Edit {
        builder: AstBuilder::new(alloc),
        scoping,
        checking,
        shadowed: false,
        escaped: false,
        deps,
        paths,
        from: &from,
        prelude,
        fresh: fresh.map(|name| Ident::from_str_in(name, &alloc)),
        text: body_text(module),
        used: Used::default(),
        missing: Vec::new(),
    };
    edit.visit_program(module.program_mut());
    (edit.used, edit.missing, edit.shadowed || edit.escaped)
}

fn edited<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    context: Context<'_>,
) -> (Used, Vec<String>) {
    let prelude = context.1;
    let text = body_text(module);
    let unshadowable = !SHADOWABLE.iter().any(|name| text.contains(name))
        && !prelude.iter().any(|name| text.contains(name.as_str()));
    if unshadowable {
        let (used, missing, _) = edit_with(alloc, module, context, (None, false, None));
        return (used, missing);
    }
    let (_, _, scoped) = edit_with(alloc, module, context, (None, true, Some("require")));
    let scoping = scoped.then(|| build_scoping(module.program()).scoping);
    let fresh = scoping
        .as_ref()
        .map_or_else(|| String::from("require"), scoped_fresh);
    let (used, missing, _) = edit_with(
        alloc,
        module,
        context,
        (scoping.as_ref(), false, Some(&fresh)),
    );
    (used, missing)
}

fn rewrite<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    (paths, prelude): (&FxHashMap<String, String>, &[String]),
    notes: &mut Vec<Note>,
) -> Used {
    let deps = std::mem::take(&mut module.dependencies);
    let (used, missing) = edited(alloc, module, (paths, prelude, deps.as_slice()));
    module.dependencies = deps;
    notes.extend(missing.iter().map(|target| Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message: crate::notes::require_missing(&module.id, target),
        span: None,
    }));
    let text = prologue_text(module, &used);
    if let Some(statements) = (!text.is_empty())
        .then(|| synthetic_statements(alloc, text))
        .flatten()
    {
        prepend(module.program_mut(), statements);
    }
    used
}

fn declarator_names(statements: &[Statement<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                names.push(id.name.to_string());
            }
        }
    }
    names
}

fn paths_text(bundle: &Bundle<'_>) -> String {
    let from = runtime_path(BundleKind::Metro);
    let entries: Vec<String> = bundle
        .modules
        .iter()
        .map(|(id, module)| {
            let path = relative_path(&from, module.path.as_str());
            format!("{}: {}", js_string(id), js_string(&path))
        })
        .collect();
    format!("var metroModulePaths = {{ {} }};\n", entries.join(", "))
}

fn bindings(used: &Used, prelude: &[String]) -> Vec<String> {
    let mut list = Vec::new();
    if let Some(fresh) = &used.require {
        list.push(format!("metroRequire: {fresh}"));
    }
    if used.import_default {
        list.push(format!("metroImportDefault: {IMPORT_DEFAULT_NAME}"));
    }
    if used.import_all {
        list.push(format!("metroImportAll: {IMPORT_ALL_NAME}"));
    }
    list.extend(
        prelude
            .iter()
            .filter(|n| used.prelude.contains(*n))
            .cloned(),
    );
    list
}

fn runtime_exports(helpers: bool, prelude: &[String], used: &[Used]) -> String {
    let mut names: Vec<String> = Vec::new();
    if helpers {
        names.extend(["metroRequire", "metroImportDefault", "metroImportAll"].map(String::from));
    }
    names.extend(
        prelude
            .iter()
            .filter(|name| used.iter().any(|u| u.prelude.contains(*name)))
            .cloned(),
    );
    format!("module.exports = {{ {} }};", names.join(", "))
}

fn runtime<'a>(
    alloc: &'a Allocator,
    bundle: &Bundle<'a>,
    mut statements: AVec<'a, Statement<'a>>,
    (prelude, used): (&[String], &[Used]),
) -> Option<Module<'a>> {
    let helpers = used.iter().any(Used::helpers);
    let mut text = String::new();
    if helpers {
        text.push_str(&paths_text(bundle));
        text.push_str(HELPERS);
    }
    text.push_str(&runtime_exports(helpers, prelude, used));
    statements.extend(synthetic_statements(alloc, text)?);
    let source_text = bundle.modules.values().next()?.program().source_text;
    module_of(alloc, source_text, BundleKind::Metro, statements)
}

fn import_runtime<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    names: &[String],
) -> Option<()> {
    let specifier = js_string(&relative_path(
        module.path.as_str(),
        &runtime_path(BundleKind::Metro),
    ));
    let text = match names.is_empty() {
        true => format!("require({specifier});"),
        false => format!("var {{ {} }} = require({specifier});", names.join(", ")),
    };
    prepend(module.program_mut(), synthetic_statements(alloc, text)?);
    Some(())
}

pub fn link<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    let mut statements = AVec::new_in(&alloc);
    if let Some(runtime) = bundle.runtime.take() {
        statements.extend(runtime.helpers.into_iter().map(|helper| helper.statement));
    }
    let prelude = declarator_names(&statements);
    let paths: FxHashMap<String, String> = bundle
        .modules
        .iter()
        .map(|(id, module)| (id.clone(), module.path.as_str().to_string()))
        .collect();
    let mut notes = Vec::new();
    let used: Vec<Used> = bundle
        .modules
        .values_mut()
        .map(|module| rewrite(alloc, module, (&paths, &prelude), &mut notes))
        .collect();
    if statements.is_empty() && !used.iter().any(|u| u.helpers() || !u.prelude.is_empty()) {
        return notes;
    }
    let Some(runtime) = runtime(alloc, bundle, statements, (&prelude, &used)) else {
        return notes;
    };
    for (module, used) in bundle.modules.values_mut().zip(&used) {
        let names = bindings(used, &prelude);
        if !names.is_empty() || module.is_entry {
            import_runtime(alloc, module, &names);
        }
    }
    bundle.modules.insert(runtime.id.clone(), runtime);
    notes
}
