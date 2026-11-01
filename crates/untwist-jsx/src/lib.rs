pub mod automatic;
pub(crate) mod build;
pub mod classic;
mod common;
pub mod notes;
pub(crate) mod pragma;
pub(crate) mod uid;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::pass::{run_to_fixpoint, PassCtor, PassGroup, PassOptions, RunReport};
use untwist_core::semantic::Semantics;

pub const JSX_ORDER: &[&str] = &[classic::PASS, automatic::PASS];

pub fn jsx_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![classic::boxed, automatic::boxed]
}

pub fn run_jsx<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    opts: &PassOptions,
) -> (Semantics, RunReport) {
    let mut group = PassGroup::from_ctors(&jsx_ctors(), opts.clone());
    run_to_fixpoint(alloc, program, sem, &mut group)
}
