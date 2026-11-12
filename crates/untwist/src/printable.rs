use std::borrow::Cow;
use std::fmt::Write as _;

fn is_unprintable(ch: char) -> bool {
    let invisible = matches!(ch, '\u{61c}' | '\u{200b}'..='\u{200f}' | '\u{feff}');
    let separator_or_direction = matches!(ch, '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2069}');
    ch.is_control() || invisible || separator_or_direction
}

pub fn printable(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_unprintable) {
        return Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        if is_unprintable(ch) {
            let _ = write!(escaped, "\\u{{{:x}}}", u32::from(ch));
        } else {
            escaped.push(ch);
        }
    }
    Cow::Owned(escaped)
}
