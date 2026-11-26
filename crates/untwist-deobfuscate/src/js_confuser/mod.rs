mod base91;
mod folds;
mod globals;
mod locks;
mod names;
mod refs;
mod strings;
mod templates;
mod tokens;

pub use folds::{boxed as helpers_boxed, PASS as HELPERS_PASS};
pub use globals::{boxed as globals_boxed, PASS as GLOBALS_PASS};
pub use locks::{boxed as locks_boxed, PASS as LOCKS_PASS};
pub use strings::{boxed as strings_boxed, PASS as STRINGS_PASS};

pub fn ctors<'a>() -> Vec<untwist_core::pass::PassCtor<'a>> {
    vec![locks_boxed, strings_boxed, globals_boxed, helpers_boxed]
}
