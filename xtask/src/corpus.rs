use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const SWAPPED: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
const PLAINTEXTS: [&str; 4] = ["log", "Hello World!", "console", "deobfuscate"];
const RC4_KEY: &str = "DiKO";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Encoding {
    None,
    Base64,
    Rc4,
}

impl Encoding {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Base64 => "base64",
            Self::Rc4 => "rc4",
        }
    }
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

fn rc4_units(units: &[u16], key: &str) -> Vec<u16> {
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

fn encode(plain: &str, encoding: Encoding) -> String {
    match encoding {
        Encoding::None => plain.to_string(),
        Encoding::Base64 => b64_encode_swapped(plain.as_bytes()),
        Encoding::Rc4 => {
            let units: Vec<u16> = plain.encode_utf16().collect();
            let ciphered = rc4_units(&units, RC4_KEY);
            b64_encode_swapped(String::from_utf16_lossy(&ciphered).as_bytes())
        }
    }
}

fn atob_body() -> String {
    format!(
        "var atobPoly = function (input) {{\n\
         var chars = '{SWAPPED}';\n\
         var output = '', temp = '';\n\
         for (var bc = 0, bs, buffer, idx = 0; (buffer = input['charAt'](idx++)); ~buffer && ((bs = bc % 4 ? bs * 64 + buffer : buffer), bc++ % 4) ? (output += String['fromCharCode'](255 & (bs >> ((-2 * bc) & 6)))) : 0) {{\n\
         buffer = chars['indexOf'](buffer);\n\
         }}\n\
         for (var k = 0, len = output['length']; k < len; k++) {{\n\
         temp += '%' + ('00' + output['charCodeAt'](k)['toString'](16))['slice'](-2);\n\
         }}\n\
         return decodeURIComponent(temp);\n\
         }};"
    )
}

fn rc4_body() -> String {
    "var rc4 = function (str, key) {\n\
     var s = [], j = 0, x, output = '';\n\
     str = atobPoly(str);\n\
     for (var i = 0; i < 256; i++) { s[i] = i; }\n\
     for (i = 0; i < 256; i++) {\n\
     j = (j + s[i] + key['charCodeAt'](i % key['length'])) % 256;\n\
     x = s[i]; s[i] = s[j]; s[j] = x;\n\
     }\n\
     i = 0; j = 0;\n\
     for (var y = 0; y < str['length']; y++) {\n\
     i = (i + 1) % 256; j = (j + s[i]) % 256;\n\
     x = s[i]; s[i] = s[j]; s[j] = x;\n\
     output += String['fromCharCode'](str['charCodeAt'](y) ^ s[(s[i] + s[j]) % 256]);\n\
     }\n\
     return output;\n\
     };"
    .to_string()
}

fn decoder_body(encoding: Encoding, shift: u32) -> String {
    let head = format!("f = f - {shift};\nvar i = e[f];");
    match encoding {
        Encoding::None => format!("{head}\nreturn i;"),
        Encoding::Base64 => format!(
            "{head}\nif (b['init'] === undefined) {{\n{}\nb['decoder'] = atobPoly;\nb['cache'] = {{}};\nb['init'] = true;\n}}\nvar first = e[0], key = f + first, hit = b['cache'][key];\nreturn !hit ? ((i = b['decoder'](i)), (b['cache'][key] = i)) : (i = hit), i;",
            atob_body()
        ),
        Encoding::Rc4 => format!(
            "{head}\nif (b['init'] === undefined) {{\n{}\n{}\nb['decoder'] = rc4;\nb['cache'] = {{}};\nb['init'] = true;\n}}\nvar first = e[0], key = f + first, hit = b['cache'][key];\nreturn !hit ? ((i = b['decoder'](i, g)), (b['cache'][key] = i)) : (i = hit), i;",
            atob_body(),
            rc4_body()
        ),
    }
}

fn key_argument_for(encoding: Encoding) -> String {
    if encoding == Encoding::Rc4 {
        format!(", '{RC4_KEY}'")
    } else {
        String::new()
    }
}

fn build_string_array(encoding: Encoding, rotation: usize) -> String {
    let mut entries: Vec<String> = PLAINTEXTS.iter().map(|p| encode(p, encoding)).collect();
    let entry_count = entries.len();
    entries.rotate_left(rotation % entry_count);
    entries
        .iter()
        .map(|entry| format!("'{entry}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_calls(shift: u32, calls_transform: bool, key_argument: &str) -> String {
    let mut calls = String::new();
    for index in 0..PLAINTEXTS.len() {
        let literal = shift as usize + index;
        if calls_transform {
            calls.push_str(&format!(
                "(function () {{ var t = {{ v: {literal} }}; return b(t.v{key_argument}); }})();\n"
            ));
        } else {
            calls.push_str(&format!("b({literal}{key_argument});\n"));
        }
    }
    calls
}

fn build_rotator(rotation: usize, shift: u32, key_argument: &str) -> String {
    if rotation == 0 {
        return String::new();
    }
    format!(
        "(function (c, d) {{\n\
         var e = c();\n\
         var tries = 0;\n\
         while (!![]) {{\n\
         try {{\n\
         if (b({shift}{key_argument}) === '{}') break;\n\
         else e['push'](e['shift']());\n\
         }} catch (h) {{ e['push'](e['shift']()); }}\n\
         if (tries++ > 1000) break;\n\
         }}\n\
         }})(a, 0);\n",
        PLAINTEXTS[0]
    )
}

fn program(
    encoding: Encoding,
    rotation: usize,
    shift: u32,
    calls_transform: bool,
) -> (String, Vec<String>) {
    let items = build_string_array(encoding, rotation);
    let key_argument = key_argument_for(encoding);
    let calls = build_calls(shift, calls_transform, &key_argument);
    let rotator = build_rotator(rotation, shift, &key_argument);
    let source = format!(
        "function a() {{ var h = [{items}]; a = function () {{ return h; }}; return a(); }}\n\
         function b(c, d) {{\n\
         var e = a();\n\
         return (b = function (f, g) {{\n{}\n}}), b(c, d);\n\
         }}\n\
         {rotator}{calls}",
        decoder_body(encoding, shift)
    );
    (source, PLAINTEXTS.iter().map(|p| p.to_string()).collect())
}

pub fn generate(root: &Path) -> Result<usize> {
    let dir = root.join("fixtures/eval");
    fs::create_dir_all(&dir).context("create fixtures/eval")?;
    let mut manifest = Vec::new();
    for encoding in [Encoding::None, Encoding::Base64, Encoding::Rc4] {
        for rotation in [0usize, 1, 5] {
            for shift in [0u32, 0x1cc] {
                for calls_transform in [false, true] {
                    let (source, expected) = program(encoding, rotation, shift, calls_transform);
                    let name = format!(
                        "{}-rot{rotation}-shift{shift:x}{}.js",
                        encoding.label(),
                        if calls_transform { "-ct" } else { "" }
                    );
                    fs::write(dir.join(&name), &source).with_context(|| format!("write {name}"))?;
                    let rotator_index = if rotation == 0 {
                        serde_json::Value::Null
                    } else {
                        serde_json::json!(0)
                    };
                    manifest.push(serde_json::json!({
                        "file": name,
                        "array_fn": "a",
                        "decoders": ["b"],
                        "rotator_index": rotator_index,
                        "expected": expected,
                    }));
                }
            }
        }
    }
    let rendered = serde_json::to_string_pretty(&manifest).context("render manifest")?;
    fs::write(dir.join("manifest.json"), rendered).context("write manifest")?;
    Ok(manifest.len())
}
