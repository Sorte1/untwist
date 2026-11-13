use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

pub const DIR: &str = "fixtures/bundle-matrix";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub app: String,
    pub name: String,
    pub family: String,
    pub tool: String,
    pub loader: String,
    pub entry: Vec<String>,
    pub source_modules: Option<usize>,
}

impl Cell {
    pub fn id(&self) -> String {
        format!("{}/{}", self.app, self.name)
    }

    pub fn dir(&self, root: &Path) -> PathBuf {
        root.join(DIR).join(&self.app).join(&self.name)
    }

    pub fn expected_stdout(&self, root: &Path) -> PathBuf {
        root.join(DIR)
            .join("expected")
            .join(&self.app)
            .join(format!("{}.stdout", self.name))
    }

    pub fn base(&self) -> &str {
        self.name.split('+').next().unwrap_or(&self.name)
    }

    pub fn bundler(&self) -> String {
        let base = self.base();
        let head = base.split('-').next().unwrap_or(base);
        match self.name.split_once('+') {
            Some((_, post)) if post.starts_with("obf") => format!("{head}+obfuscator"),
            Some(_) => format!("{head}+minifier"),
            None => head.to_string(),
        }
    }
}

pub fn parse(text: &str) -> Result<Vec<Cell>> {
    let mut cells = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [app, name, family, tool, loader, entry, modules, _flags] = fields[..] else {
            bail!(
                "MANIFEST line {}: want 8 tab-separated fields, got {}",
                number + 1,
                fields.len()
            );
        };
        let source_modules = match modules {
            "-" => None,
            n => Some(
                n.parse()
                    .with_context(|| format!("MANIFEST line {}: module count {n:?}", number + 1))?,
            ),
        };
        cells.push(Cell {
            app: app.to_string(),
            name: name.to_string(),
            family: family.to_string(),
            tool: tool.to_string(),
            loader: loader.to_string(),
            entry: entry.split(',').map(String::from).collect(),
            source_modules,
        });
    }
    Ok(cells)
}

pub fn load(root: &Path) -> Result<Vec<Cell>> {
    let path = root.join(DIR).join("MANIFEST");
    parse(&std::fs::read_to_string(&path).with_context(|| path.display().to_string())?)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn sums(root: &Path) -> Result<BTreeMap<String, String>> {
    let path = root.join(DIR).join("SHA256SUMS");
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let mut out = BTreeMap::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        let Some((sum, file)) = line.split_once("  ") else {
            bail!("SHA256SUMS line {line:?} is not `<sha256>  <path>`");
        };
        out.insert(file.to_string(), sum.to_string());
    }
    Ok(out)
}

pub fn files_of(root: &Path, cell: &Cell, sums: &BTreeMap<String, String>) -> Vec<PathBuf> {
    let prefix = format!("{}/", cell.id());
    sums.keys()
        .filter(|f| f.starts_with(&prefix))
        .map(|f| root.join(DIR).join(f))
        .collect()
}

pub fn verify(root: &Path, cells: &[Cell], sums: &BTreeMap<String, String>) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (file, want) in sums {
        let path = root.join(DIR).join(file);
        let bytes = std::fs::read(&path).with_context(|| path.display().to_string())?;
        if &sha256(&bytes) != want {
            problems.push(format!("{file} does not match its SHA256SUMS line"));
        }
    }
    for cell in cells {
        if files_of(root, cell, sums).is_empty() {
            problems.push(format!("{} has no file in SHA256SUMS", cell.id()));
        }
        if !cell.expected_stdout(root).is_file() {
            problems.push(format!("{} has no expected stdout", cell.id()));
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str = "# header\nbase\twebpack5-web-split+obf-cff\twebpack\twebpack@5\tbrowser\t561.js,index.js\t28\tflags\n";

    #[test]
    fn a_row_names_its_entry_scripts_in_order_and_its_base_cell() {
        let cells = parse(ROW).unwrap();
        assert_eq!(cells[0].entry, ["561.js", "index.js"]);
        assert_eq!(cells[0].source_modules, Some(28));
        assert_eq!(cells[0].base(), "webpack5-web-split");
        assert_eq!(cells[0].bundler(), "webpack5+obfuscator");
    }

    #[test]
    fn a_short_row_is_an_error() {
        assert!(parse("a\tb\tc\n").is_err());
    }
}
