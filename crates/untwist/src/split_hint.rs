use oxc_allocator::Allocator;

use crate::{Options, Output};

#[derive(Clone, Copy, Debug)]
pub struct SplitHint {
    asked: bool,
    unpack: untwist_unpack::UnpackOptions,
}

impl SplitHint {
    pub fn of(options: &Options) -> Self {
        SplitHint {
            asked: options.unpack && !options.split_assume_modules,
            unpack: untwist_unpack::UnpackOptions {
                split_hoisted: true,
                assume_modules: true,
                conservative: !options.extended(),
                trust_free_loader: options.unsafe_transforms,
                trust_legacy_reads: options.unsafe_transforms,
            },
        }
    }

    pub fn splits_when_assumed(self, output: &Output) -> bool {
        if !self.asked || output.bundle.is_some() {
            return false;
        }
        let [code] = output.codes.as_slice() else {
            return false;
        };
        let alloc = Allocator::default();
        let parsed = untwist_core::parse::parse_program(&alloc, code);
        untwist_unpack::splits_with(&alloc, &parsed.program, self.unpack)
    }
}
