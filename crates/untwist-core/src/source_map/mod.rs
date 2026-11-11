mod align;
mod coverage;
mod fill;
mod inline;
mod lines;
pub mod origins;
mod through;

use std::borrow::Cow;

use oxc_ast::ast::Program;
pub use oxc_sourcemap::{OwnedSourceMap, SourceMap, Token};

pub use align::retarget;
pub use coverage::{coverage, Coverage};
pub use fill::{fill_spans, fill_window};
pub use inline::{decode_base64, inline_map, last_map_url};
pub use lines::Positions;
pub use origins::{note_replaced, Origins, Recording};
pub use through::Through;

use crate::print::mapped_codegen;

pub const SOURCE_NAME: &str = "input.js";

pub struct Mapped {
    pub printed: String,
    pub map: OwnedSourceMap,
}

pub fn mapped(program: &mut Program<'_>, origins: &Origins) -> Option<Mapped> {
    fill_spans(program, origins);
    printed_with_map(program)
}

pub fn mapped_window(program: &mut Program<'_>, origins: &Origins) -> Option<(Mapped, usize)> {
    let start = fill_window(program, origins);
    printed_with_map(program).map(|mapped| (mapped, start))
}

fn printed_with_map(program: &Program<'_>) -> Option<Mapped> {
    let (printed, map) = mapped_codegen(program, SOURCE_NAME);
    map.map(|map| Mapped {
        printed,
        map: with_identifier_names(map),
    })
}

fn is_identifier_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '#'))
}

fn with_identifier_names(map: OwnedSourceMap) -> OwnedSourceMap {
    let mut parts = map.into_parts();
    let mut ids: Vec<Option<u32>> = Vec::with_capacity(parts.names.len());
    let mut names = Vec::new();
    for name in parts.names {
        ids.push(match is_identifier_name(&name) {
            true => u32::try_from(names.len()).ok(),
            false => None,
        });
        if is_identifier_name(&name) {
            names.push(name);
        }
    }
    for token in parts.tokens.iter_mut() {
        let Some(name) = token.get_name_id() else {
            continue;
        };
        *token = Token::new(
            token.get_dst_line(),
            token.get_dst_col(),
            token.get_src_line(),
            token.get_src_col(),
            token.get_source_id(),
            ids.get(name as usize).copied().flatten(),
        );
    }
    parts.names = names;
    parts.token_chunks = None;
    OwnedSourceMap::from_parts(parts)
}

pub fn unmapped(source: &str) -> OwnedSourceMap {
    from_tokens(source, Vec::new())
}

pub fn identity(text: &str) -> OwnedSourceMap {
    let starts = lines::line_starts(text.as_bytes());
    let ends = starts.iter().skip(1).copied().chain([text.len()]);
    let tokens = starts
        .iter()
        .zip(ends)
        .enumerate()
        .filter(|(_, (start, end))| {
            text.get(**start..*end)
                .is_some_and(|row| !row.trim().is_empty())
        })
        .filter_map(|(line, _)| u32::try_from(line).ok())
        .map(|line| Token::new(line, 0, line, 0, Some(0), None))
        .collect();
    from_tokens(text, tokens)
}

fn from_tokens(source: &str, tokens: Vec<Token>) -> OwnedSourceMap {
    SourceMap::new(
        None,
        Vec::new(),
        None,
        vec![Cow::Borrowed(SOURCE_NAME)],
        vec![Some(Cow::Owned(source.to_string()))],
        tokens.into_boxed_slice(),
        None,
    )
    .into_owned_sourcemap()
}

pub const INPUT_MAP_SCHEME: &str = "input-map:///";

pub fn compose(map: OwnedSourceMap, input: OwnedSourceMap) -> OwnedSourceMap {
    if map.get_sources().count() > 1 {
        return map;
    }
    let mut parts = map.compose(input).into_parts();
    let root = parts.source_root.take();
    parts.sources = parts
        .sources
        .iter()
        .map(|source| Cow::Owned(as_data(&rooted(root.as_deref(), source))))
        .collect();
    parts.token_chunks = None;
    OwnedSourceMap::from_parts(parts)
}

fn rooted(root: Option<&str>, source: &str) -> String {
    match root.filter(|root| !root.is_empty()) {
        Some(root) if root.ends_with('/') => format!("{root}{source}"),
        Some(root) => format!("{root}/{source}"),
        None => source.to_string(),
    }
}

fn is_input_name(name: &str) -> bool {
    let digits = name
        .strip_prefix("input")
        .and_then(|rest| rest.strip_suffix(".js"));
    name == SOURCE_NAME
        || digits
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn is_plain_relative(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['/', '\\'])
        && !name.contains(':')
        && !name.chars().any(char::is_control)
        && !name.split(['/', '\\']).any(|part| part == "..")
}

fn as_data(name: &str) -> String {
    match is_plain_relative(name) && !is_input_name(name) {
        true => name.to_string(),
        false => format!("{INPUT_MAP_SCHEME}{}", name.trim_start_matches(['/', '\\'])),
    }
}

pub fn input_name(index: usize, total: usize) -> String {
    match total {
        0 | 1 => SOURCE_NAME.to_string(),
        _ => format!("input{}.js", index + 1),
    }
}

pub fn input_index(name: &str, total: usize) -> Option<usize> {
    (0..total.max(1)).find(|index| input_name(*index, total) == name)
}

pub fn parse(json: &str) -> Option<OwnedSourceMap> {
    OwnedSourceMap::from_json_string(json).ok()
}

pub fn renamed(mut map: OwnedSourceMap, name: &str) -> OwnedSourceMap {
    if map.get_sources().count() == 1 && map.get_source(0) == Some(SOURCE_NAME) {
        map.set_sources([name]);
    }
    map
}

pub fn named(mut map: OwnedSourceMap, file: Option<&str>, source: &str) -> OwnedSourceMap {
    if let Some(file) = file {
        map.set_file(file);
    }
    if map.get_sources().count() == 1 && map.get_source(0) == Some(SOURCE_NAME) {
        map.set_sources([source]);
    }
    map
}
