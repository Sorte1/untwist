use oxc_span::Span;

use super::names::Names;
use super::tokens::{canonical_in, dotted, render, Token};

const TABLE: &str = "%TABLE%";

const PRELUDES: [(usize, &str); 3] = [
    (9, include_str!("templates/prelude-9-plain.txt")),
    (9, include_str!("templates/prelude-9-scrambler.txt")),
    (3, include_str!("templates/prelude-3-minify.txt")),
];

const DECODERS: [&str; 3] = [
    include_str!("templates/decoder-plain.txt"),
    include_str!("templates/decoder-minify.txt"),
    include_str!("templates/decoder-scrambler.txt"),
];

const GETTERS: [&str; 2] = [
    include_str!("templates/getter-plain.txt"),
    include_str!("templates/getter-minify.txt"),
];

const MOVED_GETTERS: [&str; 2] = [
    include_str!("templates/getter-moved-plain.txt"),
    include_str!("templates/getter-moved-minify.txt"),
];

const BUFFER_TO_STRING: [&str; 1] = [include_str!("templates/buffer-to-string-minify.txt")];

const MOVED_GET_GLOBALS: [&str; 1] = [include_str!("templates/getglobal-moved.txt")];

const GET_GLOBALS: [&str; 3] = [
    include_str!("templates/getglobal-plain.txt"),
    include_str!("templates/getglobal-minify.txt"),
    include_str!("templates/getglobal-scrambler.txt"),
];

const SELF_DEFENDING: [&str; 1] = [include_str!("templates/self-defending-plain.txt")];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CffHelper {
    Sum,
    Slice,
    Xor,
    Hash,
}

const CFF_HELPERS: [(CffHelper, &str); 4] = [
    (CffHelper::Sum, include_str!("templates/cff-sum.txt")),
    (CffHelper::Slice, include_str!("templates/cff-slice.txt")),
    (CffHelper::Xor, include_str!("templates/cff-xor.txt")),
    (CffHelper::Hash, include_str!("templates/cff-hash.txt")),
];

pub const PRELUDE_LENGTHS: [usize; 2] = [9, 3];

const INTEGRITY: [(usize, &str); 1] = [(6, include_str!("templates/integrity-6-plain.txt"))];

pub const INTEGRITY_LENGTHS: [usize; 1] = [6];

