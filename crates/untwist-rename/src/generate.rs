use crate::reserved::is_reserved;

const ACCESSOR_PREFIXES: [&str; 8] = [
    "get", "set", "fetch", "load", "create", "make", "build", "read",
];

pub fn sequential_name(index: usize) -> String {
    let mut remaining = index.saturating_add(1);
    let mut bytes = Vec::new();
    while remaining > 0 {
        remaining -= 1;
        bytes.push(b'a' + (remaining % 26) as u8);
        remaining /= 26;
    }
    bytes.reverse();
    String::from_utf8(bytes).unwrap_or_default()
}

pub fn to_identifier(input: &str) -> String {
    let mut masked = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '$' || ch == '_' {
            masked.push(ch);
        } else {
            masked.push('-');
        }
    }
    let trimmed = masked.trim_start_matches(|ch: char| ch == '-' || ch.is_ascii_digit());
    let camel = camel_case(trimmed);
    if camel.is_empty() {
        return "_".to_string();
    }
    if is_reserved(&camel) {
        return format!("_{camel}");
    }
    camel
}

fn camel_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut upper_next = false;
    for ch in input.chars() {
        if ch == '-' {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn is_identifier_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '$' || first == '_') {
        return false;
    }
    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '$' || ch == '_')) {
        return false;
    }
    !is_reserved(name)
}

pub fn title_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut at_boundary = true;
    for ch in input.chars() {
        if at_boundary && ch.is_ascii_lowercase() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push(ch);
        }
        at_boundary = ch.is_whitespace();
    }
    out.retain(|ch| ch.is_ascii_alphanumeric() || ch == '$' || ch == '_');
    out
}

pub fn lower_camel(input: &str) -> String {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out: String = first.to_lowercase().collect();
    out.extend(chars);
    out
}

pub fn strip_accessor_prefix(name: &str) -> Option<String> {
    for prefix in ACCESSOR_PREFIXES {
        let Some(rest) = name.strip_prefix(prefix) else {
            continue;
        };
        if rest.is_empty() || !rest.starts_with(|ch: char| ch.is_ascii_uppercase()) {
            continue;
        }
        return Some(lower_camel(rest));
    }
    None
}

pub fn uid_candidates(base: &str) -> impl Iterator<Item = String> + '_ {
    std::iter::once(base.to_string()).chain((2u32..1000).map(move |index| format!("{base}{index}")))
}

pub const UID_CANDIDATES: usize = 999;

pub fn uid_candidate(base: &str, position: usize) -> Option<String> {
    match position {
        0 => Some(base.to_string()),
        1..UID_CANDIDATES => Some(format!("{base}{}", position + 1)),
        _ => None,
    }
}
