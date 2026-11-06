mod common;

use common::eval_with;
use untwist_eval::budget::Budget;
use untwist_eval::test_hooks::{
    native_body_runs, record_units_charged, set_native_bodies, UnitRecord,
};

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
const BUDGET: u64 = 50_000_000;

const ATOB: &str = r"
var atobPoly = function (input) {
var chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';
var output = '', temp = '';
for (var bc = 0, bs, buffer, idx = 0; (buffer = input['charAt'](idx++)); ~buffer && ((bs = bc % 4 ? bs * 64 + buffer : buffer), bc++ % 4) ? (output += String['fromCharCode'](255 & (bs >> ((-2 * bc) & 6)))) : 0) {
buffer = chars['indexOf'](buffer);
}
for (var k = 0, len = output['length']; k < len; k++) {
temp += '%' + ('00' + output['charCodeAt'](k)['toString'](16))['slice'](-2);
}
return decodeURIComponent(temp);
};
";

const ATOB_LEXICAL: &str = r"
var atobPoly = function (input) {
const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';
let output = '', temp = '';
for (let bc = 0x0, bs, buffer, idx = 0x0; buffer = input.charAt(idx++); ~buffer && (bs = bc % 0x4 ? bs * 0x40 + buffer : buffer, bc++ % 0x4) ? output += String.fromCharCode(0xff & bs >> (-0x2 * bc & 0x6)) : 0x0) {
buffer = chars.indexOf(buffer);
}
for (let k = 0x0, len = output.length; k < len; k++) {
temp += '%' + ('00' + output.charCodeAt(k).toString(0x10)).slice(-0x2);
}
return decodeURIComponent(temp);
};
";

const RC4: &str = r"
var rc4 = function (str, key) {
var s = [], j = 0, x, output = '';
str = atobPoly(str);
for (var i = 0; i < 256; i++) { s[i] = i; }
for (i = 0; i < 256; i++) {
j = (j + s[i] + key['charCodeAt'](i % key['length'])) % 256;
x = s[i]; s[i] = s[j]; s[j] = x;
}
i = 0; j = 0;
for (var y = 0; y < str['length']; y++) {
i = (i + 1) % 256; j = (j + s[i]) % 256;
x = s[i]; s[i] = s[j]; s[j] = x;
output += String['fromCharCode'](str['charCodeAt'](y) ^ s[(s[i] + s[j]) % 256]);
}
return output;
};
";

const RC4_SEQUENCES: &str = r"
var rc4 = function (str, key) {
var s = [], j = 0x0, x, output = '';
str = atobPoly(str);
var i;
for (i = 0x0; i < 0x100; i++) { s[i] = i; }
for (i = 0x0; i < 0x100; i++) {
j = (j + s[i] + key['charCodeAt'](i % key['length'])) % 0x100, x = s[i], s[i] = s[j], s[j] = x;
}
i = 0x0, j = 0x0;
for (var y = 0x0; y < str['length']; y++) {
i = (i + 0x1) % 0x100, j = (j + s[i]) % 0x100, x = s[i], s[i] = s[j], s[j] = x, output += String['fromCharCode'](str['charCodeAt'](y) ^ s[(s[i] + s[j]) % 0x100]);
}
return output;
};
";

const SELF_TEXT: &str = "var d = function (s) {var t = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var o = '', p = '', q = o + d;for (var a = 0x0, b, c, e = 0x0; c = s['charAt'](e++); ~c && (b = a % 0x4 ? b * 0x40 + c : c, a++ % 0x4) ? o += q['charCodeAt'](e + 0xa) - 0xa !== 0x0 ? String['fromCharCode'](0xff & b >> (-0x2 * a & 0x6)) : a : 0x0) {c = t['indexOf'](c);}for (var f = 0x0, g = o['length']; f < g; f++) {p += '%' + ('00' + o['charCodeAt'](f)['toString'](0x10))['slice'](-0x2);}return decodeURIComponent(p);};";

