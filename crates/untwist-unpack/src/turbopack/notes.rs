pub const BROWSER_PREMISE: &str =
    "browser chunk read as if Turbopack's standard runtime loads it; the chunk alone runs no module";

crate::notes::kind_notes!(NOTES, BROWSER_PREMISE);

pub fn browser_premise() -> String {
    format!("turbopack: {BROWSER_PREMISE}")
}
