use oxc_allocator::Allocator;
use oxc_ast::ast::Function;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::parse_program;

use super::base64::{FLAGGED, PLAIN, SELF_TEXT};
use super::rc4::{INLINED_ATOB, SEQUENCES, STATEMENTS};
use super::shape::function_shape;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

const TEMPLATES: [(&str, &str); 6] = [
    ("plain", PLAIN),
    ("self-text", SELF_TEXT),
    ("flagged", FLAGGED),
    ("rc4", STATEMENTS),
    ("rc4-sequences", SEQUENCES),
    ("rc4-inlined", INLINED_ATOB),
];

#[derive(Default)]
struct Shapes {
    found: Vec<String>,
}

impl<'a> VisitJs<'a> for Shapes {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(shape) = function_shape(it) {
            self.found.push(shape.text);
        }
        walk_js::walk_function(self, it, flags);
    }
}

fn shapes(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut shapes = Shapes::default();
    shapes.visit_program(&parsed.program);
    shapes.found
}

fn templates(source: &str) -> Vec<&'static str> {
    shapes(source)
        .iter()
        .filter_map(|text| {
            TEMPLATES
                .iter()
                .find(|(_, shape)| shape == text)
                .map(|(name, _)| *name)
        })
        .collect()
}

fn fixture(path: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURES}/{path}")).expect(path)
}

fn fixtures_named(dir: &str, prefix: &str) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(format!("{FIXTURES}/{dir}"))
        .expect(dir)
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with(prefix) && name.ends_with(".js"))
        .map(|name| format!("{dir}/{name}"))
        .collect();
    found.sort();
    found
}

#[test]
#[ignore = "prints the shapes of the functions in UNTWIST_SHAPE_FILES"]
fn print_shapes() {
    let files = std::env::var("UNTWIST_SHAPE_FILES").unwrap_or_default();
    for path in files.split(':').filter(|path| !path.is_empty()) {
        let source = std::fs::read_to_string(path).expect(path);
        for text in shapes(&source) {
            if text.contains("'abcdefghij") || text.contains("#256") {
                println!("{path}\n{text}\n");
            }
        }
    }
}

#[test]
fn every_eval_fixture_decoder_matches_its_template() {
    for path in fixtures_named("eval", "base64-") {
        assert_eq!(templates(&fixture(&path)), ["plain"], "{path}");
    }
    for path in fixtures_named("eval", "rc4-") {
        assert_eq!(templates(&fixture(&path)), ["plain", "rc4"], "{path}");
    }
}

#[test]
fn every_obfuscator_io_encoding_fixture_matches_its_templates() {
    for (prefix, expected) in [
        ("encoding-base64--", &["plain"][..]),
        ("encoding-rc4--", &["plain", "rc4-sequences"][..]),
        ("pair-rc4+self--", &["flagged", "rc4-sequences"][..]),
    ] {
        let paths = fixtures_named("obfuscator-io", prefix);
        assert_eq!(paths.len(), 10, "{prefix}");
        for path in paths {
            assert_eq!(templates(&fixture(&path)), expected, "{path}");
        }
    }
}

#[test]
fn the_npm_corpus_decoders_match_with_opaque_numbers_folded() {
    for (case, expected) in [
        ("002", "plain"),
        ("007", "plain"),
        ("003", "self-text"),
        ("004", "rc4-sequences"),
    ] {
        let path = format!("corpus-npm/case-{case}.js");
        assert!(templates(&fixture(&path)).contains(&expected), "{path}");
    }
}

fn first_base64() -> String {
    fixture("eval/base64-rot0-shift0.js")
}

#[test]
fn a_changed_constant_operator_or_statement_is_not_a_template() {
    let plain = first_base64();
    for (from, to) in [
        ("255 & ", "254 & "),
        ("% 4", "% 5"),
        ("'%'", "'&'"),
        ("+/=", "+/-"),
        ("decodeURIComponent(temp)", "decodeURIComponent(output)"),
        ("(-2 * bc)", "(-3 * bc)"),
        ("bs * 64", "bs * 65"),
        ("'00'", "'0'"),
        ("['slice'](-2)", "['slice'](-3)"),
        ("['toString'](16)", "['toString'](8)"),
        ("idx++", "++idx"),
        ("k++", "k += 2"),
        ("k < len", "k <= len"),
    ] {
        assert!(plain.contains(from), "{from}");
        let changed = plain.replacen(from, to, 1);
        assert!(!templates(&changed).contains(&"plain"), "{from} -> {to}");
    }
}

#[test]
fn split_or_lexical_declarations_match_the_same_template() {
    let plain = first_base64();
    let from = "var output = '', temp = '';";
    assert!(plain.contains(from));
    for to in [
        "var output = '';\nvar temp = '';",
        "let output = '', temp = '';",
        "let output = '';\nlet temp = '';",
    ] {
        let changed = plain.replacen(from, to, 1);
        assert_eq!(templates(&changed), ["plain"], "{to}");
    }
}