const FLAGGED: &str = "var d = function (s) {var t = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var o = '', p = '', q = o + d;var r = ('' + function () { return 0x0; })['indexOf']('\\n') !== -0x1;for (var a = 0x0, b, c, e = 0x0; c = s['charAt'](e++); ~c && (b = a % 0x4 ? b * 0x40 + c : c, a++ % 0x4) ? o += r || q['charCodeAt'](e + 0xa) - 0xa !== 0x0 ? String['fromCharCode'](0xff & b >> (-0x2 * a & 0x6)) : a : 0x0) {c = t['indexOf'](c);}for (var f = 0x0, g = o['length']; f < g; f++) {p += '%' + ('00' + o['charCodeAt'](f)['toString'](0x10))['slice'](-0x2);}return decodeURIComponent(p);};";

fn encode(bytes: &[u8], pad: bool) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut word = [0_u8; 3];
        word[..chunk.len()].copy_from_slice(chunk);
        let bits = u32::from(word[0]) << 16 | u32::from(word[1]) << 8 | u32::from(word[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(ALPHABET[(bits >> (18 - 6 * index) & 63) as usize] as char);
            } else if pad {
                out.push('=');
            }
        }
    }
    out
}

fn texts() -> Vec<String> {
    let long = "x".repeat(300);
    let words = [
        "",
        "a",
        "ab",
        "abc",
        "abcd",
        "push",
        "length",
        "constructor",
        "héllo wörld",
        "日本語のテキスト",
        "emoji 😀 pair",
        "%41 not escaped",
        "tab\tnew\nline",
        &long,
    ];
    words
        .iter()
        .map(|word| encode(word.as_bytes(), false))
        .collect()
}

fn messy() -> Vec<String> {
    let mut out: Vec<String> = ["abc", "héllo", "x"]
        .iter()
        .map(|word| encode(word.as_bytes(), true))
        .collect();
    out.extend(["ab cd!ef\u{e9}gh", "====", "=", "a", "*", "\u{1f600}abcd"].map(str::to_string));
    out.push(encode(&[0xff, 0xfe], false));
    out
}

fn js_list(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| serde_json::to_string(item).expect("json"))
        .collect();
    format!("[{}]", quoted.join(","))
}

fn run(source: &str, budget: u64, native: bool) -> (Result<String, String>, Vec<UnitRecord>, u64) {
    set_native_bodies(native);
    let before = native_body_runs();
    let mut rendered = Err(String::new());
    let records = record_units_charged(|| rendered = eval_with(source, Budget::new(budget)));
    let runs = native_body_runs() - before;
    set_native_bodies(true);
    (rendered, records, runs)
}

fn assert_same_records(native: &[UnitRecord], interpreted: &[UnitRecord], context: &str) {
    assert_eq!(native.len(), interpreted.len(), "{context}");
    for (native, interpreted) in native.iter().zip(interpreted) {
        assert!(
            native.heap_peak <= interpreted.heap_peak,
            "{context}: {native:?} {interpreted:?}"
        );
        let mut levelled = native.clone();
        levelled.heap_peak = interpreted.heap_peak;
        assert_eq!(&levelled, interpreted, "{context}");
    }
}

fn same_at(source: &str, budget: u64) -> (Result<String, String>, u64) {
    let (interpreted, interpreted_records, none) = run(source, budget, false);
    let (native, native_records, runs) = run(source, budget, true);
    assert_eq!(none, 0);
    assert_eq!(native, interpreted, "{source}");
    assert_same_records(&native_records, &interpreted_records, source);
    (native, runs)
}

fn same(source: &str) -> (Result<String, String>, u64) {
    same_at(source, BUDGET)
}

fn decode_all(decoder: &str, call: &str, inputs: &[String]) -> String {
    format!(
        "{decoder} var inputs = {}, out = []; for (var n = 0; n < inputs.length; n++) out.push(escape({call})); out.join('|')",
        js_list(inputs)
    )
}

#[test]
fn every_base64_form_returns_what_the_interpreter_returns() {
    for decoder in [ATOB, ATOB_LEXICAL] {
        let source = decode_all(decoder, "atobPoly(inputs[n])", &texts());
        let (rendered, runs) = same(&source);
        assert!(rendered.expect("decodes").contains("h%E9llo"));
        assert_eq!(runs, 2 * texts().len() as u64);
    }
}

#[test]
fn the_self_text_forms_return_what_the_interpreter_returns() {
    for decoder in [SELF_TEXT, FLAGGED] {
        let (rendered, runs) = same(&decode_all(decoder, "d(inputs[n])", &texts()));
        assert!(rendered.is_ok());
        assert_eq!(runs, 2 * texts().len() as u64);
    }
}

