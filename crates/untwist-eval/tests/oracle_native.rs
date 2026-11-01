mod common;

use untwist_eval::decode::{Decoded, Evaluator, InterpEvaluator};

const SWAPPED: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";

fn b64_decode_swapped(text: &str) -> Vec<u16> {
    let alphabet: Vec<char> = SWAPPED.chars().collect();
    let mut accumulator: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch == '=' {
            continue;
        }
        let Some(index) = alphabet.iter().position(|candidate| *candidate == ch) else {
            continue;
        };
        accumulator = (accumulator << 6) | index as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((accumulator >> bits) & 0xFF) as u16);
        }
    }
    out
}

fn b64_encode_swapped(bytes: &[u8]) -> String {
    let alphabet: Vec<char> = SWAPPED.chars().collect();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map(u32::from).unwrap_or(0);
        let b2 = chunk.get(2).copied().map(u32::from).unwrap_or(0);
        let packed = (b0 << 16) | (b1 << 8) | b2;
        out.push(alphabet[(packed >> 18) as usize & 63]);
        out.push(alphabet[(packed >> 12) as usize & 63]);
        if chunk.len() > 1 {
            out.push(alphabet[(packed >> 6) as usize & 63]);
        }
        if chunk.len() > 2 {
            out.push(alphabet[packed as usize & 63]);
        }
    }
    out
}

fn utf8_decode(units: &[u16]) -> Option<Vec<u16>> {
    let bytes: Vec<u8> = units.iter().map(|unit| *unit as u8).collect();
    let text = String::from_utf8(bytes).ok()?;
    Some(text.encode_utf16().collect())
}

fn rc4(units: &[u16], key: &str) -> Vec<u16> {
    let key_units: Vec<u16> = key.encode_utf16().collect();
    let mut state: Vec<u16> = (0..256u16).collect();
    let mut j: usize = 0;
    for i in 0..256usize {
        j = (j + state[i] as usize + key_units[i % key_units.len()] as usize) % 256;
        state.swap(i, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    let mut out = Vec::with_capacity(units.len());
    for unit in units {
        i = (i + 1) % 256;
        j = (j + state[i] as usize) % 256;
        state.swap(i, j);
        out.push(unit ^ state[(state[i] as usize + state[j] as usize) % 256]);
    }
    out
}

fn encode_base64(plain: &str) -> String {
    b64_encode_swapped(plain.as_bytes())
}

fn encode_rc4(plain: &str, key: &str) -> String {
    let units: Vec<u16> = plain.encode_utf16().collect();
    let ciphered = rc4(&units, key);
    let text = String::from_utf16_lossy(&ciphered);
    b64_encode_swapped(text.as_bytes())
}

#[test]
fn the_oracle_round_trips_its_own_encodings() {
    let plain = "Hello World!";
    let decoded = utf8_decode(&b64_decode_swapped(&encode_base64(plain))).expect("valid utf8");
    assert_eq!(String::from_utf16_lossy(&decoded), plain);
    let ciphered = b64_decode_swapped(&encode_rc4(plain, "DiKO"));
    let recovered = rc4(&utf8_decode(&ciphered).expect("valid utf8"), "DiKO");
    assert_eq!(String::from_utf16_lossy(&recovered), plain);
}

fn base64_program(entries: &[String]) -> String {
    let items = entries
        .iter()
        .map(|entry| format!("'{entry}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "function a() {{ var h = [{items}]; a = function () {{ return h; }}; return a(); }}
function b(c, d) {{
  var e = a();
  return (b = function (f, g) {{
    f = f - 0x0;
    var i = e[f];
    if (b['init'] === undefined) {{
      var atobPoly = function (input) {{
        var chars = '{SWAPPED}';
        var output = '', temp = '';
        for (var bc = 0, bs, buffer, idx = 0; (buffer = input['charAt'](idx++)); ~buffer && ((bs = bc % 4 ? bs * 64 + buffer : buffer), bc++ % 4) ? (output += String['fromCharCode'](255 & (bs >> ((-2 * bc) & 6)))) : 0) {{
          buffer = chars['indexOf'](buffer);
        }}
        for (var k = 0, len = output['length']; k < len; k++) {{
          temp += '%' + ('00' + output['charCodeAt'](k)['toString'](16))['slice'](-2);
        }}
        return decodeURIComponent(temp);
      }};
      b['decoder'] = atobPoly;
      b['cache'] = {{}};
      b['init'] = true;
    }}
    var first = e[0x0], key = f + first, hit = b['cache'][key];
    return !hit ? ((i = b['decoder'](i)), (b['cache'][key] = i)) : (i = hit), i;
  }}), b(c, d);
}}
b(0x0);
b(0x1);
b(0x0);"
    )
}

#[test]
fn the_interpreter_agrees_with_the_native_base64_oracle() {
    let plaintexts = ["Hello World!", "log"];
    let entries: Vec<String> = plaintexts.iter().map(|p| encode_base64(p)).collect();
    let source = base64_program(&entries);
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::default();
        let values = evaluator.decode(&decoder_program, &calls).expect("decode");
        let rendered: Vec<String> = values
            .iter()
            .map(|value| match value {
                Decoded::Str(text) => text.to_lossy_string(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(rendered, vec!["Hello World!", "log", "Hello World!"]);
    });
}

#[test]
fn multi_byte_utf8_decoder_corpus_matches_the_oracle() {
    let plaintexts = ["héllo wörld", "a😀b"];
    let entries: Vec<String> = plaintexts.iter().map(|p| encode_base64(p)).collect();
    let source = base64_program(&entries);
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::default();
        let values = evaluator.decode(&decoder_program, &calls).expect("decode");
        for (index, entry) in entries.iter().enumerate() {
            let expected = utf8_decode(&b64_decode_swapped(entry)).expect("valid utf8");
            let Decoded::Str(actual) = &values[index] else {
                panic!("expected a string at index {index}");
            };
            assert_eq!(
                actual.to_lossy_string(),
                String::from_utf16_lossy(&expected)
            );
        }
    });
}

#[test]
fn malformed_utf8_is_rejected_the_way_the_rotator_expects() {
    assert!(utf8_decode(&[0xFF, 0xFE]).is_none());
}