const RC4_SEQUENCES: &str = "var r = function (str, key) {var s = [], j = 0x0, x, out = '';str = atobPoly(str);var i;for (i = 0x0; i < 0x100; i++) {s[i] = i;}for (i = 0x0; i < 0x100; i++) {j = (j + s[i] + key['charCodeAt'](i % key['length'])) % 0x100, x = s[i], s[i] = s[j], s[j] = x;}i = 0x0, j = 0x0;for (var y = 0x0; y < str['length']; y++) {i = (i + 0x1) % 0x100, j = (j + s[i]) % 0x100, x = s[i], s[i] = s[j], s[j] = x, out += String['fromCharCode'](str['charCodeAt'](y) ^ s[(s[i] + s[j]) % 0x100]);}return out;};";

#[test]
fn split_sequence_statements_match_the_sequences_template() {
    assert_eq!(templates(RC4_SEQUENCES), ["rc4-sequences"]);
    let split = RC4_SEQUENCES
        .replace(
            ", x = s[i], s[i] = s[j], s[j] = x",
            "; x = s[i]; s[i] = s[j]; s[j] = x",
        )
        .replace("i = 0x0, j = 0x0;", "i = 0x0; j = 0x0;")
        .replace("0x100, j = (j", "0x100; j = (j")
        .replace(", out +=", "; out +=");
    assert!(!split.contains("x = s[i],"));
    assert_eq!(templates(&split), ["rc4-sequences"]);
    let parenthesized = RC4_SEQUENCES.replace("i = 0x0, j = 0x0;", "(i = 0x0, j = 0x0);");
    assert_eq!(templates(&parenthesized), ["rc4-sequences"]);
}

#[test]
fn a_constant_that_the_body_writes_is_not_a_template() {
    let plain = first_base64();
    let changed = plain.replacen(
        "var output = '', temp = '';",
        "const output = '', temp = '';",
        1,
    );
    assert!(templates(&changed).is_empty());
}

#[test]
fn an_extra_statement_or_parameter_is_not_a_template() {
    let plain = first_base64();
    for (from, to) in [
        (
            "return decodeURIComponent(temp);",
            "temp += ''; return decodeURIComponent(temp);",
        ),
        ("function (input)", "function (input, extra)"),
        ("function (input)", "function (input = '')"),
        ("var chars", "arguments; var chars"),
    ] {
        assert!(plain.contains(from), "{from}");
        let changed = plain.replacen(from, to, 1);
        assert!(!templates(&changed).contains(&"plain"), "{from} -> {to}");
    }
}

const RC4_INLINED: &str = "var rc4 = function (O, P) {for (var Q, R, S = [], T = 0x0, U = '', V = '', X = 0x0, Y = (O = function (a0) {for (var a1, a2, a3 = String(a0)['replace'](/=+$/, ''), a4 = '', a5 = 0x0, a6 = 0x0; a2 = a3['charAt'](a6++); ~a2 && (a1 = a5 % 0x4 ? 0x40 * a1 + a2 : a2, a5++ % 0x4) ? a4 += String['fromCharCode'](0xff & a1 >> (-0x2 * a5 & 0x6)) : 0x0) {a2 = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/='['indexOf'](a2);}return a4;}(O))['length']; X < Y; X++) {V += '%' + ('00' + O['charCodeAt'](X)['toString'](0x10))['slice'](-0x2);}O = decodeURIComponent(V);R = 0x0;for (; R < 0x100; R++) {S[R] = R;}for (R = 0x0; R < 0x100; R++) {T = (T + S[R] + P['charCodeAt'](R % P['length'])) % 0x100;Q = S[R];S[R] = S[T];S[T] = Q;}R = 0x0;T = 0x0;for (var Z = 0x0; Z < O['length']; Z++) {T = (T + S[R = (R + 0x1) % 0x100]) % 0x100;Q = S[R];S[R] = S[T];S[T] = Q;U += String['fromCharCode'](O['charCodeAt'](Z) ^ S[(S[R] + S[T]) % 0x100]);}return U;};";

#[test]
fn the_inlined_atob_rc4_matches_its_template_and_a_changed_regex_does_not() {
    assert_eq!(templates(RC4_INLINED), ["rc4-inlined"]);
    for (from, to) in [
        ("/=+$/", "/=*$/"),
        ("/=+$/", "/=+$/g"),
        ("% 0x100;Q", "% 0xff;Q"),
    ] {
        assert!(RC4_INLINED.contains(from), "{from}");
        let changed = RC4_INLINED.replacen(from, to, 1);
        assert!(templates(&changed).is_empty(), "{from} -> {to}");
    }
}