#[test]
fn a_line_feed_in_the_source_text_appends_the_counter_as_interpreted() {
    let words: Vec<String> = ["push", "length", "constructor", &"x".repeat(300)]
        .iter()
        .map(|word| encode(word.as_bytes(), false))
        .collect();
    for decoder in [SELF_TEXT, FLAGGED] {
        let broken = decoder.replacen("{var t", "{\nvar t", 1);
        let (rendered, runs) = same(&decode_all(&broken, "d(inputs[n])", &words));
        assert!(rendered
            .expect("decodes")
            .chars()
            .any(|c| c.is_ascii_digit()));
        assert!(runs > 0);
    }
}

#[test]
fn messy_input_runs_exactly_as_interpreted() {
    for input in messy() {
        let quoted = serde_json::to_string(&input).expect("json");
        let _ = same(&format!("{ATOB} atobPoly({quoted})"));
        let _ = same(&format!("{SELF_TEXT} d({quoted})"));
        let _ = same(&format!("{FLAGGED} d({quoted})"));
        let _ = same(&format!("{ATOB}{RC4} escape(rc4({quoted}, 'key'))"));
        let _ = same(&format!(
            "{ATOB}{RC4_SEQUENCES} escape(rc4({quoted}, 'key'))"
        ));
    }
}

#[test]
fn both_rc4_forms_return_what_the_interpreter_returns() {
    let keys = ["DiKO", "k", "a longer key with ümlauts", "\u{1f600}"];
    for rc4 in [RC4, RC4_SEQUENCES] {
        let calls: Vec<String> = texts()
            .iter()
            .enumerate()
            .map(|(index, text)| {
                let key = serde_json::to_string(keys[index % keys.len()]).expect("json");
                format!("rc4({}, {key})", serde_json::to_string(text).expect("json"))
            })
            .collect();
        let source = format!(
            "{ATOB}{rc4} [{}].map(function (v) {{ return escape(v); }}).join('|')",
            calls.join(",")
        );
        let (rendered, runs) = same(&source);
        assert!(rendered.is_ok());
        assert_eq!(runs, 5 * texts().len() as u64);
    }
}

#[test]
fn a_decode_error_is_the_same_error() {
    let quoted = serde_json::to_string(&encode(&[0xff, 0xfe], true)).expect("json");
    let (rendered, runs) = same(&format!("{ATOB} atobPoly({quoted})"));
    assert!(rendered.is_err());
    assert_eq!(runs, 2);
    let caught = format!("{ATOB} var seen = ''; try {{ atobPoly({quoted}); }} catch (e) {{ seen = 'caught'; }} seen + atobPoly('YWJj')");
    let _ = same(&caught);
}

#[test]
fn patched_built_ins_run_exactly_as_interpreted() {
    let input = serde_json::to_string(&encode(b"patched", false)).expect("json");
    for patch in [
        "String.fromCharCode = function (c) { calls++; return 'x'; };",
        "String.prototype.charAt = function (i) { calls++; return i < 3 ? 'b' : ''; };",
        "String.prototype.indexOf = function (c) { calls++; return 1; };",
        "String.prototype.charCodeAt = function () { calls++; return 65; };",
        "String.prototype.slice = function () { calls++; return '41'; };",
        "Number.prototype.toString = function () { calls++; return '41'; };",
        "decodeURIComponent = function (t) { calls++; return t; };",
        "String = { fromCharCode: function () { calls++; return 'y'; } };",
        "String.prototype.indexOf = String.prototype.charAt;",
    ] {
        for decoder in [ATOB, SELF_TEXT, FLAGGED] {
            let call = if decoder == ATOB { "atobPoly" } else { "d" };
            let source = format!("var calls = 0; {patch} {decoder} {call}({input}) + ':' + calls");
            let _ = same(&source);
        }
    }
}

