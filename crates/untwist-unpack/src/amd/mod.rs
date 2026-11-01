pub mod define;
pub mod factory;
pub mod notes;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::Note;
use untwist_core::semantic::build_scoping;

use crate::amd::define::{acyclic_and_resolved, defines, identified, starts_with_define, Define};
use crate::amd::factory::{
    guard_free, module_program, prepend_dependencies, sound, Returns, EXPORTS,
};
use crate::frame::Frame;
use crate::model::{Bodies, Bundle, BundleKind, Module};

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    starts_with_define(frame.top)
}

fn proven(source: &Program<'_>, modules: &[(String, Define<'_, '_>)]) -> bool {
    let scoping = build_scoping(source).scoping;
    guard_free(source)
        && modules
            .iter()
            .all(|(_, define)| sound(&scoping, &define.callback, &define.pairs, true))
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    if !starts_with_define(frame.top) {
        return None;
    }
    let modules = identified(defines(frame.top)?)?;
    if !acyclic_and_resolved(&modules) {
        return None;
    }
    if !matches!(bodies, Bodies::Taken(_)) && !proven(source, &modules) {
        return None;
    }
    let entry_id = modules.last().map(|(id, _)| id.clone());
    let mut bundle = Bundle::new(BundleKind::Amd, entry_id.clone());
    for (id, define) in modules {
        let is_entry = entry_id.as_deref() == Some(id.as_str());
        let exports = define
            .pairs
            .iter()
            .any(|(_, dependency)| dependency == EXPORTS);
        let returns = if is_entry || exports {
            Returns::Guarded
        } else {
            Returns::Assigned
        };
        let program = module_program(alloc, source, &define.callback, returns, bodies)?;
        let mut module = Module::new(id.clone(), program, is_entry);
        module.dependencies = define.pairs;
        bundle.modules.insert(id, module);
    }
    Some(bundle)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    prepend_dependencies(alloc, bundle)
}
