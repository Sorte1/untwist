pub const REQUIRES_UNBUILT: &str = "the entry's requires of its modules could not be built";

crate::notes::kind_notes!(NOTES, REQUIRES_UNBUILT);

pub fn requires_unbuilt() -> String {
    format!("closure: {REQUIRES_UNBUILT}")
}