#[test]
fn a_patched_rc4_step_runs_exactly_as_interpreted() {
    let input = serde_json::to_string(&encode(b"patched", false)).expect("json");
    for rc4 in [RC4, RC4_SEQUENCES] {
        for patch in [
            "atobPoly = function (s) { calls++; return s; };",
            "atobPoly = function (s) { calls++; return 7; };",
            "String.prototype.charCodeAt = function () { calls++; return 7; };",
            "String.fromCharCode = function (c) { calls++; return 'x'; };",
            "Array.prototype[3] = 9;",
        ] {
            let source = format!(
                "var calls = 0; {ATOB}{rc4} {patch} escape(rc4({input}, 'key')) + ':' + calls"
            );
            let _ = same(&source);
        }
        for key in ["''", "5", "null", "'\u{1f600}'"] {
            let _ = same(&format!("{ATOB}{rc4} escape(rc4({input}, {key}))"));
        }
    }
}

#[test]
fn a_shadowed_global_is_read_as_the_interpreter_reads_it() {
    let input = serde_json::to_string(&encode(b"shadow", false)).expect("json");
    let wrapped = format!(
        "(function (String) {{ {ATOB} return atobPoly({input}); }})({{ fromCharCode: function (c) {{ return '#'; }} }})"
    );
    let (rendered, runs) = same(&wrapped);
    assert!(rendered.expect("decodes").contains("######"));
    assert_eq!(runs, 1);
}

#[test]
fn every_step_budget_across_a_call_fails_or_succeeds_as_interpreted() {
    let input = serde_json::to_string(&encode(b"abc", false)).expect("json");
    for source in [
        format!("{ATOB} atobPoly({input})"),
        format!("{FLAGGED} d({input})"),
        format!("{ATOB}{RC4_SEQUENCES} escape(rc4({input}, 'k'))"),
    ] {
        let (_, records, _) = run(&source, BUDGET, false);
        let total: u64 = records.iter().map(|record| record.steps_used).sum();
        assert!(same(&source).1 > 0, "{source}");
        let stride = (total / 150).max(1);
        for budget in (0..=total + 1)
            .step_by(stride as usize)
            .chain(total.saturating_sub(64)..=total + 1)
        {
            let _ = same_at(&source, budget);
        }
    }
}

#[test]
fn every_single_step_budget_of_a_short_decode_matches() {
    let input = serde_json::to_string(&encode(b"ab", false)).expect("json");
    for source in [
        format!("{ATOB} atobPoly({input})"),
        format!("{ATOB_LEXICAL} atobPoly({input})"),
        format!("{SELF_TEXT} d({input})"),
        format!("{FLAGGED} d({input})"),
    ] {
        let (_, records, _) = run(&source, BUDGET, false);
        let total: u64 = records.iter().map(|record| record.steps_used).sum();
        assert!(same(&source).1 > 0, "{source}");
        for budget in 0..=total {
            let _ = same_at(&source, budget);
        }
    }
}

#[test]
fn every_seventh_step_budget_of_an_rc4_decode_matches() {
    let input = serde_json::to_string(&encode(b"a", false)).expect("json");
    for rc4 in [RC4, RC4_SEQUENCES] {
        let source = format!("{ATOB}{rc4} escape(rc4({input}, 'k'))");
        let (_, records, _) = run(&source, BUDGET, false);
        let total: u64 = records.iter().map(|record| record.steps_used).sum();
        assert_eq!(same(&source).1, 5, "{source}");
        for budget in (0..=total).step_by(7) {
            let _ = same_at(&source, budget);
        }
    }
}

#[test]
fn a_loop_past_the_iteration_cap_bails_as_interpreted() {
    let long = serde_json::to_string(&"QUFB".repeat(25_001)).expect("json");
    let (rendered, runs) = same_at(&format!("{ATOB} atobPoly({long}).length"), 400_000_000);
    assert!(rendered.is_err(), "{rendered:?}");
    assert_eq!(runs, 0);
}

#[test]
fn a_value_that_is_not_text_is_interpreted() {
    for argument in [
        "5",
        "null",
        "{ charAt: function () { return ''; } }",
        "['YWJj']",
    ] {
        let _ = same(&format!("{ATOB} String(atobPoly({argument}))"));
    }
}

#[test]
fn the_native_body_spends_exactly_the_interpreted_steps() {
    let long = serde_json::to_string(&encode("decoded strings ".repeat(40).as_bytes(), false))
        .expect("json");
    let source = format!("{ATOB}{RC4} var out = ''; for (var n = 0; n < 40; n++) out = rc4({long}, 'key' + n); out.length");
    let (rendered, runs) = same(&source);
    assert_eq!(rendered, Ok("number:640".to_string()));
    assert_eq!(runs, 40 * 5);
}
