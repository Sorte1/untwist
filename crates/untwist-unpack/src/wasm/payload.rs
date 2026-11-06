pub const MAX_WASM_BYTES: usize = 32 * 1024 * 1024;
pub const WASM_MAGIC: [u8; 4] = *b"\0asm";
pub const WASM_VERSION: [u8; 4] = [1, 0, 0, 0];
pub const BASE64_MAGIC: &str = "AGFzbQ";
const HEADER_LEN: usize = 8;
pub const MIN_MODULE_BYTES: usize = HEADER_LEN + 2;
const MIN_MODULE_BASE64: usize = (MIN_MODULE_BYTES * 4).div_ceil(3);
const DATA_URI_TYPES: &[&str] = &["application/wasm", "application/octet-stream"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadRefusal {
    TooLarge(usize),
    TooShort,
    BadMagic,
    BadVersion([u8; 4]),
    BadBase64,
    Malformed(String),
}

impl std::fmt::Display for PayloadRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge(len) => write!(
                f,
                "{len} bytes is over the {MAX_WASM_BYTES}-byte wasm size limit"
            ),
            Self::TooShort => write!(f, "shorter than a wasm header"),
            Self::BadMagic => write!(f, "does not start with the \\0asm magic"),
            Self::BadVersion(version) => write!(f, "has wasm version {version:?}, not 1"),
            Self::BadBase64 => write!(f, "is not valid base64"),
            Self::Malformed(reason) => write!(f, "is not a well-formed wasm module ({reason})"),
        }
    }
}

pub fn check_size(len: usize) -> Result<(), PayloadRefusal> {
    match len > MAX_WASM_BYTES {
        true => Err(PayloadRefusal::TooLarge(len)),
        false => Ok(()),
    }
}

pub fn check_header(bytes: &[u8]) -> Result<(), PayloadRefusal> {
    check_size(bytes.len())?;
    if bytes.len() < HEADER_LEN {
        return Err(PayloadRefusal::TooShort);
    }
    if bytes[..4] != WASM_MAGIC {
        return Err(PayloadRefusal::BadMagic);
    }
    let version = [bytes[4], bytes[5], bytes[6], bytes[7]];
    match version == WASM_VERSION {
        true => Ok(()),
        false => Err(PayloadRefusal::BadVersion(version)),
    }
}

pub fn data_uri_base64(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("data:")?;
    let (media, payload) = rest.split_once(',')?;
    let media_type = media.strip_suffix(";base64")?;
    DATA_URI_TYPES
        .iter()
        .any(|known| media_type.eq_ignore_ascii_case(known))
        .then_some(payload)
}

pub fn wasm_base64(text: &str) -> Option<&str> {
    let body = data_uri_base64(text).unwrap_or(text);
    body.starts_with(BASE64_MAGIC).then_some(body)
}

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

pub fn base64_holds_module(body: &str) -> bool {
    body.bytes()
        .take(MIN_MODULE_BASE64)
        .take_while(|byte| sextet(*byte).is_some())
        .count()
        == MIN_MODULE_BASE64
}

pub fn decode_base64(text: &str) -> Result<Vec<u8>, PayloadRefusal> {
    let body = text.trim_end_matches('=');
    if text.len() - body.len() > 2 || body.len() % 4 == 1 {
        return Err(PayloadRefusal::BadBase64);
    }
    check_size(body.len() / 4 * 3 + (body.len() % 4).saturating_sub(1))?;
    let mut out = Vec::with_capacity(body.len() / 4 * 3 + 2);
    for chunk in body.as_bytes().chunks(4) {
        let mut word = 0u32;
        for (index, byte) in chunk.iter().enumerate() {
            word |= sextet(*byte).ok_or(PayloadRefusal::BadBase64)? << (18 - 6 * index);
        }
        let bytes = word.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    Ok(out)
}

pub fn decode_text(text: &str) -> Option<Result<Vec<u8>, PayloadRefusal>> {
    let body = wasm_base64(text)?;
    Some(decode_base64(body).and_then(|bytes| check_header(&bytes).map(|()| bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_every_tail_length() {
        assert_eq!(decode_base64("AGFzbQEAAAA=").unwrap(), b"\0asm\x01\0\0\0");
        assert_eq!(decode_base64("AGFzbQEAAA").unwrap(), b"\0asm\x01\0\0");
        assert_eq!(decode_base64("AGFzbQEA").unwrap(), b"\0asm\x01\0");
        assert_eq!(decode_base64("AGFz").unwrap(), b"\0as");
    }

    #[test]
    fn a_data_uri_of_a_known_type_carries_its_base64() {
        let text = "data:application/octet-stream;base64,AGFzbQEAAAA=";
        assert_eq!(data_uri_base64(text), Some("AGFzbQEAAAA="));
        assert_eq!(data_uri_base64("data:text/plain;base64,AGFzbQ"), None);
        assert_eq!(data_uri_base64("data:application/wasm,AGFzbQ"), None);
    }

    #[test]
    fn base64_holds_a_module_only_past_the_header() {
        assert!(!base64_holds_module("AGFzbQEAAAA="));
        assert!(!base64_holds_module("AGFzbQEAAAAB"));
        assert!(base64_holds_module("AGFzbQEAAAABBw=="));
    }

    #[test]
    fn a_header_with_the_wrong_version_is_refused() {
        let refusal = decode_text("AGFzbQIAAAA=").unwrap().unwrap_err();
        assert_eq!(refusal, PayloadRefusal::BadVersion([2, 0, 0, 0]));
    }

    #[test]
    fn text_that_is_not_base64_is_refused_not_decoded() {
        assert_eq!(
            decode_text("AGFzbQ!!").unwrap().unwrap_err(),
            PayloadRefusal::BadBase64
        );
        assert!(decode_text("hello").is_none());
    }

    #[test]
    fn an_oversize_payload_is_refused_before_it_is_decoded() {
        let text = format!("AGFzbQEA{}", "A".repeat(MAX_WASM_BYTES / 3 * 4 + 8));
        assert!(matches!(
            decode_text(&text).unwrap(),
            Err(PayloadRefusal::TooLarge(_))
        ));
    }
}
