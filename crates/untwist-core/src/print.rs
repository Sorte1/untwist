use oxc_ast::ast::{Function, Program};
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions, Context, Gen, IndentChar};

use std::path::PathBuf;

use oxc_sourcemap::{OwnedSourceMap, SourceMap};

use crate::reflow::reflow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintStyle {
    Pretty,
    Compact,
}

fn options(style: PrintStyle) -> CodegenOptions {
    match style {
        PrintStyle::Pretty => CodegenOptions {
            minify: false,
            indent_char: IndentChar::Space,
            indent_width: 2,
            comments: CommentOptions::default(),
            ..CodegenOptions::default()
        },
        PrintStyle::Compact => CodegenOptions {
            minify: true,
            comments: CommentOptions::disabled(),
            ..CodegenOptions::default()
        },
    }
}

fn codegen(program: &Program<'_>, style: PrintStyle) -> String {
    let mut code = Codegen::new()
        .with_options(options(style))
        .build(program)
        .code;
    code.truncate(code.trim_end().len());
    code.shrink_to_fit();
    code
}

pub fn pretty_codegen(program: &Program<'_>) -> String {
    codegen(program, PrintStyle::Pretty)
}

pub fn mapped_codegen(program: &Program<'_>, source: &str) -> (String, Option<OwnedSourceMap>) {
    let options = CodegenOptions {
        comments: CommentOptions::disabled(),
        source_map_path: Some(PathBuf::from(source)),
        ..options(PrintStyle::Pretty)
    };
    let printed = Codegen::new().with_options(options).build(program);
    let mut code = printed.code;
    code.truncate(code.trim_end().len());
    (code, printed.map.map(SourceMap::into_owned_sourcemap))
}

fn unwrapped(text: String) -> String {
    match text
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
    {
        Some(inner) => inner.to_string(),
        None => text,
    }
}

fn printed_alone(function: &Function<'_>, options: CodegenOptions) -> String {
    let mut codegen = Codegen::new().with_options(options);
    function.print(&mut codegen, Context::empty());
    unwrapped(codegen.into_source_text())
}

pub fn function_text(function: &Function<'_>, minify: bool) -> String {
    printed_alone(
        function,
        CodegenOptions {
            minify,
            indent_char: IndentChar::Space,
            indent_width: 0,
            comments: CommentOptions::disabled(),
            ..CodegenOptions::default()
        },
    )
}

pub fn pretty_function_text(function: &Function<'_>) -> String {
    printed_alone(function, options(PrintStyle::Pretty))
}

pub fn compact_function_text(function: &Function<'_>) -> String {
    function_text(function, false).replace('\n', "")
}

pub fn print(program: &Program<'_>, style: PrintStyle) -> String {
    let code = codegen(program, style);
    match style {
        PrintStyle::Pretty => reflow(code),
        PrintStyle::Compact => code,
    }
}