fn slice(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

pub(super) fn canonical_text(source: &str, span: Span, names: &Names) -> Option<String> {
    let resolve = |start: u32| names.resolve(start);
    Some(render(&canonical_in(
        slice(source, span),
        span.start,
        &resolve,
        names.literals().as_ref(),
    )?))
}

fn listed(list: &[&str], found: Option<String>) -> bool {
    found.is_some_and(|found| list.iter().any(|t| t.trim() == found))
}

pub fn matches_prelude(source: &str, span: Span, names: &Names) -> bool {
    listed(
        &PRELUDES.map(|(_, t)| t),
        canonical_text(source, span, names),
    )
}

pub fn matches_get_global(source: &str, span: Span, names: &Names) -> bool {
    listed(&GET_GLOBALS, canonical_text(source, span, names))
}

pub fn matches_self_defending(source: &str, span: Span, names: &Names) -> bool {
    listed(&SELF_DEFENDING, canonical_text(source, span, names))
}

pub fn matches_integrity(source: &str, span: Span, names: &Names) -> bool {
    listed(
        &INTEGRITY.map(|(_, t)| t),
        canonical_text(source, span, names),
    )
}

pub fn cff_helper(source: &str, span: Span, names: &Names) -> Option<CffHelper> {
    let found = canonical_text(source, span, names)?;
    CFF_HELPERS
        .iter()
        .find(|(_, template)| template.trim() == found)
        .map(|(kind, _)| *kind)
}

pub fn matches_getter(source: &str, span: Span, names: &Names) -> bool {
    listed(&GETTERS, canonical_text(source, span, names))
}

fn dotted_text(source: &str, span: Span, names: &Names) -> Option<String> {
    let resolve = |start: u32| names.resolve(start);
    let tokens = canonical_in(
        slice(source, span),
        span.start,
        &resolve,
        names.literals().as_ref(),
    )?;
    Some(render(&dotted(tokens)))
}

pub fn matches_moved_getter(source: &str, span: Span, names: &Names) -> bool {
    listed(&MOVED_GETTERS, dotted_text(source, span, names))
}

pub fn matches_buffer_to_string(source: &str, span: Span, names: &Names) -> bool {
    listed(&BUFFER_TO_STRING, dotted_text(source, span, names))
}

pub fn matches_moved_get_global(source: &str, span: Span, names: &Names) -> bool {
    listed(&MOVED_GET_GLOBALS, dotted_text(source, span, names))
}

pub(super) fn decoder_shape(source: &str, span: Span, names: &Names) -> Option<(String, Vec<u16>)> {
    let resolve = |start: u32| names.resolve(start);
    let mut tokens = canonical_in(
        slice(source, span),
        span.start,
        &resolve,
        names.literals().as_ref(),
    )?;
    let at = tokens.iter().position(|t| matches!(t, Token::Str(_)))?;
    let Token::Str(table) = std::mem::replace(&mut tokens[at], Token::Name(TABLE.to_string()))
    else {
        return None;
    };
    Some((render(&tokens), table))
}

pub fn matches_decoder(source: &str, span: Span, names: &Names) -> Option<Vec<u16>> {
    let (found, table) = decoder_shape(source, span, names)?;
    DECODERS.iter().any(|t| t.trim() == found).then_some(table)
}

#[cfg(test)]
mod dump {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_span::{GetSpan, Span};
    use untwist_core::parse::parse_program;
    use untwist_core::semantic::build_scoping;

    use super::super::names::names;
    use super::*;

    #[test]
    #[ignore]
    fn print_canonical_pieces() {
        let path = std::env::var("JSC_DUMP").expect("JSC_DUMP");
        let source = std::fs::read_to_string(path).expect("read");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &source);
        let program = &parsed.program;
        let sem = build_scoping(program);
        let names = names(program, &sem.scoping);
        let body = &program.body;
        let text = |span: Span| slice(&source, span);
        let start = body.iter().position(|s| {
            matches!(s, Statement::FunctionDeclaration(_))
                && text(s.span()).contains("return globalThis")
        });
        if let Some(start) = start {
            println!(
                "GETGLOBAL\n{}",
                canonical_text(&source, body[start].span(), &names).expect("getter")
            );
            let end = (start..body.len()).find(|i| {
                matches!(body[*i], Statement::FunctionDeclaration(_))
                    && text(body[*i].span()).contains("TextDecoder")
            });
            if let Some(end) = end {
                let span = Span::new(body[start].span().start, body[end].span().end);
                println!(
                    "PRELUDE {}\n{}",
                    end - start + 1,
                    canonical_text(&source, span, &names).expect("prelude")
                );
            }
        }
        if let Some(start) = body
            .iter()
            .position(|s| text(s.span()).starts_with("function MathImulPolyfill"))
        {
            if let Some(end) =
                (start..body.len()).find(|i| text(body[*i].span()).contains("fnObject"))
            {
                let span = Span::new(body[start].span().start, body[end].span().end);
                println!(
                    "INTEGRITY {}\n{}",
                    end - start + 1,
                    canonical_text(&source, span, &names).expect("integrity")
                );
            }
        }
        for (index, statement) in body.iter().enumerate() {
            let piece = text(statement.span());
            if matches!(statement, Statement::FunctionDeclaration(_))
                && (piece.contains("*91") || piece.contains("*0x5b"))
                && piece.len() < 4096
            {
                println!(
                    "DECODER\n{}",
                    decoder_shape(&source, statement.span(), &names)
                        .expect("decoder")
                        .0
                );
                println!(
                    "GETTER\n{}",
                    canonical_text(&source, body[index + 1].span(), &names).expect("getter")
                );
            }
            if matches!(statement, Statement::ExpressionStatement(_)) && piece.contains("RegExp(") {
                println!(
                    "SELFDEF\n{}",
                    canonical_text(&source, statement.span(), &names).expect("self-defending")
                );
            }
        }
    }
}
