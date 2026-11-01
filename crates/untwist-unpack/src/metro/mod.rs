mod edit;
mod layout;
mod link;
mod normalize;
pub mod notes;
mod verify;

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::Program;
use oxc_ast::builder::AstBuilder;
use oxc_span::SourceType;
use untwist_core::pass::Note;

use crate::frame::Frame;
use crate::model::{module_from_body_with, Bodies, Bundle, BundleKind, Helper, Module, Runtime};
use crate::runtime::{declared_names, js_string, synthetic_statements};

pub const GLOBAL_NAME: &str = "global";
pub const IMPORT_DEFAULT_NAME: &str = "_$$_IMPORT_DEFAULT";
pub const IMPORT_ALL_NAME: &str = "_$$_IMPORT_ALL";
pub const IMPORT_PREFIX: &str = "_$$_IMPORT_";
pub const DEPENDENCY_MAP_NAME: &str = "_dependencyMap";
pub const CANONICAL: [&str; 7] = [
    GLOBAL_NAME,
    "require",
    IMPORT_DEFAULT_NAME,
    IMPORT_ALL_NAME,
    "module",
    "exports",
    DEPENDENCY_MAP_NAME,
];
pub const RUNTIME_NAMES: [&str; 8] = [
    "metroModulePaths",
    "metroHasOwn",
    "metroEmpty",
    "metroImports",
    "metroRequire",
    "metroRecord",
    "metroImportDefault",
    "metroImportAll",
];
const ENTRY_ID: &str = "index";

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    layout::probe(frame.top)
}

fn entry_module<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    runs: &[(String, bool)],
) -> Option<Module<'a>> {
    let text: String = runs
        .iter()
        .map(|(id, numeric)| match numeric {
            true => format!("require({id});\n"),
            false => format!("require({});\n", js_string(id)),
        })
        .collect();
    let builder = AstBuilder::new(alloc);
    let program = Program::new(
        oxc_span::SPAN,
        SourceType::cjs(),
        source.source_text,
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        synthetic_statements(alloc, text)?,
        &builder,
    );
    Some(Module::new(ENTRY_ID.to_string(), program, true))
}

fn prelude_runtime<'a>(alloc: &'a Allocator, source: &Program<'a>, prelude: usize) -> Runtime<'a> {
    let helpers = source.body[..prelude]
        .iter()
        .map(|statement| Helper {
            name: declared_names(std::slice::from_ref(statement)).join(", "),
            statement: statement.clone_in(alloc),
            movable: true,
        })
        .collect();
    Runtime {
        require_name: String::new(),
        helpers,
    }
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let layout = layout::layout(frame.top)?;
    if !matches!(bodies, Bodies::Taken(_)) && !verify::verify(source, &layout) {
        return None;
    }
    let planned = matches!(bodies, Bodies::Planned(_));
    let entry = match layout.runs.as_slice() {
        [(only, _)] => Some(only.clone()),
        [] => None,
        _ => Some(ENTRY_ID.to_string()),
    };
    let mut bundle = Bundle::new(BundleKind::Metro, entry.clone());
    for def in &layout.defs {
        let factory = def.factory()?;
        let mut program = module_from_body_with(alloc, source, factory.body, bodies);
        if !planned {
            program.directives = factory.body.directives.clone_in(alloc);
            normalize::normalize(alloc, &mut program, &def.params());
        }
        let is_entry = entry.as_deref() == Some(def.id.as_str());
        let mut module = Module::new(def.id.clone(), program, is_entry);
        if !planned {
            module.dependencies = normalize::dependencies(def.deps);
        }
        bundle.modules.insert(def.id.clone(), module);
    }
    if layout.runs.len() > 1 {
        if bundle.modules.contains_key(ENTRY_ID) {
            return None;
        }
        let module = entry_module(alloc, source, &layout.runs)?;
        bundle.modules.insert(ENTRY_ID.to_string(), module);
    }
    if !planned {
        bundle.runtime = Some(prelude_runtime(alloc, source, layout.prelude));
    }
    Some(bundle)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    link::link(alloc, bundle)
}
