use oxc_ast::ast::{Expression, Function};
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions, Context, Gen, GenExpr};
use oxc_span::GetSpan;
use oxc_syntax::precedence::Precedence;
use regress::{Flags, Regex};

use crate::value::FnCode;

const SELF_DEFENDING_PATTERN: &str = r##"\w+ *\(\) *\{\w+ *['|"].+['|"];? *\}"##;

pub fn function_source(source_text: &str, code: FnCode<'_>) -> String {
    let span = match code {
        FnCode::Function(function) => function.span(),
        FnCode::Arrow(arrow) => arrow.span(),
    };
    let start = span.start as usize;
    let end = span.end as usize;
    if start < end
        && end <= source_text.len()
        && source_text.is_char_boundary(start)
        && source_text.is_char_boundary(end)
    {
        return source_text[start..end].to_string();
    }
    pretty(code)
}

pub fn is_method_code(source_text: &str, code: FnCode<'_>) -> bool {
    let FnCode::Function(function) = code else {
        return false;
    };
    let start = function.span().start as usize;
    let text = source_text.get(start..).unwrap_or_default();
    !(text.starts_with("function") || text.starts_with("async"))
}

fn pretty(code: FnCode<'_>) -> String {
    let mut codegen = Codegen::new().with_options(CodegenOptions {
        minify: false,
        comments: CommentOptions::disabled(),
        ..CodegenOptions::default()
    });
    match code {
        FnCode::Function(function) => function.print(&mut codegen, Context::empty()),
        FnCode::Arrow(arrow) => {
            arrow.print_expr(&mut codegen, Precedence::Lowest, Context::empty())
        }
    }
    let rendered = codegen.into_source_text();
    match code {
        FnCode::Function(_) => strip_expression_statement_wrap(rendered),
        FnCode::Arrow(_) => rendered,
    }
}

fn strip_expression_statement_wrap(text: String) -> String {
    if text.starts_with('(') && text.ends_with(')') {
        text[1..text.len() - 1].to_string()
    } else {
        text
    }
}

fn compact_options() -> CodegenOptions {
    CodegenOptions {
        minify: true,
        comments: CommentOptions::disabled(),
        ..CodegenOptions::default()
    }
}

pub(crate) fn compact_function(function: &Function<'_>) -> String {
    let mut codegen = Codegen::new().with_options(compact_options());
    function.print(&mut codegen, Context::empty());
    codegen.into_source_text()
}

pub(crate) fn compact_expression(expr: &Expression<'_>) -> String {
    let mut codegen = Codegen::new().with_options(compact_options());
    expr.print_expr(&mut codegen, Precedence::Lowest, Context::empty());
    codegen.into_source_text()
}

pub fn satisfies_self_defending_regex(text: &str) -> bool {
    let Ok(regex) = Regex::with_flags(SELF_DEFENDING_PATTERN, Flags::from("")) else {
        return false;
    };
    let units: Vec<u16> = text.encode_utf16().collect();
    regex.find_from_ucs2(&units, 0).next().is_some()
}
