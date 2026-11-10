pub mod notes;
pub mod wrapper;

use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, Expression, IdentifierReference, Program};
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use untwist_core::pass::Note;
use untwist_core::semantic::build_scoping;

use crate::amd::factory::{module_program, prepend_dependencies, sound, Callback};
use crate::esbuild::check::bare_specifier;
use crate::frame::Frame;
use crate::model::{Bodies, Bundle, BundleKind, Module};
use crate::scan::{Frame as Depth, Policy, Scan, Step};
use crate::umd::wrapper::{parts, Umd};

const MODULE_ID: &str = "umd";
const HOST_NAMES: [&str; 5] = ["require", "module", "exports", "__filename", "__dirname"];

struct Hosts<'s> {
    scoping: &'s Scoping,
    read: bool,
}

impl Hosts<'_> {
    fn free(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .is_none_or(|reference| self.scoping.get_reference(reference).symbol_id().is_none())
    }
}

impl<'a> Policy<'a> for Hosts<'_> {
    fn done(&self) -> bool {
        self.read
    }

    fn call(&mut self, _frame: Depth, it: &CallExpression<'a>) -> Step {
        let host_require = matches!(&it.callee, Expression::Identifier(callee)
            if callee.name == "require" && self.free(callee));
        match host_require && !it.optional && bare_specifier(it) {
            true => Step::Skip,
            false => Step::Walk,
        }
    }

    fn reference(&mut self, _frame: Depth, it: &IdentifierReference<'a>) {
        self.read |= HOST_NAMES.contains(&it.name.as_str()) && self.free(it);
    }

    fn with(&mut self) {
        self.read = true;
    }
}

fn reads_host(scoping: &Scoping, callback: &Callback<'_, '_>) -> bool {
    let mut hosts = Hosts {
        scoping,
        read: false,
    };
    let mut scan = Scan::new(&mut hosts);
    scan.visit_formal_parameters(callback.params);
    scan.visit_function_body(callback.body);
    hosts.read
}

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    parts(frame).is_some()
}

fn proven(source: &Program<'_>, umd: &Umd<'_, '_>) -> bool {
    let scoping = build_scoping(source).scoping;
    sound(&scoping, &umd.callback, &umd.pairs, false) && !reads_host(&scoping, &umd.callback)
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let umd = parts(frame)?;
    if !matches!(bodies, Bodies::Taken(_)) && !proven(source, &umd) {
        return None;
    }
    let program = module_program(alloc, source, &umd.callback, umd.returns, bodies)?;
    let mut module = Module::new(MODULE_ID.to_string(), program, true);
    module.dependencies = umd.pairs;
    let mut bundle = Bundle::new(BundleKind::Umd, Some(MODULE_ID.to_string()));
    bundle.modules.insert(MODULE_ID.to_string(), module);
    Some(bundle)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    prepend_dependencies(alloc, bundle)
}
