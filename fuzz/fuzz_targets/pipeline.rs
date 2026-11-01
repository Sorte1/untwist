#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    for unsafe_transforms in [false, true] {
        for extended in [false, true] {
            let options = untwist::Options {
                unsafe_transforms,
                extended,
                max_iterations: Some(8),
                ..untwist::Options::default()
            };
            let out = untwist::Pipeline::new(options).run(data);
            assert!(
                out.panicked.is_empty(),
                "the pipeline panicked on a parseable input"
            );
        }
    }
});
