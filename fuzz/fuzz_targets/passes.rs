#![no_main]

use libfuzzer_sys::fuzz_target;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;

fuzz_target!(|data: &str| {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, data);
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: Some(4),
        fuel: None,
    };
    let ctors = untwist::all_pass_ctors();
    let mut group = PassGroup::from_ctors(&ctors, opts);
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
});
