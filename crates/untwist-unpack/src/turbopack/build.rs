use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{SourceType, Span};

use super::layout::{Def, Id, Layout};
use super::links::{loader, runtime_path, Group, NAMESPACE};
use super::{ENTRY_ID, NAMES};
use crate::esbuild::build::has_use_strict;
use crate::matcher::param_names;
use crate::model::{module_from_body_with, Bodies, Bundle, BundleKind, Module};
use crate::path::{relative_path, PosixPath};
use crate::rename::rename_factory_parameters;
use crate::runtime::{js_string, synthetic_statements};

pub const CONTEXT: &str = "__turbopack_context__";

fn use_strict<'a>(alloc: &'a Allocator) -> Option<Directive<'a>> {
    let parsed = untwist_core::parse::parse_program(alloc, alloc.alloc_str("\"use strict\";"));
    parsed.program.directives.into_iter().next()
}

fn factory_module<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    def: &Def<'_, 'a>,
    bodies: &mut Bodies<'a>,
    strict: bool,
) -> (Program<'a>, Option<String>) {
    let mut program = module_from_body_with(alloc, source, def.body, bodies);
    let params = param_names(def.params);
    let context = params.first().map(|name| name.to_string());
    if matches!(bodies, Bodies::Planned(_)) {
        return (program, context);
    }
    program.directives = def.body.directives.clone_in(alloc);
    if strict && !has_use_strict(&program.directives) {
        program.directives.extend(use_strict(alloc));
    }
    if let Some(rest) = params.get(1..).filter(|rest| !rest.is_empty()) {
        let to = &NAMES[..rest.len()];
        if rest != to {
            rename_factory_parameters(alloc, &mut program, (rest, to), false);
        }
    }
    (program, context)
}

fn literal(id: &Id) -> String {
    if id.number {
        id.key.clone()
    } else {
        js_string(&id.key)
    }
}

fn synthetic_entry<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    entries: &[Id],
) -> Option<Module<'a>> {
    let text: String = entries
        .iter()
        .map(|id| format!("{CONTEXT}.r({});\n", literal(id)))
        .collect();
    entry_module(alloc, source.source_text, text)
}

fn entry_module<'a>(
    alloc: &'a Allocator,
    source_text: &'a str,
    text: String,
) -> Option<Module<'a>> {
    let builder = AstBuilder::new(alloc);
    let program = Program::new(
        Span::default(),
        SourceType::cjs(),
        source_text,
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        synthetic_statements(alloc, text)?,
        &builder,
    );
    Some(Module::new(ENTRY_ID.to_string(), program, true))
}

pub fn bundle<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Layout<'_, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let direct = match layout.entries.as_slice() {
        [only] => Some(only.key.clone()),
        _ => None,
    };
    let entry_id = match layout.entries.len() {
        0 => None,
        1 => direct.clone(),
        _ => Some(ENTRY_ID.to_string()),
    };
    let mut bundle = Bundle::new(BundleKind::Turbopack, entry_id);
    for def in &layout.defs {
        let (program, context) = factory_module(alloc, source, def, bodies, layout.strict);
        let key = def.id.key.clone();
        let is_entry = direct.as_deref() == Some(key.as_str());
        if let Some(context) = context {
            bundle.fresh_requires.push((key.clone(), context));
        }
        bundle
            .modules
            .insert(key.clone(), Module::new(key, program, is_entry));
    }
    if layout.entries.len() > 1 && !matches!(bodies, Bodies::Planned(_)) {
        bundle.modules.insert(
            ENTRY_ID.to_string(),
            synthetic_entry(alloc, source, &layout.entries)?,
        );
        bundle
            .fresh_requires
            .push((ENTRY_ID.to_string(), CONTEXT.to_string()));
    }
    Some(bundle)
}

fn moved_program<'a>(alloc: &'a Allocator, program: &mut Program<'a>) -> Program<'a> {
    let builder = AstBuilder::new(alloc);
    let comments = std::mem::replace(&mut program.comments, AVec::new_in(&builder));
    let directives = std::mem::replace(&mut program.directives, AVec::new_in(&builder));
    let body = std::mem::replace(&mut program.body, AVec::new_in(&builder));
    Program::new(
        program.span,
        program.source_type,
        program.source_text,
        comments,
        program.hashbang.take(),
        directives,
        body,
        &builder,
    )
}

pub fn loader_bundle<'a>(
    alloc: &'a Allocator,
    programs: &mut [Program<'a>],
    group: &Group,
) -> Option<Bundle<'a>> {
    let found = loader(programs.get(group.loader)?)?;
    let path = runtime_path(&found)?;
    let runtime = programs.get_mut(group.runtime)?;
    let entry_path = format!("{ENTRY_ID}.js");
    let text = format!(
        "require({})({});\n{CONTEXT}.r({});\n",
        js_string(&relative_path(&entry_path, &path)),
        js_string(&found.name),
        literal(&found.entry)
    );
    let entry = entry_module(alloc, runtime.source_text, text)?;
    let mut module = Module::new(path.clone(), moved_program(alloc, runtime), false);
    module.path = PosixPath::parse(&path)?;
    module.unsafe_path_source = None;
    let mut bundle = Bundle::new(BundleKind::Turbopack, Some(ENTRY_ID.to_string()));
    bundle.modules.insert(path.clone(), module);
    bundle.modules.insert(ENTRY_ID.to_string(), entry);
    bundle
        .fresh_requires
        .push((ENTRY_ID.to_string(), CONTEXT.to_string()));
    bundle.link = Some(NAMESPACE.to_string());
    bundle.fixed_paths.push(path);
    Some(bundle)
}
