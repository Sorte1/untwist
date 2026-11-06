pub mod glue;
pub mod graph;
pub mod module;
pub mod payload;
mod rename;
pub mod sites;

use oxc_allocator::Allocator;
use oxc_semantic::SemanticBuilder;
use untwist_core::parse::{has_error, parse_program};

pub use graph::{GlueImport, ImportValue, ModuleFacts};
pub use module::{summarize, WasmExport, WasmImport, WasmSummary};
pub use oxc_span::Span;
pub use payload::{PayloadRefusal, MAX_WASM_BYTES};
pub use sites::{url_name, PayloadSite, RemoteSite, Scan};

const MARKERS: &[&str] = &[
    payload::BASE64_MAGIC,
    ".wasm",
    "0, 97, 115, 109",
    "0,97,115,109",
];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rewritten {
    pub text: String,
    pub renamed: usize,
    pub mismatch: Option<String>,
}

pub fn may_hold_wasm(text: &str) -> bool {
    MARKERS.iter().any(|marker| text.contains(marker))
}

pub fn scan(text: &str) -> Option<Scan> {
    if !may_hold_wasm(text) {
        return None;
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    Some(Scan::of(semantic.nodes()))
}

pub fn rewrite(
    text: &str,
    replacements: &[(Span, String)],
    module: Option<&WasmSummary>,
) -> Option<Rewritten> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let naming = match module {
        Some(summary) => {
            let semantic = SemanticBuilder::new()
                .with_build_nodes(true)
                .build(&parsed.program)
                .semantic;
            glue::name_glue(&semantic, summary)
        }
        None => glue::Naming::default(),
    };
    let mut edits = replacements.to_vec();
    edits.extend(naming.edits);
    Some(Rewritten {
        text: splice(text, edits),
        renamed: naming.renamed,
        mismatch: naming.mismatch,
    })
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
