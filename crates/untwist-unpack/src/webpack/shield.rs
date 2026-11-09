use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use rustc_hash::FxHashSet;
use untwist_core::semantic::build_scoping;

use crate::esbuild::mentions::binds_any;
use crate::model::{Module, RequireParam};
use crate::rename::{generate_uid, rename_parameters};

const REQUIRE: &str = "require";

pub struct Shield {
    host: String,
    runtime: Option<String>,
}

fn reads_free(program: &Program<'_>, name: &str) -> bool {
    build_scoping(program)
        .scoping
        .root_unresolved_references()
        .contains_key(name)
}

pub fn shield<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    fresh: Option<&str>,
) -> Option<Shield> {
    let runtime = match module.runtime_require {
        RequireParam::Default => return None,
        RequireParam::Fresh => Some(fresh?.to_string()),
        RequireParam::Absent => None,
    };
    let program = module.program_mut();
    if !reads_free(program, REQUIRE) {
        return None;
    }
    let scoping = build_scoping(program).scoping;
    let host = generate_uid("host_require", &scoping, &FxHashSet::default());
    match runtime.as_deref() {
        Some(name) => rename_parameters(alloc, program, &[REQUIRE, name], &[&host, REQUIRE]),
        None => rename_parameters(alloc, program, &[REQUIRE], &[&host]),
    };
    Some(Shield { host, runtime })
}

struct Restore<'a, 's> {
    alloc: &'a Allocator,
    shield: &'s Shield,
    converted: &'s FxHashSet<u32>,
    scoping: Option<Scoping>,
}

impl<'a> VisitMut<'a> for Restore<'a, '_> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if !crate::scan::is_free(self.scoping.as_ref(), it) {
            return;
        }
        if it.name == self.shield.host.as_str() {
            it.name = Ident::from_str_in(REQUIRE, &self.alloc);
        } else if it.name == REQUIRE && !self.converted.contains(&it.span.start) {
            if let Some(runtime) = &self.shield.runtime {
                it.name = Ident::from_str_in(runtime, &self.alloc);
            }
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        walk_mut::walk_binding_identifier(self, it);
    }
}

pub fn restore<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    shield: &Shield,
    converted: &FxHashSet<u32>,
) {
    let program = module.program();
    let bound = binds_any(program, [REQUIRE, shield.host.as_str()].into_iter());
    let mut restore = Restore {
        alloc,
        shield,
        converted,
        scoping: bound.then(|| build_scoping(program).scoping),
    };
    restore.visit_program(module.program_mut());
}
