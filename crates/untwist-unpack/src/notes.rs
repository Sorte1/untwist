use std::fmt::{Debug, Display};

use untwist_core::catalogue;

pub const UNSAFE_PATH_REPLACED: &str = "replaced by";
pub const UNSAFE_OUTPUT_PATH_REPLACED: &str = "unsafe output path";
pub const PATH_COLLISION: &str = "output path collision, writing";
pub const MAPPING_UNSAFE_PATH: &str = "unsafe path, ignored";
pub const MAPPING_UNMATCHED: &str = "no module matched";
pub const MERGE_KIND_MISMATCH: &str = "chunk merge: ignoring a";
pub const LATER_ENTRY_IGNORED: &str = "from a later chunk ignored; keeping";
pub const SECOND_SELF_CONTAINED: &str = "chunk merge: skipped a second self-contained";
pub const ENTRY_CANDIDATES: &str = "top-level require calls and no entry marker is assigned";
pub const REQUIRE_MISSING: &str = "has no module in this bundle";
pub const UNHANDLED_HELPER: &str = "unhandled webpack helper require.";
pub const FACTORY_NOT_CLOSED: &str = "is not closed over its own scope; no bundle is written";
pub const RUNTIME_HELPER_NOT_CLOSED: &str = "cannot move to a runtime module";
pub const SHAPE_REFUSED: &str = "bundle shape found but not proven; no bundle is written";
pub const GROUP_REFUSED: &str = "linked inputs refused as a whole; no bundle is written for";
pub const RUNTIME_NOT_PROVEN: &str = "is not proven to be webpack's runtime; no bundle is written";
pub const REQUIRE_NOT_SHARED: &str =
    "uses its require function in a way the unpacked files cannot share; no bundle is written";
pub const RUNTIMES_DIFFER: &str = "two webpack runtimes differ; no bundle is written";
pub const CHUNK_START_UNPROVEN: &str =
    "runs code on arrival that is not webpack's deferred-entry list; no bundle is written";
pub const DEFERRED_WITH_RUNTIME: &str =
    "starts modules once the runtime loads, which the written files cannot order; no bundle is written";
pub const DEFERRED_NOT_RUN: &str =
    "starts these modules only after webpack's runtime loads other files";
pub const DUPLICATE_DIFFERS: &str =
    "is defined differently by two inputs and the load order is unknown; no bundle is written";

catalogue!(
    ALL_NOTES,
    UNSAFE_PATH_REPLACED,
    UNSAFE_OUTPUT_PATH_REPLACED,
    PATH_COLLISION,
    MAPPING_UNSAFE_PATH,
    MAPPING_UNMATCHED,
    MERGE_KIND_MISMATCH,
    LATER_ENTRY_IGNORED,
    SECOND_SELF_CONTAINED,
    ENTRY_CANDIDATES,
    REQUIRE_MISSING,
    UNHANDLED_HELPER,
    FACTORY_NOT_CLOSED,
    RUNTIME_HELPER_NOT_CLOSED,
    SHAPE_REFUSED,
    GROUP_REFUSED,
    RUNTIME_NOT_PROVEN,
    REQUIRE_NOT_SHARED,
    RUNTIMES_DIFFER,
    CHUNK_START_UNPROVEN,
    DEFERRED_WITH_RUNTIME,
    DEFERRED_NOT_RUN,
    DUPLICATE_DIFFERS,
);

macro_rules! kind_notes {
    ($table:ident $(, $entry:ident)* $(,)?) => {
        pub const $table: &[(&str, &str)] = &[$((stringify!($entry), $entry)),*];
    };
}
pub(crate) use kind_notes;

pub static TABLES: &[&[(&str, &str)]] = &[
    ALL_NOTES,
    crate::browserify::notes::NOTES,
    crate::esbuild::notes::NOTES,
    crate::bun::notes::NOTES,
    crate::rollup::notes::NOTES,
    crate::systemjs::notes::NOTES,
    crate::amd::notes::NOTES,
    crate::umd::notes::NOTES,
    crate::parcel::notes::NOTES,
    crate::metro::notes::NOTES,
    crate::turbopack::notes::NOTES,
    crate::closure::notes::NOTES,
];

