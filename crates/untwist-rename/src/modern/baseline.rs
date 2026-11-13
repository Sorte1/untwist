use std::cmp::Ordering;
use std::fmt;

use super::compat::Compat;

#[derive(Clone, Debug)]
pub struct Version {
    parts: Vec<u64>,
    text: String,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let parts = text
            .split('.')
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;
        Some(Version {
            parts,
            text: text.to_string(),
        })
    }

    fn padded(&self, at: usize) -> u64 {
        self.parts.get(at).copied().unwrap_or(0)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.parts.len().max(other.parts.len());
        (0..len)
            .map(|at| self.padded(at).cmp(&other.padded(at)))
            .find(|order| order.is_ne())
            .unwrap_or(Ordering::Equal)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

const ENGINES: &[(&str, &str)] = &[
    ("chrome", "chrome"),
    ("edge", "edge"),
    ("firefox", "firefox"),
    ("ff", "firefox"),
    ("safari", "safari"),
    ("ios", "ios"),
    ("ios_saf", "ios"),
    ("node", "node"),
    ("opera", "opera"),
    ("samsung", "samsung"),
    ("android", "android"),
    ("chrome-android", "chrome-android"),
    ("and_chr", "chrome-android"),
    ("firefox-android", "firefox-android"),
    ("and_ff", "firefox-android"),
    ("opera-android", "opera-android"),
    ("op_mob", "opera-android"),
    ("deno", "deno"),
    ("bun", "bun"),
    ("electron", "electron"),
    ("hermes", "hermes"),
    ("react-native", "react-native"),
    ("quest", "quest"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Baseline {
    text: String,
    engines: Vec<(String, Version)>,
}

fn engine_name(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    ENGINES
        .iter()
        .find(|(alias, _)| *alias == lower)
        .map(|(_, engine)| *engine)
}

fn engine_item(item: &str) -> Result<(String, Version), String> {
    let (name, version) = item
        .split_once(|c: char| c == ' ' || c == '@')
        .ok_or_else(|| format!("`{item}` is neither an ES year (such as es2022) nor an engine and version (such as chrome 120)"))?;
    let engine = engine_name(name.trim()).ok_or_else(|| {
        format!(
            "unknown engine `{}` (known: {})",
            name.trim(),
            known_engines()
        )
    })?;
    let version = Version::parse(version).ok_or_else(|| {
        format!(
            "`{}` is not an engine version (such as 120 or 16.4)",
            version.trim()
        )
    })?;
    Ok((engine.to_string(), version))
}

fn known_engines() -> String {
    let mut names: Vec<&str> = ENGINES.iter().map(|(_, engine)| *engine).collect();
    names.dedup();
    names.join(", ")
}

impl Baseline {
    pub fn parse(text: &str, compat: &Compat) -> Result<Self, String> {
        let mut engines: Vec<(String, Version)> = Vec::new();
        for item in text
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            let lower = item.to_ascii_lowercase();
            let found: Vec<(String, Version)> = match compat.year(&lower) {
                Some(year) => year.to_vec(),
                None if lower.starts_with("es")
                    && lower[2..].chars().all(|c| c.is_ascii_digit()) =>
                {
                    let years: Vec<&str> = compat.year_names().collect();
                    return Err(format!(
                        "`{item}` is not a supported ES year (supported: {})",
                        years.join(", ")
                    ));
                }
                None => vec![engine_item(item)?],
            };
            for (engine, version) in found {
                add_engine(&mut engines, engine, version);
            }
        }
        if engines.is_empty() {
            return Err("an empty baseline: give an ES year (such as es2022) or engines (such as chrome 120, firefox 121)".to_string());
        }
        Ok(Baseline {
            text: text.trim().to_string(),
            engines,
        })
    }

    pub fn engines(&self) -> &[(String, Version)] {
        &self.engines
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn describe(&self) -> String {
        self.engines
            .iter()
            .map(|(engine, version)| format!("{engine} {version}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn add_engine(engines: &mut Vec<(String, Version)>, engine: String, version: Version) {
    match engines.iter_mut().find(|(known, _)| *known == engine) {
        Some((_, known)) if version < *known => *known = version,
        Some(_) => {}
        None => engines.push((engine, version)),
    }
}
