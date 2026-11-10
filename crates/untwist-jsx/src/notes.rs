use untwist_core::catalogue;

pub const ATTRIBUTE_SHAPE: &str =
    "a prop is a getter, a setter, a method or a key JSX cannot spell, which no JSX attribute \
     can express, so the element was left as a call";
pub const ELEMENT_NAME_SHAPE: &str =
    "the element type is a name JSX cannot spell as a tag, so the element was left as a call";
pub const COMPONENT_LOOKING_STRING: &str =
    "the element type is a string that JSX would read as a component reference, so the element \
     was left as a call";
pub const LOWERCASE_UNBOUND: &str =
    "the element type is a lowercase identifier that resolves to no binding, so renaming it away \
     from the HTML tag it collides with is not possible and the call was left alone";
pub const TYPE_HOISTED: &str =
    "the element type is an expression with no JSX spelling, so it was hoisted into a const \
     declaration before the enclosing statement";
pub const HOIST_ANCHOR_UNRESOLVED: &str =
    "the element type has no JSX spelling and reads a binding that is not visible where its \
     const declaration would land, so the element was left as a call";
pub const KEY_SHAPE: &str =
    "the call passes more than a type, its props and a key, which only the development runtime \
     (jsxDEV/jsxsDEV) emits, and it is not reconstructed, so the call was left alone";
pub const CHILDREN_SHAPE: &str =
    "the children prop is not the last prop, appears more than once or holds an array with holes, \
     which JSX children cannot express in the same order, so the call was left alone";
pub const COMPONENT_NAME_EXHAUSTED: &str =
    "every Component uid up to the suffix budget is already bound in this program, so the \
     element type could not be renamed or hoisted and the call was left alone";

pub const HOISTS_NOT_PLACED: &str =
    "the statement a hoisted element type was keyed to left the program before the declaration \
     could be placed, so";

const REPEATED_SITES: &str = "the same reason held at";

catalogue!(
    ALL_NOTES,
    ATTRIBUTE_SHAPE,
    ELEMENT_NAME_SHAPE,
    COMPONENT_LOOKING_STRING,
    LOWERCASE_UNBOUND,
    TYPE_HOISTED,
    HOIST_ANCHOR_UNRESOLVED,
    KEY_SHAPE,
    CHILDREN_SHAPE,
    COMPONENT_NAME_EXHAUSTED,
);

catalogue!(GUARD_NOTES, HOISTS_NOT_PLACED);

pub fn hoists_not_placed(count: usize) -> String {
    format!("{HOISTS_NOT_PLACED} {count} const declarations were not emitted")
}

pub fn all_notes() -> &'static [(&'static str, &'static str)] {
    ALL_NOTES
}

pub fn spanned(message: &str, detail: &str) -> String {
    format!("{message} ({detail})")
}

pub fn summary(message: &str, total: usize, breakdown: &[(String, usize)]) -> String {
    let detail = breakdown
        .iter()
        .map(|(name, count)| format!("{name} x{count}"))
        .collect::<Vec<String>>()
        .join(", ");
    format!("{message}; {REPEATED_SITES} {total} sites ({detail})")
}
