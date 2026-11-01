use super::lexer::decode;

pub(super) fn escape_end(source: &[u8], position: usize) -> usize {
    match source.get(position + 1) {
        Some(&b'u') if source.get(position + 2) == Some(&b'{') => {
            code_point_end(source, position + 3)
        }
        Some(&b'u') => hex_run_end(source, position + 2, 4),
        Some(&byte) if byte >= 0x80 => position + 1 + decode(source, position + 1).1,
        Some(_) => position + 2,
        None => position + 1,
    }
}

fn hex_value(byte: Option<&u8>) -> Option<u32> {
    byte.and_then(|&byte| char::from(byte).to_digit(16))
}

fn hex_run_end(source: &[u8], mut position: usize, most: usize) -> usize {
    let stop = position + most;
    while position < stop && hex_value(source.get(position)).is_some() {
        position += 1;
    }
    position
}

fn code_point_end(source: &[u8], mut position: usize) -> usize {
    let mut value = 0u32;
    let mut digits = 0;
    while let Some(digit) = hex_value(source.get(position)) {
        position += 1;
        digits += 1;
        value = (value << 4) | digit;
        if value > 0x0010_ffff {
            return position;
        }
    }
    if digits > 0 && source.get(position) == Some(&b'}') {
        return position + 1;
    }
    position
}

pub(super) fn unescaped(word: &[u8], out: &mut [u8; 12]) -> Option<usize> {
    let mut length = 0;
    let mut position = 0;
    while position < word.len() {
        let (byte, next) = match word.get(position) {
            Some(&b'\\') => (escaped_ascii(word, position)?, escape_end(word, position)),
            Some(&byte) if byte < 0x80 => (byte, position + 1),
            _ => return None,
        };
        *out.get_mut(length)? = byte;
        length += 1;
        position = next;
    }
    Some(length)
}

fn escaped_ascii(word: &[u8], position: usize) -> Option<u8> {
    if word.get(position + 1) != Some(&b'u') {
        return None;
    }
    let (from, to) = if word.get(position + 2) == Some(&b'{') {
        let end = code_point_end(word, position + 3);
        if word.get(end.checked_sub(1)?) != Some(&b'}') {
            return None;
        }
        (position + 3, end - 1)
    } else {
        let end = hex_run_end(word, position + 2, 4);
        if end != position + 6 {
            return None;
        }
        (position + 2, end)
    };
    let text = std::str::from_utf8(word.get(from..to)?).ok()?;
    let value = u32::from_str_radix(text, 16).ok()?;
    u8::try_from(value).ok().filter(u8::is_ascii)
}
