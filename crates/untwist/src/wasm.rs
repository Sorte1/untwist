use oxc_allocator::Allocator;
use serde::Serialize;
use untwist_core::pass::{Note, NoteKind};
pub use untwist_unpack::wasm::MAX_WASM_BYTES;
use untwist_unpack::wasm::{self, Glue, PayloadRefusal, Scan, Span, WasmSummary};

use crate::pipeline::{admits_reparse, Output};
use crate::write::DEOBFUSCATED;

const PASS: &str = "wasm";
const WASM_DIR: &str = "wasm";
const HASHED_SUFFIX: &str = ".module.wasm";
const MAX_SUFFIX: usize = 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WasmCheck {
    #[default]
    Off,
    On,
    Auto,
}

impl WasmCheck {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "off" => Some(Self::Off),
            "on" => Some(Self::On),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    fn admits(self, text: &str) -> bool {
        match self {
            Self::Off => wasm::may_reference_wasm(text),
            Self::On => true,
            Self::Auto => wasm::may_hold_wasm(text),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WasmOrigin {
    Embedded,
    Supplied,
    Fetched,
}

impl WasmOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::Supplied => "supplied",
            Self::Fetched => "fetched",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WasmFile {
    pub path: String,
    pub origin: WasmOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    pub imports: usize,
    pub exports: usize,
    pub renamed: usize,
    #[serde(skip)]
    pub bytes: Vec<u8>,
    #[serde(skip)]
    pub summary: WasmSummary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WasmReference {
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct SuppliedWasm {
    pub name: String,
    pub bytes: Vec<u8>,
    pub origin: WasmOrigin,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct WasmSupply {
    pub files: Vec<SuppliedWasm>,
}

const MIN_HASH_SEGMENT: usize = 6;

fn unhashed(name: &str) -> String {
    name.split('.')
        .filter(|segment| {
            segment.len() < MIN_HASH_SEGMENT
                || !segment.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn names_a_module(unhashed: &str) -> bool {
    unhashed.ends_with(".wasm")
        && unhashed
            .split('.')
            .any(|segment| !matches!(segment, "" | "wasm" | "module"))
}

impl WasmSupply {
    pub fn find(&self, name: &str) -> Option<&SuppliedWasm> {
        if let Some(exact) = self.files.iter().find(|file| file.name == name) {
            return Some(exact);
        }
        let wanted = unhashed(name);
        let mut alike = self
            .files
            .iter()
            .filter(|file| unhashed(&file.name) == wanted);
        match (alike.next(), alike.next()) {
            (Some(only), None) if names_a_module(&wanted) => Some(only),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Host {
    Code(usize),
    Module(usize),
}

struct Text {
    host: Host,
    path: String,
    scan: Scan,
    files: Vec<usize>,
    replacements: Vec<(Span, String)>,
}

impl Text {
    fn new(host: Host, path: String, scan: Scan) -> Self {
        Self {
            host,
            path,
            scan,
            files: Vec::new(),
            replacements: Vec::new(),
        }
    }
}

fn note(kind: NoteKind, message: String) -> Note {
    Note {
        pass: PASS,
        kind,
        message,
        span: None,
    }
}

fn safe_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(
            |ch| match ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                true => ch,
                false => '_',
            },
        )
        .collect();
    let stem = stem.trim_start_matches('.');
    match stem.is_empty() {
        true => "module".to_string(),
        false => stem.to_string(),
    }
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.split_once('.').map_or(name, |(stem, _)| stem)
}

fn relative_to(host: &str, target: &str) -> String {
    match host.matches('/').count() {
        0 => format!("./{target}"),
        depth => format!("{}{target}", "../".repeat(depth)),
    }
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

impl Output {
    fn wasm_hosts(&self) -> Vec<(Host, String)> {
        let mut hosts = Vec::new();
        if let Some(bundle) = &self.bundle {
            for (index, module) in bundle.modules.iter().enumerate() {
                let path = match self.single_file {
                    true => DEOBFUSCATED.to_string(),
                    false => module.path.clone(),
                };
                hosts.push((Host::Module(index), path));
            }
        }
        for index in 0..self.codes.len() {
            hosts.push((Host::Code(index), DEOBFUSCATED.to_string()));
        }
        hosts
    }

    fn host_text(&self, host: Host) -> &str {
        match host {
            Host::Code(index) => &self.codes[index],
            Host::Module(index) => match &self.bundle {
                Some(bundle) => &bundle.modules[index].code,
                None => "",
            },
        }
    }

    fn host_text_mut(&mut self, host: Host) -> Option<&mut String> {
        match host {
            Host::Code(index) => self.codes.get_mut(index),
            Host::Module(index) => Some(&mut self.bundle.as_mut()?.modules.get_mut(index)?.code),
        }
    }

    fn wasm_candidates(&self, check: WasmCheck) -> Vec<(Host, String)> {
        self.wasm_hosts()
            .into_iter()
            .filter(|(host, _)| {
                let text = self.host_text(*host);
                check.admits(text) && admits_reparse(text)
            })
            .collect()
    }

    fn scanned_texts(&self) -> Vec<Text> {
        self.wasm_candidates(WasmCheck::Off)
            .into_iter()
            .filter_map(|(host, path)| {
                Some(Text::new(host, path, wasm::scan(self.host_text(host))?))
            })
            .collect()
    }

    fn may_embed_wasm(&self) -> bool {
        self.wasm_hosts()
            .into_iter()
            .any(|(host, _)| wasm::may_embed_wasm(self.host_text(host)))
    }

    pub fn wasm_references(&self) -> Vec<WasmReference> {
        references_of(&self.scanned_texts())
    }

    pub fn settle_wasm(&mut self, supply: &WasmSupply, extract: bool) {
        let check = self.wasm_check;
        self.wasm_unchecked = check == WasmCheck::Off && self.may_embed_wasm();
        let candidates = self.wasm_candidates(check);
        if candidates.is_empty() {
            return;
        }
        let sources: Vec<String> = candidates
            .iter()
            .map(|(host, _)| self.host_text(*host).to_string())
            .collect();
        let allocs: Vec<Allocator> = sources.iter().map(|_| Allocator::default()).collect();
        let (mut texts, glues): (Vec<Text>, Vec<Glue<'_>>) = candidates
            .into_iter()
            .zip(sources.iter().zip(&allocs))
            .filter_map(|((host, path), (source, alloc))| {
                let glue = Glue::parse(alloc, source)?;
                let mut scan = glue.scan();
                if check == WasmCheck::Off {
                    scan.payloads.clear();
                }
                Some((Text::new(host, path, scan), glue))
            })
            .unzip();
        let mut notes = Vec::new();
        for text in &mut texts {
            self.take_payloads(text, extract, &mut notes);
        }
        let references = references_of(&texts);
        let mut resolved = std::collections::HashMap::new();
        for reference in &references {
            if let Some(index) = self.resolve_reference(reference, supply, &mut texts, &mut notes) {
                resolved.insert(reference.name.clone(), index);
            }
        }
        for (text, glue) in texts.iter().zip(&glues) {
            self.rename_glue(text, glue, &mut notes);
        }
        self.name_bundle_glue(&resolved, &mut notes);
        self.notes.extend(notes);
    }

    fn file_named(&self, wanted: &str) -> String {
        let taken = |path: &str| self.wasm.iter().any(|file| file.path == path);
        let first = format!("{WASM_DIR}/{wanted}");
        if !taken(&first) {
            return first;
        }
        let (stem, extension) = wanted.rsplit_once('.').unwrap_or((wanted, "wasm"));
        (2..=MAX_SUFFIX)
            .map(|suffix| format!("{WASM_DIR}/{stem}-{suffix}.{extension}"))
            .find(|path| !taken(path))
            .unwrap_or(first)
    }

    fn add_file(
        &mut self,
        wanted: &str,
        bytes: Vec<u8>,
        summary: WasmSummary,
        origin: (WasmOrigin, Option<String>),
    ) -> usize {
        if let Some(index) = self.wasm.iter().position(|file| file.bytes == bytes) {
            return index;
        }
        let path = self.file_named(wanted);
        self.wasm.push(WasmFile {
            path,
            origin: origin.0,
            url: origin.1,
            byte_length: bytes.len(),
            imports: summary.imports.len(),
            exports: summary.exports.len(),
            renamed: 0,
            bytes,
            summary,
        });
        self.wasm.len() - 1
    }

    fn take_payloads(&mut self, text: &mut Text, extract: bool, notes: &mut Vec<Note>) {
        for site in std::mem::take(&mut text.scan.payloads) {
            let checked = site
                .bytes
                .and_then(|bytes| wasm::summarize(&bytes).map(|summary| (bytes, summary)));
            let (bytes, summary) = match checked {
                Ok(checked) => checked,
                Err(refusal) => {
                    notes.push(refused_payload(&text.path, &refusal));
                    continue;
                }
            };
            if self.wasm_check == WasmCheck::Auto && summary.is_inert() {
                continue;
            }
            let wanted = format!("{}.wasm", safe_stem(file_stem(&text.path)));
            let index = self.add_file(&wanted, bytes, summary, (WasmOrigin::Embedded, None));
            text.files.push(index);
            if extract {
                let reference = relative_to(&text.path, &self.wasm[index].path);
                text.replacements.push((site.span, quoted(&reference)));
            }
        }
    }

    fn resolve_reference(
        &mut self,
        reference: &WasmReference,
        supply: &WasmSupply,
        texts: &mut [Text],
        notes: &mut Vec<Note>,
    ) -> Option<usize> {
        let Some(supplied) = supply.find(&reference.name) else {
            self.wasm_unresolved.push(reference.clone());
            return None;
        };
        let summary = match wasm::summarize(&supplied.bytes) {
            Ok(summary) => summary,
            Err(refusal) => {
                notes.push(note(
                    NoteKind::Refusal,
                    format!(
                        "wasm file supplied for {} {refusal}; it is not used",
                        reference.name
                    ),
                ));
                self.wasm_unresolved.push(reference.clone());
                return None;
            }
        };
        let origin = (supplied.origin, supplied.url.clone());
        let index = self.add_file(
            &safe_stem(&reference.name),
            supplied.bytes.clone(),
            summary,
            origin,
        );
        for text in texts.iter_mut() {
            if text_mentions(text, reference) {
                text.files.push(index);
            }
        }
        Some(index)
    }

    fn rename_glue(&mut self, text: &Text, glue: &Glue<'_>, notes: &mut Vec<Note>) {
        let mut files = text.files.clone();
        files.sort_unstable();
        files.dedup();
        let module = match files.as_slice() {
            [only] => Some(*only),
            [] => None,
            _ => {
                notes.push(note(
                    NoteKind::Progress,
                    format!(
                        "{} loads {} wasm modules; its glue is left unnamed",
                        text.path,
                        files.len()
                    ),
                ));
                None
            }
        };
        if module.is_none() && text.replacements.is_empty() {
            return;
        }
        let summary = module.map(|index| &self.wasm[index].summary);
        let rewritten = glue.rewrite(&text.replacements, summary);
        if let (Some(index), Some(mismatch)) = (module, &rewritten.mismatch) {
            notes.push(note(
                NoteKind::Refusal,
                format!(
                    "{} does not match the glue in {}: {mismatch}; no names were taken from it",
                    self.wasm[index].path, text.path
                ),
            ));
        }
        if let Some(index) = module {
            self.wasm[index].renamed += rewritten.renamed;
        }
        if let Some(slot) = self.host_text_mut(text.host) {
            *slot = rewritten.text;
        }
    }
}

fn refused_payload(path: &str, refusal: &PayloadRefusal) -> Note {
    note(
        NoteKind::Refusal,
        format!("embedded wasm payload in {path} {refusal}; the literal is left as it was"),
    )
}

fn text_mentions(text: &Text, reference: &WasmReference) -> bool {
    text.scan
        .remotes
        .iter()
        .any(|remote| remote.name == reference.name)
        || reference
            .name
            .strip_suffix(HASHED_SUFFIX)
            .is_some_and(|hash| text.scan.hashes.iter().any(|seen| seen == hash))
}

fn references_of(texts: &[Text]) -> Vec<WasmReference> {
    let hashed = texts.iter().any(|text| text.scan.hashed_suffix);
    let mut references: Vec<WasmReference> = Vec::new();
    let mut push = |name: String, url: String| {
        if !references.iter().any(|known| known.name == name) {
            references.push(WasmReference { name, url });
        }
    };
    for text in texts {
        for remote in &text.scan.remotes {
            push(remote.name.clone(), remote.url.clone());
        }
        if hashed {
            for hash in &text.scan.hashes {
                let name = format!("{hash}{HASHED_SUFFIX}");
                push(name.clone(), name);
            }
        }
    }
    references
}
