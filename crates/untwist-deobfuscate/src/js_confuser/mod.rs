mod ast;
mod base91;
mod cff;
mod dispatcher;
mod flatten;
mod folds;
mod globals;
mod integrity;
mod locks;
mod names;
mod refs;
mod resolution;
mod strings;
mod templates;
mod tokens;

pub use cff::{boxed as cff_boxed, PASS as CFF_PASS};
pub use dispatcher::{boxed as dispatcher_boxed, PASS as DISPATCHER_PASS};
pub use flatten::{boxed as flatten_boxed, PASS as FLATTEN_PASS};
pub use folds::{boxed as helpers_boxed, PASS as HELPERS_PASS};
pub use globals::{boxed as globals_boxed, PASS as GLOBALS_PASS};
pub use integrity::{boxed as integrity_boxed, PASS as INTEGRITY_PASS};
pub use locks::{boxed as locks_boxed, PASS as LOCKS_PASS};
pub use strings::{boxed as strings_boxed, PASS as STRINGS_PASS};

pub fn ctors<'a>() -> Vec<untwist_core::pass::PassCtor<'a>> {
    vec![
        integrity_boxed,
        locks_boxed,
        strings_boxed,
        globals_boxed,
        helpers_boxed,
        dispatcher_boxed,
        flatten_boxed,
        cff_boxed,
    ]
}
