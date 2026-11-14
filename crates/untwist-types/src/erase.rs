use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;
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

fn printed(program: &Program<'_>) -> String {
    Codegen::new()
        .with_options(CodegenOptions {
            comments: CommentOptions::disabled(),
            ..CodegenOptions::default()
        })
        .build(program)
        .code
}

fn parse<'a>(alloc: &'a Allocator, source: &'a str, source_type: SourceType) -> Option<Program<'a>> {
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

pub fn typed_source_type(plain: SourceType, mode: Mode, jsx: bool) -> SourceType {
    let module = plain.is_module();
    let base = match (mode, jsx) {
        (Mode::Ts, true) => SourceType::tsx(),
        (Mode::Ts, false) => SourceType::ts(),
        (Mode::Jsdoc, _) => plain,
    };
    base.with_module(module).with_script(!module)
}

pub fn erases(plain: &str, typed: &str, mode: Mode, jsx: bool) -> Result<(), String> {
    let plain_type = {
        let alloc = Allocator::default();
        untwist_core::parse::parse_program(&alloc, plain).source_type
    };
    erases_as(plain, typed, plain_type, mode, jsx)
}

pub fn erases_as(plain: &str, typed: &str, plain_type: SourceType, mode: Mode, jsx: bool) -> Result<(), String> {
    let alloc = Allocator::default();
    let Some(untyped) = parse(&alloc, plain, plain_type) else {
        return Err("the untyped output does not parse".to_string());
    };
    let Some(mut annotated) = parse(&alloc, typed, typed_source_type(plain_type, mode, jsx)) else {
        return Err("the annotated output does not parse".to_string());
    };
    Strip.visit_program(&mut annotated);
    match printed(&untyped) == printed(&annotated) {
        true => Ok(()),
        false => Err("stripping the annotations does not give the untyped output back".to_string()),
    }
}
