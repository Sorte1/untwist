pub const STRING_ARRAY_NAME: &str = "__STRING_ARRAY__";
pub const DECODE_PREFIX: &str = "__DECODE_";
pub const ARRAY_PREFIX: &str = "__STRING_ARRAY_";
const MACHINERY_LEAD: &[u8] = b"__";

#[inline(always)]
fn numbered(name: &str, prefix: &str) -> Option<usize> {
    name.strip_prefix(prefix)?.strip_suffix("__")?.parse().ok()
}

fn array_number(name: &str) -> Option<usize> {
    if name == STRING_ARRAY_NAME {
        return Some(0);
    }
    numbered(name, ARRAY_PREFIX)
}

fn decoder_number(name: &str) -> Option<usize> {
    numbered(name, DECODE_PREFIX)
}

#[inline]
pub fn string_array_unit(name: &str) -> Option<usize> {
    if !may_name_machinery(name) {
        return None;
    }
    array_number(name)
}

#[inline]
pub fn decoder_index(name: &str) -> Option<usize> {
    if !may_name_machinery(name) {
        return None;
    }
    decoder_number(name)
}

#[inline]
pub fn may_name_machinery(name: &str) -> bool {
    name.as_bytes().starts_with(MACHINERY_LEAD)
}

pub fn is_machinery_name(name: &str) -> bool {
    decoder_index(name).is_some() || string_array_unit(name).is_some()
}
