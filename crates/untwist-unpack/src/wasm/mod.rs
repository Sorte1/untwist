pub mod glue;
pub mod graph;
mod holders;
mod library;
pub mod module;
pub mod payload;
mod rename;
pub mod sites;

use oxc_allocator::Allocator;
use oxc_semantic::{Semantic, SemanticBuilder};
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::has_module_syntax;

pub use graph::{GlueImport, ImportValue, ModuleFacts};
pub use module::{
    summarize, ExportKind, Signature, ValueType, WasmExport, WasmImport, WasmSummary,
};
pub use oxc_span::Span;
pub use payload::{PayloadRefusal, MAX_WASM_BYTES};
pub use sites::{url_name, PayloadSite, RemoteSite, Scan};

const URL_MARKER: &str = ".wasm";
const BYTE_MAGICS: [&str; 2] = ["0, 97, 115, 109", "0,97,115,109"];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rewritten {
    pub text: String,
    pub applied: Vec<(String, String)>,
    pub mismatch: Option<String>,
    pub copies: Vec<Result<usize, String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CopiedRenames {
    pub functions: Vec<(Span, Vec<(String, String)>)>,
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
    holds_candidate(text, payload::BASE64_MAGIC, |at| {
        let run = &text[at..];
        let len = run
            .bytes()
            .take_while(|byte| payload::is_base64_byte(*byte))
            .count();
        let body = &run[..len];
        let found = payload::base64_holds_module(body)
            && payload::decode_base64(body).map_or(true, |bytes| loadable(&bytes));
        (found, at + len)
    })
}

fn holds_byte_module(text: &str) -> bool {
    BYTE_MAGICS.iter().any(|magic| {
        holds_candidate(text, magic, |at| {
            let start = at + magic.len();
            let (bytes, used) = listed_bytes(&text[start..]);
            (bytes.is_some_and(|bytes| loadable(&bytes)), start + used)
        })
    })
}

fn holds_candidate(text: &str, magic: &str, mut judge: impl FnMut(usize) -> (bool, usize)) -> bool {
    let mut from = 0;
    while let Some(found) = text.get(from..).and_then(|rest| rest.find(magic)) {
        if untwist_core::budget::check().is_err() {
            return true;
        }
        let (holds, end) = judge(from + found);
        if holds {
            return true;
        }
        from = end.max(from + found + magic.len());
    }
    false
}

fn listed_bytes(text: &str) -> (Option<Vec<u8>>, usize) {
    let mut bytes = payload::WASM_MAGIC.to_vec();
    let mut rest = text;
    let used = |rest: &str| text.len() - rest.len();
    loop {
        let trimmed = rest.trim_start();
        if trimmed.starts_with(']') {
            return (Some(bytes), used(trimmed));
        }
        let Some(after) = trimmed.strip_prefix(',').map(str::trim_start) else {
            return (None, used(trimmed));
        };
        let len = after
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'.')
            .count();
        let Some(byte) = byte_literal(&after[..len]) else {
            return (None, used(after));
        };
        bytes.push(byte);
        rest = &after[len..];
        if bytes.len() > MAX_WASM_BYTES {
            return (Some(bytes), used(rest));
        }
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
    script: bool,
}

impl<'a> Glue<'a> {
    pub fn parse(alloc: &'a Allocator, text: &'a str) -> Option<Self> {
        let parsed = parse_program(alloc, text);
        if has_error(&parsed.diagnostics) {
            return None;
        }
        let program = alloc.alloc(parsed.program);
        let script = !has_module_syntax(program);
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(program)
            .semantic;
        Some(Self {
            text,
            semantic,
            script,
        })
    }

    pub fn in_module_file(self) -> Self {
        Self {
            script: false,
            ..self
        }
    }

    pub fn scan(&self) -> Scan {
        Scan::of(self.semantic.nodes(), self.semantic.scoping())
    }

    pub fn rewrite(
        &self,
        replacements: &[(Span, String)],
        module: Option<&WasmSummary>,
        copies: &[CopiedRenames],
    ) -> Rewritten {
        let mut claimed = rename::Claimed::default();
        let naming = module.map_or_else(glue::Naming::default, |summary| {
            glue::name_glue(&self.semantic, self.script, summary, &mut claimed)
        });
        let mut edits = replacements.to_vec();
        edits.extend(naming.edits);
        let copies = copies
            .iter()
            .map(|copy| self.copy_renames(copy, &mut claimed, &mut edits))
            .collect();
        Rewritten {
            text: splice(self.text, edits),
            applied: naming.applied,
            mismatch: naming.mismatch,
            copies,
        }
    }

    fn copy_renames(
        &self,
        copy: &CopiedRenames,
        claimed: &mut rename::Claimed,
        edits: &mut Vec<(Span, String)>,
    ) -> Result<usize, String> {
        let renamer = rename::Renamer::new(&self.semantic, self.script);
        let planned = copy
            .functions
            .iter()
            .map(|(body, pairs)| renamer.plan_within(*body, pairs, claimed))
            .collect::<Result<Vec<_>, String>>()?;
        Ok(planned
            .into_iter()
            .map(|plan| renamer.commit_within(plan, claimed, edits))
            .sum())
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

pub fn rename_names(
    text: &str,
    wanted: &[(String, String)],
) -> Option<(String, Vec<(String, String)>)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let (edits, applied) = rename::Renamer::new(&semantic, false).apply_by_name(wanted);
    Some((splice(text, edits), applied))
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
