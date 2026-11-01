use crate::value::JsString;

pub fn to_number(text: &str) -> f64 {
    let trimmed = trim_js_whitespace(text);
    if trimmed.is_empty() {
        return 0.0;
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if let Some(rest) = strip_prefix_ci(trimmed, "0x") {
        return parse_radix(rest, 16).flatten().unwrap_or(f64::NAN);
    }
    if let Some(rest) = strip_prefix_ci(trimmed, "0b") {
        return parse_radix(rest, 2).flatten().unwrap_or(f64::NAN);
    }
    if let Some(rest) = strip_prefix_ci(trimmed, "0o") {
        return parse_radix(rest, 8).flatten().unwrap_or(f64::NAN);
    }
    if !is_str_decimal_literal(trimmed) {
        return f64::NAN;
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

pub fn to_int32(n: f64) -> i32 {
    if !n.is_finite() {
        return 0;
    }
    let t = n.trunc();
    let m = t.rem_euclid(4294967296.0);
    (m as u64 as u32) as i32
}

pub fn to_uint32(n: f64) -> u32 {
    if !n.is_finite() {
        return 0;
    }
    let t = n.trunc();
    let m = t.rem_euclid(4294967296.0);
    m as u64 as u32
}

pub fn number_to_string(n: f64) -> String {
    ryu_js::Buffer::new().format(n).to_string()
}

pub fn to_number_units(units: &[u16]) -> f64 {
    match String::from_utf16(units) {
        Ok(text) => to_number(&text),
        Err(_) => f64::NAN,
    }
}

pub fn to_uint16(n: f64) -> u16 {
    to_uint32(n) as u16
}

pub fn to_integer_or_infinity(n: f64) -> f64 {
    if n.is_nan() {
        return 0.0;
    }
    if n.is_infinite() {
        return n;
    }
    n.trunc()
}

pub fn number_to_js_string(n: f64) -> JsString {
    JsString::from_str(&number_to_string(n))
}

pub fn number_to_radix_string(n: f64, radix: u32) -> String {
    if !(2..=36).contains(&radix) || n.is_nan() || n.is_infinite() {
        return number_to_string(n);
    }
    if radix == 10 {
        return number_to_string(n);
    }
    if n == 0.0 {
        return "0".to_string();
    }
    let negative = n < 0.0;
    let magnitude = n.abs();
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&integer_digits(magnitude.trunc(), radix));
    let fraction = magnitude.fract();
    if fraction > 0.0 {
        out.push('.');
        out.push_str(&fraction_digits(fraction, radix));
    }
    out
}

fn integer_digits(mut value: f64, radix: u32) -> String {
    if value < 1.0 {
        return "0".to_string();
    }
    let base = f64::from(radix);
    let mut digits = Vec::new();
    while value >= 1.0 {
        let digit = (value % base) as u32;
        digits.push(digit_char(digit));
        value = (value / base).trunc();
    }
    digits.iter().rev().collect()
}

fn fraction_digits(mut value: f64, radix: u32) -> String {
    let base = f64::from(radix);
    let mut out = String::new();
    for _ in 0..52 {
        if value == 0.0 {
            break;
        }
        value *= base;
        let digit = value.trunc() as u32;
        out.push(digit_char(digit));
        value -= value.trunc();
    }
    out
}

fn digit_char(digit: u32) -> char {
    char::from_digit(digit, 36).unwrap_or('0')
}

pub fn parse_int(units: &[u16], radix: Option<f64>) -> Option<f64> {
    let text: String = units
        .iter()
        .map(|&u| char::from_u32(u32::from(u)).unwrap_or('\u{FFFD}'))
        .collect();
    let mut rest = trim_js_whitespace(&text);
    let sign = match rest.as_bytes().first() {
        Some(b'-') => {
            rest = &rest[1..];
            -1.0
        }
        Some(b'+') => {
            rest = &rest[1..];
            1.0
        }
        _ => 1.0,
    };
    let requested = radix.map(to_int32).unwrap_or(0);
    let mut radix = if requested == 0 { 10 } else { requested };
    if !(2..=36).contains(&radix) {
        return Some(f64::NAN);
    }
    if (requested == 0 || requested == 16) && strip_prefix_ci(rest, "0x").is_some() {
        rest = strip_prefix_ci(rest, "0x").unwrap_or(rest);
        radix = 16;
    }
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_digit(radix as u32))
        .collect();
    if digits.is_empty() {
        return Some(f64::NAN);
    }
    match parse_radix(&digits, radix as u32) {
        Some(Some(value)) => Some(sign * value),
        Some(None) => None,
        None => Some(f64::NAN),
    }
}

pub fn parse_float(units: &[u16]) -> f64 {
    let text: String = units
        .iter()
        .map(|&u| char::from_u32(u32::from(u)).unwrap_or('\u{FFFD}'))
        .collect();
    let rest = trim_js_whitespace(&text);
    for prefix in ["-Infinity", "+Infinity", "Infinity"] {
        if let Some(remainder) = rest.strip_prefix(prefix) {
            let _ = remainder;
            return if prefix.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
    }
    let mut best = f64::NAN;
    for end in (1..=rest.len()).rev() {
        if !rest.is_char_boundary(end) {
            continue;
        }
        let candidate = &rest[..end];
        if !is_str_decimal_literal(candidate) {
            continue;
        }
        if let Ok(value) = candidate.parse::<f64>() {
            best = value;
            break;
        }
    }
    best
}

pub fn is_js_whitespace_char(c: char) -> bool {
    matches!(
        c,
        '\u{9}'
            | '\u{a}'
            | '\u{b}'
            | '\u{c}'
            | '\u{d}'
            | '\u{20}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

pub fn is_js_whitespace_unit(unit: u16) -> bool {
    char::from_u32(u32::from(unit)).is_some_and(is_js_whitespace_char)
}

fn trim_js_whitespace(s: &str) -> &str {
    s.trim_matches(is_js_whitespace_char)
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    if s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes()) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn parse_radix(digits: &str, radix: u32) -> Option<Option<f64>> {
    if digits.is_empty() {
        return None;
    }
    for c in digits.chars() {
        c.to_digit(radix)?;
    }
    if let Some(exact) = accumulate_exact(digits, radix) {
        if exact > MAX_SAFE_INTEGER && radix != 10 && !radix.is_power_of_two() {
            return Some(None);
        }
        return Some(Some(exact as f64));
    }
    if radix == 10 {
        return Some(digits.parse::<f64>().ok());
    }
    if radix.is_power_of_two() {
        return Some(Some(power_of_two_digits(digits, radix.trailing_zeros())));
    }
    Some(None)
}

fn accumulate_exact(digits: &str, radix: u32) -> Option<u128> {
    let radix128 = radix as u128;
    let mut value: u128 = 0;
    for c in digits.chars() {
        let digit = c.to_digit(radix).unwrap_or(0) as u128;
        value = value.checked_mul(radix128)?.checked_add(digit)?;
    }
    Some(value)
}

const SIGNIFICAND_BITS: usize = 53;
const MAX_SAFE_INTEGER: u128 = 1 << SIGNIFICAND_BITS;

fn power_of_two_digits(digits: &str, bits_per_digit: u32) -> f64 {
    let mut bits: Vec<bool> = Vec::with_capacity(digits.len() * bits_per_digit as usize);
    for c in digits.chars() {
        let digit = c.to_digit(1 << bits_per_digit).unwrap_or(0);
        for shift in (0..bits_per_digit).rev() {
            bits.push((digit >> shift) & 1 == 1);
        }
    }
    let Some(first) = bits.iter().position(|bit| *bit) else {
        return 0.0;
    };
    let significant = &bits[first..];
    let mut mantissa: u64 = 0;
    for bit in significant.iter().take(SIGNIFICAND_BITS) {
        mantissa = (mantissa << 1) | u64::from(*bit);
    }
    let dropped = significant.len().saturating_sub(SIGNIFICAND_BITS);
    if dropped > 0 {
        let half = significant[SIGNIFICAND_BITS];
        let sticky = significant[SIGNIFICAND_BITS + 1..].iter().any(|bit| *bit);
        if half && (sticky || mantissa & 1 == 1) {
            mantissa += 1;
        }
    }
    (mantissa as f64) * 2f64.powi(dropped as i32)
}

fn is_str_decimal_literal(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut has_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        has_digit = true;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            has_digit = true;
        }
    }
    if !has_digit {
        return false;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exponent_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exponent_start {
            return false;
        }
    }
    i == bytes.len()
}
