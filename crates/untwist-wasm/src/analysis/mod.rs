mod build;
mod collect;
mod complete;
mod index;
mod insight;
mod offsets;
mod rename;

#[cfg(test)]
mod insight_tests;
#[cfg(test)]
mod tests;

use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

use index::{Index, NONE};
pub use rename::Outcome;

pub const ANALYSIS_CAP_BYTES: u32 = 32 << 20;

#[wasm_bindgen]
pub struct Analysis {
    index: Option<Index>,
}

#[derive(Serialize)]
struct Definition<'a> {
    span: [u32; 2],
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    decoded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a [Box<str>]>,
}

#[wasm_bindgen]
pub fn analyze(text: &str) -> Analysis {
    let fits = u32::try_from(text.len()).is_ok_and(|len| len <= ANALYSIS_CAP_BYTES);
    Analysis {
        index: fits.then(|| build::build(text)),
    }
}

#[wasm_bindgen]
pub fn analysis_cap_bytes() -> u32 {
    ANALYSIS_CAP_BYTES
}

#[derive(Serialize)]
struct Described<'a> {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    decoded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a [Box<str>]>,
    properties: Vec<&'a str>,
}

fn sorted(mut list: Vec<&str>) -> Vec<&str> {
    list.sort_unstable();
    list.dedup();
    list
}

impl Analysis {
    fn described(&self, pos: u32, name: &str) -> Option<Described<'_>> {
        let index = self.index.as_ref()?;
        let Some(symbol) = index.resolve(pos, name) else {
            let object = index.names.find(name)?;
            return Some(Described {
                kind: "global",
                form: None,
                value: None,
                decoded: false,
                params: None,
                properties: sorted(index.properties_of(NONE, object).collect()),
            });
        };
        let detail = index.detail(symbol);
        Some(Described {
            kind: index
                .symbol(symbol)
                .map_or("binding", |entry| entry.kind.label()),
            form: detail.and_then(|detail| detail.form.label(&index.names)),
            value: detail.and_then(|detail| detail.value.as_deref()),
            decoded: detail.is_some_and(|detail| detail.decoded),
            params: detail.and_then(|detail| detail.params.as_deref()),
            properties: sorted(index.properties_of(symbol, NONE).collect()),
        })
    }

    fn definition_of(&self, pos: u32) -> Option<Definition<'_>> {
        let index = self.index.as_ref()?;
        if let Some(symbol) = index
            .occurrence_at(pos)
            .and_then(|occ| index.symbol(occ.symbol).map(|symbol| (occ.symbol, symbol)))
        {
            let (id, symbol) = symbol;
            let detail = index.detail(id);
            return Some(Definition {
                span: symbol.decl,
                kind: symbol.kind.label(),
                module: symbol.link.as_ref().map(|link| link.module.as_ref()),
                name: symbol.link.as_ref().map(|link| link.name.as_ref()),
                form: detail.and_then(|detail| detail.form.label(&index.names)),
                value: detail.and_then(|detail| detail.value.as_deref()),
                decoded: detail.is_some_and(|detail| detail.decoded),
                params: detail.and_then(|detail| detail.params.as_deref()),
            });
        }
        let member = index.member_at(pos)?;
        let object = index.symbol(member.object)?;
        let link = object.link.as_ref()?;
        Some(Definition {
            span: object.decl,
            kind: "member",
            module: Some(link.module.as_ref()),
            name: Some(member.name.as_ref()),
            form: None,
            value: None,
            decoded: false,
            params: None,
        })
    }

    pub fn rename_outcome(&self, pos: u32, name: &str) -> Outcome {
        match &self.index {
            Some(index) => rename::rename(index, pos, name),
            None => Outcome::Refused("language features are off for this text".into()),
        }
    }
}

#[wasm_bindgen]
impl Analysis {
    pub fn available(&self) -> bool {
        self.index.is_some()
    }

    pub fn definition(&self, pos: u32) -> Option<String> {
        serde_json::to_string(&self.definition_of(pos)?).ok()
    }

    pub fn references(&self, pos: u32) -> Vec<u32> {
        let Some(index) = &self.index else {
            return Vec::new();
        };
        let Some(occ) = index
            .occurrence_at(pos)
            .filter(|occ| index.symbol(occ.symbol).is_some())
        else {
            return Vec::new();
        };
        index
            .occurrences_of(occ.symbol)
            .flat_map(|occ| [occ.start, occ.end])
            .collect()
    }

    pub fn rename(&self, pos: u32, name: &str) -> String {
        serde_json::to_string(&self.rename_outcome(pos, name)).unwrap_or_default()
    }

    pub fn completions(&self, pos: u32) -> String {
        let list = self
            .index
            .as_ref()
            .map(|index| complete::completions(index, pos))
            .unwrap_or_default();
        serde_json::to_string(&list).unwrap_or_default()
    }

    pub fn describe(&self, pos: u32, name: &str) -> Option<String> {
        serde_json::to_string(&self.described(pos, name)?).ok()
    }

    pub fn problems_json(&self) -> String {
        let problems = self.index.as_ref().map(|index| index.problems.as_slice());
        serde_json::to_string(&problems.unwrap_or_default()).unwrap_or_default()
    }

    pub fn export_span(&self, name: &str) -> Vec<u32> {
        self.index
            .as_ref()
            .and_then(|index| {
                index
                    .exports
                    .iter()
                    .find(|(export, _)| export.as_ref() == name)
            })
            .map(|(_, span)| span.to_vec())
            .unwrap_or_default()
    }
}
