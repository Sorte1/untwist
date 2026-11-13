use oxc_allocator::Allocator;
use untwist_core::pass::{Note, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_unminify::{lexical_ctors, run_last_traversal};
use untwist_unpack::{Bundle, BundleKind};

use super::Pipeline;

impl Pipeline {
    fn relexes(&self, bundle: &Bundle<'_>) -> bool {
        self.options.split_assume_modules
            && self.options.unminify
            && bundle.kind == BundleKind::Rollup
            && bundle.modules.len() > 1
    }

    pub(super) fn relex_modules<'a>(
        &self,
        alloc: &'a Allocator,
        bundle: &mut Bundle<'a>,
        notes: &mut Vec<Note>,
    ) {
        if !self.relexes(bundle) {
            return;
        }
        let opts = PassOptions {
            keep_captured_vars: false,
            ..self.pass_options()
        };
        for module in bundle.modules.values_mut() {
            let program = module.program_mut();
            let sem = build_scoping(program);
            let mut group = PassGroup::from_ctors(&lexical_ctors(), opts.clone());
            let (_kept, report) = run_last_traversal(alloc, program, sem, &mut group, false);
            notes.extend(report.notes);
        }
    }
}
