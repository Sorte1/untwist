use oxc_ast::ast::{ArrowFunctionExpression, Function, Program};

use super::program_is_strict;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Strictness(bool);

impl Strictness {
    pub fn of_program(program: &Program<'_>) -> Self {
        Self(program_is_strict(program))
    }

    pub fn in_function(self, function: &Function<'_>) -> Self {
        Self(self.0 || function.has_use_strict_directive())
    }

    pub fn in_arrow(self, arrow: &ArrowFunctionExpression<'_>) -> Self {
        Self(self.0 || arrow.has_use_strict_directive())
    }

    pub fn in_class(self) -> Self {
        Self(true)
    }

    pub fn is_strict(self) -> bool {
        self.0
    }
}
