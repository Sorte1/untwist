use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::{ContentEq, SourceType};
use oxc_syntax::scope::ScopeFlags;

use crate::Mode;

struct Strip;

impl<'a> VisitMut<'a> for Strip {
    fn visit_formal_parameter(&mut self, it: &mut FormalParameter<'a>) {
        it.type_annotation = None;
        it.optional = false;
        walk_mut::walk_formal_parameter(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        it.type_annotation = None;
        it.definite = false;
        walk_mut::walk_variable_declarator(self, it);
    }

    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        it.return_type = None;
        walk_mut::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &mut ArrowFunctionExpression<'a>) {
        it.return_type = None;
        walk_mut::walk_arrow_function_expression(self, it);
    }
}

fn parse<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    source_type: SourceType,
) -> Option<Program<'a>> {
    let options = ParseOptions {
        allow_return_outside_function: true,
        preserve_parens: false,
        ..ParseOptions::default()
    };
    let parsed = Parser::new(alloc, source, source_type)
        .with_options(options)
        .parse();
    let failed = parsed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == oxc_diagnostics::Severity::Error);
    (!failed).then_some(parsed.program)
}

pub(crate) fn parse_js<'a>(
    alloc: &'a Allocator,
    source: &'a str,
) -> Option<(Program<'a>, SourceType)> {
    let script = untwist_core::parse::script_source_type();
    if let Some(program) = parse(alloc, source, script) {
        return Some((program, script));
    }
    let module = SourceType::mjs().with_jsx(true);
    parse(alloc, source, module).map(|program| (program, module))
}

pub(crate) fn typed_source_type(plain: SourceType, mode: Mode, jsx: bool) -> SourceType {
    let module = plain.is_module();
    let base = match (mode, jsx) {
        (Mode::Ts, true) => SourceType::tsx(),
        (Mode::Ts, false) => SourceType::ts(),
        (Mode::Jsdoc, _) => plain,
    };
    base.with_module(module).with_script(!module)
}

pub(crate) fn same_after_strip(
    plain: &Program<'_>,
    typed: &str,
    plain_type: SourceType,
    mode: Mode,
    jsx: bool,
) -> bool {
    let alloc = Allocator::default();
    let Some(mut annotated) = parse(&alloc, typed, typed_source_type(plain_type, mode, jsx)) else {
        return false;
    };
    Strip.visit_program(&mut annotated);
    plain.hashbang.content_eq(&annotated.hashbang)
        && plain.directives.content_eq(&annotated.directives)
        && plain.body.content_eq(&annotated.body)
}

pub fn erases(plain: &str, typed: &str, mode: Mode, jsx: bool) -> Result<(), String> {
    let alloc = Allocator::default();
    let Some((untyped, plain_type)) = parse_js(&alloc, plain) else {
        return Err("the untyped output does not parse".to_string());
    };
    match same_after_strip(&untyped, typed, plain_type, mode, jsx) {
        true => Ok(()),
        false => Err("stripping the annotations does not give the untyped output back".to_string()),
    }
}
