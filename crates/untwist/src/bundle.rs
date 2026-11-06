use serde::Serialize;
use untwist_core::print::pretty_codegen;
use untwist_core::reflow::{GuardRecord, GuardTally, Reflower};
use untwist_unpack::model::Bundle;

use crate::write::BUNDLE_JSON;

#[derive(Clone, Debug)]
pub struct FileOutput {
    pub path: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModuleOutput {
    pub id: String,
    pub path: String,
    #[serde(rename = "isEntry")]
    pub is_entry: bool,
    #[serde(skip)]
    pub code: String,
    #[serde(skip)]
    pub span: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BundleOutput {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "entryId")]
    pub entry_id: Option<String>,
    #[serde(rename = "chunkIds", skip_serializing_if = "Vec::is_empty")]
    pub chunk_ids: Vec<String>,
    pub modules: Vec<ModuleOutput>,
}

pub(crate) fn single_text_len<T: AsRef<str>>(codes: &[T]) -> Option<u32> {
    match codes {
        [only] => u32::try_from(only.as_ref().len()).ok(),
        _ => None,
    }
}

fn span_of(span: oxc_span::Span, text_len: Option<u32>) -> Option<(u32, u32)> {
    let limit = text_len?.min(untwist_core::ast_util::respan::SYNTHETIC_BASE);
    (span.start < span.end && span.end <= limit).then_some((span.start, span.end))
}

impl BundleOutput {
    pub(crate) fn unreflowed(bundle: &Bundle<'_>, text_len: Option<u32>) -> Self {
        Self {
            kind: bundle.kind.as_str().to_string(),
            entry_id: bundle.entry_id.clone(),
            chunk_ids: bundle.chunk_ids.clone(),
            modules: bundle
                .modules
                .values()
                .map(|module| ModuleOutput {
                    id: module.id.clone(),
                    path: module.path.as_str().to_string(),
                    is_entry: module.is_entry,
                    code: pretty_codegen(module.program()),
                    span: span_of(module.program().span, text_len),
                })
                .collect(),
        }
    }

    pub(crate) fn reflowed(self, earlier: &GuardRecord) -> (Self, GuardTally) {
        let mut found = GuardRecord::default();
        let mut reflower = Reflower::default();
        let modules = self
            .modules
            .into_iter()
            .map(|module| {
                let (code, record) = reflower.reflow_recorded(module.code);
                found.absorb(record);
                ModuleOutput { code, ..module }
            })
            .collect();
        (Self { modules, ..self }, found.tally_beyond(earlier))
    }

    pub fn entry(&self) -> Option<&ModuleOutput> {
        self.modules.iter().find(|module| module.is_entry)
    }

    pub fn bundle_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn files(&self) -> Vec<FileOutput> {
        let mut files = Vec::with_capacity(self.modules.len() + 1);
        files.push(FileOutput {
            path: BUNDLE_JSON.to_string(),
            code: self.bundle_json(),
        });
        for module in &self.modules {
            files.push(FileOutput {
                path: module.path.clone(),
                code: module.code.clone(),
            });
        }
        files
    }
}
