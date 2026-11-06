use std::borrow::Cow;

use oxc_str::JSStr;

const ESCAPE: char = '\u{FFFD}';

pub fn js_text(value: JSStr<'_>) -> Cow<'_, str> {
    match value.as_str() {
        Some(text) => Cow::Borrowed(text),
        None => Cow::Owned(escape_lone_surrogates(value)),
    }
}

fn escape_lone_surrogates(value: JSStr<'_>) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for ch in value.chars() {
        match ch.to_char() {
            Some(ESCAPE) => out.push_str("\u{FFFD}fffd"),
            Some(ch) => out.push(ch),
            None => {
                out.push(ESCAPE);
                out.push_str(&format!("{:04x}", ch.to_u32()));
            }
        }
    }
    out
}
