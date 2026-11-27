use std::collections::BTreeMap;

use serde::Serialize;

#[cfg(feature = "library-imports")]
mod apply;
#[cfg(feature = "library-imports")]
pub use apply::apply;

pub const PACKAGE_JSON: &str = "package.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeptReason {
    SideEffects,
    Format,
    Ambiguous,
}

impl KeptReason {
    pub fn as_str(self) -> &'static str {
        match self {
            KeptReason::SideEffects => "side-effects",
            KeptReason::Format => "format",
            KeptReason::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Replaced {
    pub module: String,
    pub package: String,
    pub version: String,
    pub specifier: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub module: String,
    pub package: String,
    pub reason: KeptReason,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportsReport {
    pub replaced: Vec<Replaced>,
    pub kept: Vec<Kept>,
    pub pins: BTreeMap<String, String>,
}

impl ImportsReport {
    pub fn package_json(&self) -> Option<String> {
        if self.pins.is_empty() {
            return None;
        }
        let manifest = serde_json::json!({
            "private": true,
            "dependencies": self.pins,
        });
        serde_json::to_string_pretty(&manifest)
            .ok()
            .map(|text| text + "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_without_pins_writes_no_package_json() {
        assert_eq!(ImportsReport::default().package_json(), None);
        let report = ImportsReport {
            pins: BTreeMap::from([("ieee754".to_string(), "1.2.1".to_string())]),
            ..ImportsReport::default()
        };
        let text = report.package_json().expect("a manifest");
        assert!(text.contains("\"ieee754\": \"1.2.1\""), "{text}");
    }
}
