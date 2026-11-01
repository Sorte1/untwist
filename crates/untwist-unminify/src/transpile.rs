use untwist_core::pass::PassCtor;

use crate::{
    default_parameters, logical_assignments, nullish_coalescing, nullish_coalescing_assignment,
    optional_chaining, template_literals,
};

pub fn transpile_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        default_parameters::boxed,
        logical_assignments::boxed,
        nullish_coalescing::boxed,
        nullish_coalescing_assignment::boxed,
        optional_chaining::boxed,
        template_literals::boxed,
    ]
}
