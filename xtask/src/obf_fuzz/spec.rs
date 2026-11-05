use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

pub const SPEC: &str = "spec.json";
pub const HAZARDS: [&str; 4] = [
    "observes-global-names",
    "patches-builtin",
    "text-from-data",
    "top-fn-prints-param",
];
const KEY_SHAPED: [&str; 3] = ["object-spread", "object-rest", "symbol-key"];

#[derive(Clone, Debug)]
pub struct Spec {
    pub raw: Value,
    pub dir: PathBuf,
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}

impl Spec {
    pub fn load(dir: &Path) -> Result<Spec> {
        let path = dir.join(SPEC);
        let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        let raw = serde_json::from_str(&text).with_context(|| path.display().to_string())?;
        Ok(Spec {
            raw,
            dir: dir.to_path_buf(),
        })
    }

    pub fn id(&self) -> &str {
        text(&self.raw, "id")
    }

    pub fn form(&self) -> &str {
        text(&self.raw, "form")
    }

    pub fn is_module(&self) -> bool {
        self.form() == "esm"
    }

    pub fn built(&self) -> Result<(), String> {
        if text(&self.raw, "status") == "ok" {
            Ok(())
        } else {
            Err(format!("not built: {}", text(&self.raw, "skip")))
        }
    }

    pub fn residue_only(&self) -> Option<&str> {
        text(&self.raw, "behaviour").strip_prefix("residue-only:")
    }

    pub fn features(&self) -> Vec<&str> {
        self.raw["features"]
            .as_array()
            .map(|all| all.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features().contains(&feature)
    }

    pub fn has_hazard(&self) -> bool {
        HAZARDS.iter().any(|h| self.has_feature(h))
    }

    pub fn key_shaped(&self) -> bool {
        KEY_SHAPED.iter().any(|f| self.has_feature(f))
    }

    pub fn domain_lock(&self) -> bool {
        self.raw["options"]["domainLock"]
            .as_array()
            .is_some_and(|hosts| !hosts.is_empty())
    }

    pub fn entry(&self) -> PathBuf {
        self.dir.join(text(&self.raw, "entry"))
    }

    pub fn sample(&self) -> PathBuf {
        self.dir.join(text(&self.raw, "sample"))
    }

    pub fn summary(&self) -> String {
        format!(
            "{} {} javascript-obfuscator@{} options {} features {}",
            self.id(),
            self.form(),
            text(&self.raw, "obfuscator"),
            self.raw["options"],
            self.features().join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_fields_read_from_the_generator_json() {
        let raw = serde_json::json!({
            "id": "s1-0", "form": "esm", "status": "ok", "behaviour": "residue-only:hostile",
            "features": ["object-rest", "patches-builtin"], "options": {"domainLock": ["a.com"]},
            "entry": "orig/index.mjs", "sample": "sample.mjs"
        });
        let spec = Spec {
            raw,
            dir: PathBuf::from("/w"),
        };
        assert!(spec.is_module() && spec.built().is_ok());
        assert_eq!(spec.residue_only(), Some("hostile"));
        assert!(spec.has_hazard() && spec.key_shaped() && spec.domain_lock());
        assert_eq!(spec.entry(), PathBuf::from("/w/orig/index.mjs"));
    }
}
