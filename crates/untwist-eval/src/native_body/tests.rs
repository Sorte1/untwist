use oxc_allocator::Allocator;
use oxc_ast::ast::Function;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::parse_program;

use super::base64::{FLAGGED, PLAIN, SELF_TEXT};
use super::rc4::{SEQUENCES, STATEMENTS};
use super::shape::function_shape;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

const TEMPLATES: [(&str, &str); 5] = [
    ("plain", PLAIN),
    ("self-text", SELF_TEXT),
    ("flagged", FLAGGED),
    ("rc4", STATEMENTS),
    ("rc4-sequences", SEQUENCES),
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
