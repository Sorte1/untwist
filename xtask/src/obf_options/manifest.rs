use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub const DIR: &str = "fixtures/obfuscator-io";
pub const NODE: &str = "node";
pub const RESIDUE_ONLY: &str = "residue-only:";

#[derive(Clone, Debug)]
pub struct Entry {
    pub file: String,
    pub cell: String,
    pub source: String,
    pub seed: u64,
    pub obfuscator: String,
    pub behaviour: String,
    pub options: String,
    pub sha256: String,
}

impl Entry {
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(DIR).join(&self.file)
    }

    pub fn program(&self) -> &str {
        Path::new(&self.source)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
    }

    pub fn expected_stdout(&self, root: &Path) -> PathBuf {
        root.join(DIR)
            .join("expected")
            .join(format!("{}.stdout", self.program()))
    }

    pub fn residue_only(&self) -> Option<&str> {
        self.behaviour.strip_prefix(RESIDUE_ONLY)
    }
}

pub fn parse(text: &str) -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [file, cell, source, seed, obfuscator, behaviour, options, sha256] = fields.as_slice()
        else {
            bail!(
                "MANIFEST line {}: want 8 fields, got {}",
                number + 1,
                fields.len()
            );
        };
        if *behaviour != NODE && !behaviour.starts_with(RESIDUE_ONLY) {
            bail!("MANIFEST line {}: behaviour {behaviour:?}", number + 1);
        }
        out.push(Entry {
            file: file.to_string(),
            cell: cell.to_string(),
            source: source.to_string(),
            seed: seed
                .parse()
                .with_context(|| format!("MANIFEST line {} seed", number + 1))?,
            obfuscator: obfuscator.to_string(),
            behaviour: behaviour.to_string(),
            options: options.to_string(),
            sha256: sha256.to_string(),
        });
    }
    Ok(out)
}

pub fn load(root: &Path) -> Result<Vec<Entry>> {
    let path = root.join(DIR).join("MANIFEST");
    parse(&std::fs::read_to_string(&path).with_context(|| path.display().to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_round_trip_and_bad_rows_are_rejected() {
        let rows = parse(
            "# header\na--greeting-1.js\ta\ttools/obfuscated/sources/greeting.js\t7\tjavascript-obfuscator@5.9.1\tresidue-only:hostile\t{}\tabc\n",
        )
        .unwrap();
        assert_eq!(rows[0].program(), "greeting");
        assert_eq!(rows[0].residue_only(), Some("hostile"));
        assert!(parse("a\tb\n").is_err());
        assert!(parse("a\tb\tc\t1\td\tmaybe\t{}\tx\n").is_err());
    }
}
