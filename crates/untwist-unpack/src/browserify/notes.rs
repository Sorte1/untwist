use std::fmt::{Debug, Display};

pub const LATER_ENTRIES_NOT_RUN: &str =
    "later entries of the bundle are not run from its entry module";
pub const UNMAPPED_MODULE_ID: &str = "outside its dependency map; left as written";
pub const REQUIRE_LEFT_UNBOUND: &str = "renamed require that the written file does not bind";
pub const NOT_PROVEN: &str = "is not proven to be browserify's output; no bundle is written";
pub const HOST_OBJECT: &str =
    "which browserify does not give its modules and node does; no bundle is written";

crate::notes::kind_notes!(
    NOTES,
    LATER_ENTRIES_NOT_RUN,
    UNMAPPED_MODULE_ID,
    REQUIRE_LEFT_UNBOUND,
    NOT_PROVEN,
    HOST_OBJECT
);

pub fn later_entries_not_run(count: usize) -> String {
    format!("{LATER_ENTRIES_NOT_RUN} ({count} left out)")
}

pub fn unmapped_module_id(id: impl Display, request: impl Debug, count: usize) -> String {
    format!(
        "module {id}: require({request:?}) names a module {UNMAPPED_MODULE_ID} ({count} such calls in the bundle)"
    )
}

pub fn require_left_unbound(id: impl Display, count: usize) -> String {
    format!(
        "module {id}: still calls its {REQUIRE_LEFT_UNBOUND} ({count} such modules in the bundle)"
    )
}

pub fn not_proven(why: &str) -> String {
    format!("the bundle {NOT_PROVEN}: {why}")
}

pub fn host_object(id: impl Display, what: &str) -> String {
    format!("module {id}: it {what}, {HOST_OBJECT}")
}
