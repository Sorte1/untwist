const URL_MARKERS: [&str; 2] = ["//# sourceMappingURL=", "//@ sourceMappingURL="];
const DATA_PREFIX: &str = "data:application/json";
const BASE64_MARKER: &str = ";base64,";

fn sextet(byte: u8) -> Option<u32> {
    match byte {
        b'A'..=b'Z' => Some(u32::from(byte - b'A')),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(byte - b'0') + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

pub fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for byte in text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=')
    {
        buffer = (buffer << 6) | sextet(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xFF).ok()?);
        }
    }
    Some(out)
}

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(value)) => {
                out.push(value);
                index += 3;
            }
            (byte, _) => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn last_map_url(source: &str) -> Option<&str> {
    let (at, marker) = URL_MARKERS
        .iter()
        .filter_map(|marker| source.rfind(marker).map(|at| (at, *marker)))
        .max_by_key(|(at, _)| *at)?;
    let rest = &source[at + marker.len()..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '*')
        .unwrap_or(rest.len());
    Some(&rest[..end]).filter(|url| !url.is_empty())
}

pub fn inline_map(source: &str) -> Option<String> {
    let url = last_map_url(source)?;
    let payload = url.strip_prefix(DATA_PREFIX)?;
    let comma = payload.find(',')?;
    let (head, body) = payload.split_at(comma);
    if format!("{head},").ends_with(BASE64_MARKER) {
        return String::from_utf8(decode_base64(&body[1..])?).ok();
    }
    Some(percent_decoded(&body[1..]))
}
