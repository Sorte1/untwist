use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::{OxcDiagnostic, Severity};
use oxc_parser::{ParseOptions, Parser};
pub use oxc_span::SourceType;

pub struct Parsed<'a> {
    pub program: Program<'a>,
    pub diagnostics: Vec<OxcDiagnostic>,
    pub source_type: SourceType,
}

pub fn parse_program<'a>(alloc: &'a Allocator, source: &'a str) -> Parsed<'a> {
    let as_module = parse_with(alloc, source, SourceType::mjs().with_jsx(true));
    if !has_error(&as_module.diagnostics) {
        return as_module;
    }
    let as_script = parse_with(alloc, source, SourceType::cjs().with_jsx(true));
    if error_count(&as_script.diagnostics) < error_count(&as_module.diagnostics) {
        as_script
    } else {
        as_module
    }
}

pub fn parse_with<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    source_type: SourceType,
) -> Parsed<'a> {
    let options = ParseOptions {
        allow_return_outside_function: true,
        preserve_parens: false,
        ..ParseOptions::default()
    };
    let ret = Parser::new(alloc, source, source_type)
        .with_options(options)
        .parse();
    Parsed {
        program: ret.program,
        diagnostics: ret.diagnostics.into_vec(),
        source_type,
    }
}

pub fn has_error(diagnostics: &[OxcDiagnostic]) -> bool {
    error_count(diagnostics) > 0
}

pub fn error_count(diagnostics: &[OxcDiagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count()
}

pub fn strip_bookmarklet(source: &str) -> Cow<'_, str> {
    let Some(rest) = source.strip_prefix("javascript:") else {
        return Cow::Borrowed(source);
    };
    Cow::Owned(decode_percent_segments(rest))
}

fn decode_percent_segments(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let is_escape = bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit();
        if is_escape {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00");
            out.push(u8::from_str_radix(hex, 16).unwrap_or(b'%'));
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
