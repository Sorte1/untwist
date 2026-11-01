use untwist_core::pass::PassCtor;

use crate::{block_statements, sequence, split_variable_declarations};

pub fn prepare_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        block_statements::boxed,
        sequence::boxed,
        split_variable_declarations::boxed,
    ]
}
