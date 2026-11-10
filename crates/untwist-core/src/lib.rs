#[macro_export]
macro_rules! catalogue {
    ($table:ident, $($entry:ident),+ $(,)?) => {
        pub const $table: &[(&str, &str)] = &[$((stringify!($entry), $entry)),+];
    };
}

pub mod ast_util;
pub mod budget;
pub mod clock;
pub mod depth;
pub mod guard_shape;
pub mod machinery_name;
pub mod parse;
pub mod pass;
pub mod print;
pub mod reflow;
pub mod semantic;
pub mod type_space;
