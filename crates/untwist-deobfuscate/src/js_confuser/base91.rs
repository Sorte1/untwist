const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];

pub fn slice(text: &[u16], start: f64, length: f64) -> Option<Vec<u16>> {
    let whole = |n: f64| n.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&n);
    if !whole(start) || !whole(length) {
        return None;
    }
    let len = text.len();
    let from = (start as usize).min(len);
    let to = ((start + length) as usize).min(len);
    Some(text.get(from..to.max(from))?.to_vec())
}

pub fn bytes(table: &[u16], raw: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut b: i32 = 0;
    let mut n: i32 = 0;
    let mut v: i32 = -1;
    for unit in raw {
        let Some(p) = table.iter().position(|t| t == unit) else {
            continue;
        };
        let p = p as i32;
        if v < 0 {
            v = p;
            continue;
        }
        v = v.wrapping_add(p.wrapping_mul(91));
        b |= v.wrapping_shl(n as u32);
        n += if (v & 8191) > 88 { 13 } else { 14 };
        loop {
            out.push((b & 255) as u8);
            b >>= 8;
            n -= 8;
            if n <= 7 {
                break;
            }
        }
        v = -1;
    }
    if v > -1 {
        out.push(((b | v.wrapping_shl(n as u32)) & 255) as u8);
    }
    out
}

pub fn utf8(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&BOM) {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn encode(table: &[u16], data: &[u8]) -> Vec<u16> {
        let mut out = Vec::new();
        let (mut b, mut n) = (0u32, 0u32);
        for byte in data {
            b |= u32::from(*byte) << n;
            n += 8;
            if n > 13 {
                let mut v = b & 8191;
                if v > 88 {
                    b >>= 13;
                    n -= 13;
                } else {
                    v = b & 16383;
                    b >>= 14;
                    n -= 14;
                }
                out.push(table[(v % 91) as usize]);
                out.push(table[(v / 91) as usize]);
            }
        }
        if n > 0 {
            out.push(table[(b % 91) as usize]);
            if n > 7 || b > 90 {
                out.push(table[(b / 91) as usize]);
            }
        }
        out
    }

    #[test]
    fn decodes_what_base91_encodes_with_a_shuffled_table() {
        let table = units("Spkl+2yoGxX_dH9YwQZMN4/$V6@Ib7i.h%0Tt!Buazq=8|Un~mC^K>?:3,[\"#Fg<WeP`r}&AO{E]D;c1Lv*jR(J)fs5");
        for text in ["hello, ", "done with ", "ünïcödé ✓ 𝄞", "a", ""] {
            let encoded = encode(&table, text.as_bytes());
            assert_eq!(utf8(&bytes(&table, &encoded)).as_deref(), Some(text));
        }
    }

    #[test]
    fn characters_outside_the_table_are_skipped() {
        let table = units("Spkl+2yoGxX_dH9YwQZMN4/$V6@Ib7i.h%0Tt!Buazq=8|Un~mC^K>?:3,[\"#Fg<WeP`r}&AO{E]D;c1Lv*jR(J)fs5");
        let mut encoded = encode(&table, b"abc");
        encoded.insert(1, u16::from(b' '));
        assert_eq!(utf8(&bytes(&table, &encoded)).as_deref(), Some("abc"));
    }

    #[test]
    fn invalid_utf8_and_a_leading_bom_are_refused() {
        assert_eq!(utf8(&[0xed, 0xa0, 0x80]), None);
        assert_eq!(utf8(&[0xef, 0xbb, 0xbf, 0x41]), None);
        assert_eq!(utf8(&[0xc0, 0x80]), None);
    }

    #[test]
    fn slice_clamps_like_string_prototype_slice() {
        let text = units("abcdef");
        assert_eq!(slice(&text, 2.0, 3.0), Some(units("cde")));
        assert_eq!(slice(&text, 5.0, 9.0), Some(units("f")));
        assert_eq!(slice(&text, 9.0, 1.0), Some(Vec::new()));
        assert_eq!(slice(&text, 1.5, 1.0), None);
    }
}
