#![no_main]

use libfuzzer_sys::fuzz_target;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

fuzz_target!(|data: &str| {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, data);
    let sem = build_scoping(&parsed.program);
    let opts = untwist_core::pass::PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: Some(4),
        fuel: None,
    };
    let (_sem, report) =
        untwist_deobfuscate::run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    assert!(
        report.iterations <= untwist_deobfuscate::DEOBFUSCATE_MAX_ROUNDS + 1,
        "the deobfuscate driver ran past its round cap"
    );
});
