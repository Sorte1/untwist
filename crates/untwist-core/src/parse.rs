use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionExpression, AwaitExpression, Class, ForOfStatement, Function, Program, Statement,
    VariableDeclaration,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::{OxcDiagnostic, Severity};
use oxc_parser::{ParseOptions, Parser};
pub use oxc_span::SourceType;

mod unnamed;
use oxc_syntax::scope::ScopeFlags;

pub struct Parsed<'a> {
    pub program: Program<'a>,
    pub diagnostics: Vec<OxcDiagnostic>,
    pub source_type: SourceType,
}

pub fn script_source_type() -> SourceType {
    SourceType::cjs().with_jsx(true)
}

pub fn parse_program<'a>(alloc: &'a Allocator, source: &'a str) -> Parsed<'a> {
    let (mut as_module, module_syntax) =
        parse_recording(alloc, source, SourceType::mjs().with_jsx(true));
    if !has_error(&as_module.diagnostics) {
        if !module_syntax && !has_top_level_await(&as_module.program) {
            as_module.source_type = script_source_type();
            as_module.program.source_type = as_module.source_type;
        }
        return as_module;
    }
    let as_script = parse_with(alloc, source, script_source_type());
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
    parse_recording(alloc, source, source_type).0
}

fn parse_recording<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    source_type: SourceType,
) -> (Parsed<'a>, bool) {
    let options = ParseOptions {
        allow_return_outside_function: true,
        preserve_parens: false,
        ..ParseOptions::default()
    };
    let mut ret = Parser::new(alloc, source, source_type)
        .with_options(options)
        .parse();
    unnamed::keep_unnamed(alloc, source, &mut ret.program);
    let module_syntax = ret.module_record.has_module_syntax;
    let parsed = Parsed {
        program: ret.program,
        diagnostics: ret.diagnostics.into_vec(),
        source_type,
    };
    (parsed, module_syntax)
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

#[derive(Default)]
struct TopLevelAwait {
    found: bool,
}

impl<'a> VisitJs<'a> for TopLevelAwait {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.found {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_await_expression(&mut self, _: &AwaitExpression<'a>) {
        self.found = true;
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.found |= it.r#await;
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        self.found |= it.kind.is_await_using();
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _: &Class<'a>) {}
}

fn has_top_level_await(program: &Program<'_>) -> bool {
    let mut scan = TopLevelAwait::default();
    scan.visit_program(program);
    scan.found
}
