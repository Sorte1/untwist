use std::fmt::Display;

pub const PRELUDE_UNBUILT: &str = "its dependency prelude could not be built";

crate::notes::kind_notes!(NOTES, PRELUDE_UNBUILT);

pub fn prelude_unbuilt(id: impl Display) -> String {
    format!("module {id}: {PRELUDE_UNBUILT}")
}
