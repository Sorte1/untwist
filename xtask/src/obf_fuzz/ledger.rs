use std::path::Path;

use anyhow::{bail, Context, Result};

pub const SOUND_REFUSALS: &str = "fixtures/obfuscator-io/SOUND_REFUSALS";
pub const KNOWN: &str = "fixtures/fuzz/EXPECT";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub cause: String,
    pub feature: String,
    pub rows: String,
    pub argument: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Known {
    pub lane: String,
    pub cause: String,
    pub reason: String,
}

fn fields(text: &str, want: usize, file: &str) -> Result<Vec<Vec<String>>> {
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<String> = line.split('\t').map(str::to_string).collect();
        if parts.len() != want || parts.iter().any(|p| p.trim().is_empty()) {
            bail!(
                "{file} line {}: want {want} non-empty tab-separated fields",
                number + 1
            );
        }
        out.push(parts);
    }
    Ok(out)
}

pub fn parse_refusals(text: &str) -> Result<Vec<Refusal>> {
    Ok(fields(text, 4, SOUND_REFUSALS)?
        .into_iter()
        .map(|f| Refusal {
            cause: f[0].clone(),
            feature: f[1].clone(),
            rows: f[2].clone(),
            argument: f[3].clone(),
        })
        .collect())
}

pub fn parse_known(text: &str) -> Result<Vec<Known>> {
    Ok(fields(text, 3, KNOWN)?
        .into_iter()
        .map(|f| Known {
            lane: f[0].clone(),
            cause: f[1].clone(),
            reason: f[2].clone(),
        })
        .collect())
}

fn read(root: &Path, file: &str) -> Result<String> {
    let path = root.join(file);
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).with_context(|| path.display().to_string())
}

pub fn load_refusals(root: &Path) -> Result<Vec<Refusal>> {
    parse_refusals(&read(root, SOUND_REFUSALS)?)
}

pub fn load_known(root: &Path) -> Result<Vec<Known>> {
    parse_known(&read(root, KNOWN)?)
}

pub fn sound<'r>(
    refusals: &'r [Refusal],
    cause: &str,
    has_feature: impl Fn(&str) -> bool,
) -> Option<&'r Refusal> {
    refusals
        .iter()
        .find(|r| r.cause == cause && has_feature(&r.feature))
}

pub fn sound_by_cause<'r>(refusals: &'r [Refusal], cause: &str) -> Option<&'r Refusal> {
    refusals.iter().find(|r| r.cause == cause)
}

pub fn known<'k>(known: &'k [Known], lane: &str, cause: &str) -> Option<&'k Known> {
    known
        .iter()
        .find(|k| k.cause == cause && (k.lane == "*" || k.lane == lane))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_is_sound_only_with_its_feature() {
        let refusals = parse_refusals(
            "# header\nsafe-global-names\tobserves-global-names\trows.rs\tthe program prints the names\n",
        )
        .unwrap();
        let with = |f: &str| f == "observes-global-names";
        assert!(sound(&refusals, "safe-global-names", with).is_some());
        assert!(sound(&refusals, "safe-global-names", |_| false).is_none());
        assert!(sound(&refusals, "other", with).is_none());
        assert!(sound_by_cause(&refusals, "safe-global-names").is_some());
    }

    #[test]
    fn known_gaps_match_their_lane_or_any_lane() {
        let known_gaps =
            parse_known("safe\tsafe-global-key-reach\topen gap\n*\tspelled-decoder-name\tgap\n")
                .unwrap();
        assert!(known(&known_gaps, "safe", "safe-global-key-reach").is_some());
        assert!(known(&known_gaps, "unsafe", "safe-global-key-reach").is_none());
        assert!(known(&known_gaps, "unsafe", "spelled-decoder-name").is_some());
    }

    #[test]
    fn the_checked_in_ledgers_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or(Path::new("."));
        let refusals = load_refusals(root).unwrap();
        assert!(!refusals.is_empty());
        assert!(refusals.iter().all(|r| !r.cause.contains(' ')));
        load_known(root).unwrap();
    }

    #[test]
    fn short_or_empty_lines_are_rejected() {
        assert!(parse_refusals("a\tb\tc\n").is_err());
        assert!(parse_known("safe\t\treason\n").is_err());
    }
}
