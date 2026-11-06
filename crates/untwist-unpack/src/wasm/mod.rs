pub mod glue;
pub mod graph;
pub mod module;
pub mod payload;
mod rename;
pub mod sites;

use oxc_allocator::Allocator;
use oxc_semantic::{Semantic, SemanticBuilder};
use untwist_core::parse::{has_error, parse_program};

pub use graph::{GlueImport, ImportValue, ModuleFacts};
pub use module::{summarize, WasmExport, WasmImport, WasmSummary};
pub use oxc_span::Span;
pub use payload::{PayloadRefusal, MAX_WASM_BYTES};
pub use sites::{url_name, PayloadSite, RemoteSite, Scan};

const URL_MARKER: &str = ".wasm";
const BYTE_MAGICS: [&str; 2] = ["0, 97, 115, 109", "0,97,115,109"];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rewritten {
    pub text: String,
    pub renamed: usize,
    pub mismatch: Option<String>,
}

pub fn may_hold_wasm(text: &str) -> bool {
    may_reference_wasm(text) || may_embed_wasm(text)
}

pub fn may_reference_wasm(text: &str) -> bool {
    text.contains(URL_MARKER)
}

pub fn may_embed_wasm(text: &str) -> bool {
    holds_base64_module(text) || holds_byte_module(text)
}

fn holds_base64_module(text: &str) -> bool {
    text.match_indices(payload::BASE64_MAGIC).any(|(at, _)| {
        let run = &text[at..];
        let len = run
            .bytes()
            .take_while(|byte| payload::is_base64_byte(*byte))
            .count();
        let body = &run[..len];
        payload::base64_holds_module(body)
            && payload::decode_base64(body).map_or(true, |bytes| loadable(&bytes))
    })
}

fn holds_byte_module(text: &str) -> bool {
    BYTE_MAGICS.iter().any(|magic| {
        text.match_indices(magic).any(|(at, _)| {
            listed_bytes(&text[at + magic.len()..]).is_some_and(|bytes| loadable(&bytes))
        })
    })
}

fn listed_bytes(mut rest: &str) -> Option<Vec<u8>> {
    let mut bytes = payload::WASM_MAGIC.to_vec();
    loop {
        let trimmed = rest.trim_start();
        if trimmed.starts_with(']') {
            return Some(bytes);
        }
        let after = trimmed.strip_prefix(',')?.trim_start();
        let len = after
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'.')
            .count();
        bytes.push(byte_literal(&after[..len])?);
        if bytes.len() > MAX_WASM_BYTES {
            return Some(bytes);
        }
        rest = &after[len..];
    }
}

fn byte_literal(token: &str) -> Option<u8> {
    match token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        Some(hex) => u8::from_str_radix(hex, 16).ok(),
        None => token.parse().ok(),
    }
}

fn loadable(bytes: &[u8]) -> bool {
    bytes.len() >= payload::MIN_MODULE_BYTES
        && summarize(bytes).map_or(true, |summary| !summary.is_inert())
}

pub struct Glue<'a> {
    text: &'a str,
    semantic: Semantic<'a>,
}

impl<'a> Glue<'a> {
    pub fn parse(alloc: &'a Allocator, text: &'a str) -> Option<Self> {
        let parsed = parse_program(alloc, text);
        if has_error(&parsed.diagnostics) {
            return None;
        }
        let program = alloc.alloc(parsed.program);
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(program)
            .semantic;
        Some(Self { text, semantic })
    }

    pub fn scan(&self) -> Scan {
        Scan::of(self.semantic.nodes())
    }

    pub fn rewrite(
        &self,
        replacements: &[(Span, String)],
        module: Option<&WasmSummary>,
    ) -> Rewritten {
        let naming = module.map_or_else(glue::Naming::default, |summary| {
            glue::name_glue(&self.semantic, summary)
        });
        let mut edits = replacements.to_vec();
        edits.extend(naming.edits);
        Rewritten {
            text: splice(self.text, edits),
            renamed: naming.renamed,
            mismatch: naming.mismatch,
        }
    }
}

pub fn scan(text: &str) -> Option<Scan> {
    if !may_hold_wasm(text) {
        return None;
    }
    let alloc = Allocator::default();
    Some(Glue::parse(&alloc, text)?.scan())
}

pub fn module_facts(text: &str) -> Option<ModuleFacts> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    Some(graph::facts_of(&semantic))
}

pub fn rename_names(text: &str, wanted: &[(String, String)]) -> Option<(String, usize)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let (edits, renamed) = rename::Renamer::new(&semantic).apply_by_name(wanted);
    Some((splice(text, edits), renamed))
}

fn splice(text: &str, mut edits: Vec<(Span, String)>) -> String {
    edits.sort_by_key(|(span, _)| (span.start, span.end));
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    for (span, replacement) in edits {
        let (start, end) = (span.start as usize, span.end as usize);
        if start < at || end > text.len() {
            continue;
        }
        out.push_str(&text[at..start]);
        out.push_str(&replacement);
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_only_feature_detect_does_not_trip_the_trigger() {
        assert!(!may_hold_wasm(
            "new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])"
        ));
        assert!(!may_hold_wasm("new Uint8Array([0,97,115,109,1,0,0,0,1])"));
        assert!(!may_hold_wasm("atob(\"AGFzbQEAAAA=\")"));
        assert!(!may_hold_wasm("[0, 97, 115, 109, 1, 0, 0, 0, x, 1]"));
    }

    #[test]
    fn a_module_with_no_imports_or_exports_does_not_trip_the_trigger() {
        assert!(!may_hold_wasm(
            "WebAssembly.validate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0]))"
        ));
        assert!(!may_hold_wasm("atob(\"AGFzbQEAAAABBAFgAAA=\")"));
    }

    #[test]
    fn a_byte_list_with_an_element_that_is_not_a_literal_does_not_trip_the_trigger() {
        assert!(!may_hold_wasm(
            "[0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, x]"
        ));
        assert!(!may_hold_wasm(
            "[0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, ...rest]"
        ));
    }

    #[test]
    fn a_header_with_a_section_trips_the_trigger() {
        assert!(may_hold_wasm(
            "new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 4])"
        ));
        assert!(may_hold_wasm("[0,97,115,109,1,0,0,0,0x1,7,9]"));
        assert!(may_hold_wasm("\"AGFzbQEAAAABBw==\""));
        assert!(may_hold_wasm("fetch(\"app.wasm\")"));
    }
}
