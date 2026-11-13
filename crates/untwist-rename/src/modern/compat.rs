use std::sync::OnceLock;

use rustc_hash::FxHashMap;

use super::baseline::{Baseline, Version};

pub const HEADER: &str = "#untwist-core-js-compat";
pub const FORMAT: &str = "v1";

static TABLE_GZ: &[u8] = include_bytes!("../../data/core-js-compat.tsv.gz");

#[derive(Clone, Debug)]
struct Entry {
    engine: String,
    version: Version,
    covers: Vec<(usize, usize)>,
}

#[derive(Debug, Default)]
pub struct Compat {
    versions: Vec<Version>,
    years: Vec<(String, Vec<(String, Version)>)>,
    modules: FxHashMap<String, Vec<Entry>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Native,
    Needed {
        engine: String,
        needs: Option<String>,
        baseline: String,
    },
    Unlisted,
}

fn parse_covers(field: &str, versions: &[Version]) -> Option<Vec<(usize, usize)>> {
    let index = |text: &str| {
        let version = Version::parse(text)?;
        versions.iter().position(|known| *known == version)
    };
    field
        .split(',')
        .map(|range| match range.split_once('~') {
            Some((low, high)) => Some((index(low)?, index(high)?)),
            None => index(range).map(|at| (at, at)),
        })
        .collect()
}

fn parse_year(rest: &str) -> Option<(String, Vec<(String, Version)>)> {
    let (name, engines) = rest.split_once('\t')?;
    let engines = engines
        .split(',')
        .map(|pair| {
            let (engine, version) = pair.split_once(':')?;
            Some((engine.to_string(), Version::parse(version)?))
        })
        .collect::<Option<Vec<_>>>()?;
    Some((name.to_string(), engines))
}

impl Compat {
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let mut header = lines.next()?.split('\t');
        if header.next() != Some(HEADER) || header.next() != Some(FORMAT) {
            return None;
        }
        let mut compat = Compat::default();
        for line in lines {
            if let Some(rest) = line.strip_prefix("#versions\t") {
                compat.versions = rest.split(',').map(Version::parse).collect::<Option<_>>()?;
            } else if let Some(rest) = line.strip_prefix("#baseline\t") {
                compat.years.push(parse_year(rest)?);
            } else if !line.starts_with('#') && !line.is_empty() {
                compat.add_row(line)?;
            }
        }
        Some(compat)
    }

    fn add_row(&mut self, line: &str) -> Option<()> {
        let fields: Vec<&str> = line.split('\t').collect();
        let [module, engine, version, covers] = fields.as_slice() else {
            return None;
        };
        let entry = Entry {
            engine: (*engine).to_string(),
            version: Version::parse(version)?,
            covers: parse_covers(covers, &self.versions)?,
        };
        self.modules
            .entry((*module).to_string())
            .or_default()
            .push(entry);
        Some(())
    }

    pub fn year(&self, name: &str) -> Option<&[(String, Version)]> {
        self.years
            .iter()
            .find(|(year, _)| year == name)
            .map(|(_, engines)| engines.as_slice())
    }

    pub fn year_names(&self) -> impl Iterator<Item = &str> {
        self.years.iter().map(|(name, _)| name.as_str())
    }

    fn indices_between(&self, low: &str, high: &str) -> Vec<usize> {
        let (Some(low), Some(high)) = (Version::parse(low), Version::parse(high)) else {
            return Vec::new();
        };
        self.versions
            .iter()
            .enumerate()
            .filter(|(_, version)| **version >= low && **version <= high)
            .map(|(index, _)| index)
            .collect()
    }

    fn needs<'e>(&self, entries: &'e [Entry], at: usize, engine: &str) -> Option<&'e Version> {
        entries
            .iter()
            .find(|entry| {
                entry.engine == engine
                    && entry
                        .covers
                        .iter()
                        .any(|(low, high)| *low <= at && at <= *high)
            })
            .map(|entry| &entry.version)
    }

    pub fn verdict(&self, module: &str, versions: (&str, &str), baseline: &Baseline) -> Verdict {
        let Some(entries) = self.modules.get(module) else {
            return Verdict::Unlisted;
        };
        let indices: Vec<usize> = self
            .indices_between(versions.0, versions.1)
            .into_iter()
            .filter(|at| listed_at(entries, *at))
            .collect();
        if indices.is_empty() {
            return Verdict::Unlisted;
        }
        for at in indices {
            for (engine, version) in baseline.engines() {
                match self.needs(entries, at, engine) {
                    Some(needed) if needed <= version => {}
                    needed => {
                        return Verdict::Needed {
                            engine: engine.clone(),
                            needs: needed.map(Version::to_string),
                            baseline: version.to_string(),
                        }
                    }
                }
            }
        }
        Verdict::Native
    }
}

fn listed_at(entries: &[Entry], at: usize) -> bool {
    entries.iter().any(|entry| {
        entry
            .covers
            .iter()
            .any(|(low, high)| *low <= at && at <= *high)
    })
}

pub fn builtin() -> &'static Compat {
    static COMPAT: OnceLock<Compat> = OnceLock::new();
    COMPAT.get_or_init(|| {
        crate::library::table::inflate(TABLE_GZ)
            .and_then(|text| Compat::parse(&text))
            .unwrap_or_default()
    })
}