pub fn unsafe_path_replaced(id: impl Display, source: impl Debug, path: &str) -> String {
    format!("module {id}: unsafe path {source:?} {UNSAFE_PATH_REPLACED} {path}")
}

pub fn unsafe_output_path_replaced(id: impl Display, path: impl Debug) -> String {
    format!("module {id}: {UNSAFE_OUTPUT_PATH_REPLACED} {path:?} replaced")
}

pub fn path_collision(id: impl Display, path: &str, base: &str) -> String {
    format!("module {id}: {PATH_COLLISION} {path} instead of {base}")
}

pub fn mapping_unsafe_path(path: impl Debug) -> String {
    format!("mapping {path:?}: {MAPPING_UNSAFE_PATH}")
}

pub fn mapping_unmatched(path: impl Debug) -> String {
    format!("mapping {path:?}: {MAPPING_UNMATCHED}")
}

pub fn merge_kind_mismatch(kind: &str, merged: &str) -> String {
    format!("{MERGE_KIND_MISMATCH} {kind} bundle in a {merged} merge")
}

pub fn later_entry_ignored(id: impl Display, kept: impl Display) -> String {
    format!("entry {id} {LATER_ENTRY_IGNORED} {kept}")
}

pub fn second_self_contained(kind: &str) -> String {
    format!("{SECOND_SELF_CONTAINED} {kind} bundle; it has its own entry and runtime")
}


pub fn entry_candidates(id: impl Display, count: usize) -> String {
    format!(
        "entry {id} is the first of {count} {ENTRY_CANDIDATES}; the others are modules the \
         startup body also runs"
    )
}

pub fn require_missing(id: impl Display, target: impl Display) -> String {
    format!("module {id}: require(\"{target}\") {REQUIRE_MISSING}")
}

pub fn unhandled_helper(id: impl Display, helper: impl Display) -> String {
    format!("module {id}: {UNHANDLED_HELPER}{helper}")
}

pub fn factory_not_closed(id: impl Display, why: impl Display) -> String {
    format!("module {id} {FACTORY_NOT_CLOSED}: {why}")
}

pub fn runtime_helper_not_closed(
    kind: impl Display,
    helper: impl Display,
    why: impl Display,
) -> String {
    format!("{kind} helper {helper} {RUNTIME_HELPER_NOT_CLOSED}: {why}")
}

pub fn shape_refused(kind: impl Display) -> String {
    format!("{kind} {SHAPE_REFUSED}")
}

pub fn group_refused(namespace: impl Display) -> String {
    format!("{GROUP_REFUSED} {namespace}")
}

pub fn runtime_not_proven(why: impl Display) -> String {
    format!("the prelude {RUNTIME_NOT_PROVEN}: {why}")
}

pub fn require_not_shared(id: impl Display, why: impl Display) -> String {
    format!("module {id} {REQUIRE_NOT_SHARED}: {why}")
}

pub fn runtimes_differ() -> String {
    format!("chunk merge: {RUNTIMES_DIFFER}; the inputs carry their own helpers")
}

pub fn chunk_start_unproven(ids: impl Display) -> String {
    format!("chunk {ids} {CHUNK_START_UNPROVEN}")
}

pub fn deferred_with_runtime(ids: impl Display) -> String {
    format!("chunk merge: a chunk {DEFERRED_WITH_RUNTIME} ({ids})")
}

pub fn deferred_not_run(ids: impl Display) -> String {
    format!("the bundle {DEFERRED_NOT_RUN}: {ids}; the written modules do not start them")
}

pub fn duplicate_differs(id: impl Display) -> String {
    format!("chunk merge: module {id} {DUPLICATE_DIFFERS}")
}
