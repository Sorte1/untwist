use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::hash::mix;
use super::table::{inflate_with_limit, Table, INFLATED_LIMIT};
use super::{CLUSTER_HITS, SHAPE_EPOCH};

pub const SCHEMA: &str = "untwist.library-collection/1";
pub const INDEX: &str = "collection.json";
const BLOOM_MAGIC: &[u8; 8] = b"UTBLOOM1";
pub const BLOOM_BITS_PER_KEY: u64 = 16;
pub const BLOOM_HASHES: u8 = 11;
const BLOOM_SEED: u64 = 0x5bd1_e995_9e37_79b9;
const BLOOM_MIN_BITS: u64 = 64;
pub const SHARD_BLOOM: char = '#';
pub const EXACT_BLOOM: char = '!';
pub const EXACT_BITS_PER_KEY: u64 = 20;
pub const EXACT_HASHES: u8 = 14;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRef {
    pub file: String,
    pub sha256: String,
    #[serde(default)]
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shard {
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub versions: String,
    #[serde(default)]
    pub rows: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub versions: Vec<String>,
    pub shards: Vec<Shard>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub curated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub schema: String,
    pub version: String,
    pub shape_epoch: u32,
    #[serde(default)]
    pub untwist: String,
    pub packages: Vec<Package>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<FileRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionError {
    Json(String),
    Schema(String),
    Epoch { found: u32 },
    Filters(String),
    Shard { file: String, reason: String },
    Unknown(String),
}

impl std::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectionError::Json(error) => write!(f, "{INDEX} does not parse: {error}"),
            CollectionError::Schema(found) => {
                write!(f, "{INDEX} has schema {found:?}, this untwist reads {SCHEMA:?}")
            }
            CollectionError::Epoch { found } => write!(
                f,
                "the collection is for shape epoch {found}, this untwist computes shape epoch {SHAPE_EPOCH}"
            ),
            CollectionError::Filters(reason) => write!(f, "the bloom filters are unusable: {reason}"),
            CollectionError::Shard { file, reason } => write!(f, "shard {file}: {reason}"),
            CollectionError::Unknown(sha) => {
                write!(f, "no package of the collection has a shard with SHA-256 {sha}")
            }
        }
    }
}

impl std::error::Error for CollectionError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bloom {
    bits: Vec<u8>,
    count: u64,
    hashes: u8,
}

fn positions(key: u64, count: u64, hashes: u8) -> impl Iterator<Item = u64> {
    let hash = mix(key ^ BLOOM_SEED);
    let first = hash & 0xffff_ffff;
    let step = (hash >> 32) | 1;
    (0..u64::from(hashes)).map(move |index| first.wrapping_add(index.wrapping_mul(step)) % count)
}

pub fn pair_key(loose: u32, exact: u64) -> u64 {
    mix(exact ^ u64::from(loose).rotate_left(29))
}

impl Bloom {
    fn build(keys: impl Iterator<Item = u64>, bits_per_key: u64, hashes: u8) -> Self {
        let mut unique: Vec<u64> = keys.collect();
        unique.sort_unstable();
        unique.dedup();
        let count = (unique.len() as u64 * bits_per_key).max(BLOOM_MIN_BITS);
        let count = count.div_ceil(8) * 8;
        let mut bits = vec![0u8; (count / 8) as usize];
        for key in unique {
            for position in positions(key, count, hashes) {
                bits[(position / 8) as usize] |= 1 << (position % 8);
            }
        }
        Self {
            bits,
            count,
            hashes,
        }
    }

    pub fn with_keys(keys: &[u32]) -> Self {
        Self::build(
            keys.iter().map(|&key| u64::from(key)),
            BLOOM_BITS_PER_KEY,
            BLOOM_HASHES,
        )
    }

    pub fn with_pairs(pairs: &[(u32, u64)]) -> Self {
        Self::build(
            pairs.iter().map(|&(loose, exact)| pair_key(loose, exact)),
            EXACT_BITS_PER_KEY,
            EXACT_HASHES,
        )
    }

    fn holds(&self, key: u64) -> bool {
        positions(key, self.count, self.hashes)
            .all(|position| self.bits[(position / 8) as usize] & (1 << (position % 8)) != 0)
    }

    pub fn contains(&self, key: u32) -> bool {
        self.holds(u64::from(key))
    }

    pub fn contains_pair(&self, loose: u32, exact: u64) -> bool {
        self.holds(pair_key(loose, exact))
    }
}

pub fn bloom_file(entries: &[(String, Bloom)]) -> Vec<u8> {
    let mut out = BLOOM_MAGIC.to_vec();
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (id, bloom) in entries {
        out.extend_from_slice(&(id.len() as u32).to_le_bytes());
        out.extend_from_slice(id.as_bytes());
        out.push(bloom.hashes);
        out.extend_from_slice(&bloom.count.to_le_bytes());
        out.extend_from_slice(&bloom.bits);
    }
    out
}

struct Reader<'b> {
    bytes: &'b [u8],
}

impl<'b> Reader<'b> {
    fn take(&mut self, count: usize) -> Option<&'b [u8]> {
        if self.bytes.len() < count {
            return None;
        }
        let (head, rest) = self.bytes.split_at(count);
        self.bytes = rest;
        Some(head)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
}

pub fn parse_bloom_file(bytes: &[u8]) -> Option<Vec<(String, Bloom)>> {
    let mut reader = Reader { bytes };
    if reader.take(BLOOM_MAGIC.len())? != BLOOM_MAGIC {
        return None;
    }
    let entries = reader.u32()?;
    let mut out = Vec::new();
    for _ in 0..entries {
        let length = reader.u32()? as usize;
        let id = std::str::from_utf8(reader.take(length)?).ok()?.to_string();
        let hashes = *reader.take(1)?.first()?;
        let count = reader.u64()?;
        if count == 0 || count % 8 != 0 || hashes == 0 {
            return None;
        }
        let bits = reader.take(usize::try_from(count / 8).ok()?)?.to_vec();
        out.push((
            id,
            Bloom {
                bits,
                count,
                hashes,
            },
        ));
    }
    reader.bytes.is_empty().then_some(out)
}

struct Hits {
    all: usize,
    strong: usize,
    program: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramKeys {
    pub loose: FxHashSet<u32>,
    pub strong: FxHashSet<u32>,
    pub program: Option<u32>,
    pub pairs: Vec<(u32, u64)>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Debug)]
pub struct Collection {
    pub index: Index,
    blooms: Vec<Option<Bloom>>,
    shard_blooms: Vec<Vec<Option<Bloom>>>,
    exact_blooms: Vec<Vec<Option<Bloom>>>,
}

pub fn shard_bloom_id(package: &str, shard: usize) -> String {
    format!("{package}{SHARD_BLOOM}{shard}")
}

pub fn exact_bloom_id(bloom_id: &str) -> String {
    format!("{bloom_id}{EXACT_BLOOM}")
}

fn safe_file(file: &str) -> bool {
    !file.is_empty()
        && !file.starts_with('/')
        && !file.contains('\\')
        && file
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn check_file(file: &str, sha256: &str) -> Result<(), CollectionError> {
    let hex = sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
    match (safe_file(file), hex) {
        (true, true) => Ok(()),
        (false, _) => Err(CollectionError::Shard {
            file: file.to_string(),
            reason: "the path leaves the collection".to_string(),
        }),
        (true, false) => Err(CollectionError::Shard {
            file: file.to_string(),
            reason: "the SHA-256 is not 64 hex digits".to_string(),
        }),
    }
}

impl Collection {
    pub fn parse(json: &str) -> Result<Self, CollectionError> {
        let index: Index =
            serde_json::from_str(json).map_err(|error| CollectionError::Json(error.to_string()))?;
        if index.schema != SCHEMA {
            return Err(CollectionError::Schema(index.schema));
        }
        if index.shape_epoch != SHAPE_EPOCH {
            return Err(CollectionError::Epoch {
                found: index.shape_epoch,
            });
        }
        for shard in index.packages.iter().flat_map(|package| &package.shards) {
            check_file(&shard.file, &shard.sha256)?;
        }
        if let Some(filters) = &index.filters {
            check_file(&filters.file, &filters.sha256)?;
        }
        let blooms = vec![None; index.packages.len()];
        let shard_blooms: Vec<Vec<Option<Bloom>>> = index
            .packages
            .iter()
            .map(|package| vec![None; package.shards.len()])
            .collect();
        Ok(Self {
            index,
            blooms,
            exact_blooms: shard_blooms.clone(),
            shard_blooms,
        })
    }

    pub fn filters(&self) -> Option<&FileRef> {
        self.index.filters.as_ref()
    }

    pub fn packages(&self) -> &[Package] {
        &self.index.packages
    }

    pub fn attach_filters(&mut self, bytes: &[u8]) -> Result<(), CollectionError> {
        let expected = self.filters().ok_or_else(|| {
            CollectionError::Filters("the index names no filter file".to_string())
        })?;
        if sha256_hex(bytes) != expected.sha256 {
            return Err(CollectionError::Filters("SHA-256 mismatch".to_string()));
        }
        let entries = parse_bloom_file(bytes)
            .ok_or_else(|| CollectionError::Filters("the file does not parse".to_string()))?;
        for (id, bloom) in entries {
            let (id, exact) = match id.strip_suffix(EXACT_BLOOM) {
                Some(base) => (base.to_string(), true),
                None => (id, false),
            };
            let (package_id, shard) = match id.rsplit_once(SHARD_BLOOM) {
                Some((package, shard)) => match shard.parse::<usize>() {
                    Ok(shard) => (package.to_string(), Some(shard)),
                    Err(_) => (id, None),
                },
                None => (id, None),
            };
            let Some(index) = self
                .index
                .packages
                .iter()
                .position(|package| package.id == package_id)
            else {
                continue;
            };
            match (shard, exact) {
                (Some(shard), false) => {
                    if let Some(slot) = self.shard_blooms[index].get_mut(shard) {
                        *slot = Some(bloom);
                    }
                }
                (Some(shard), true) => {
                    if let Some(slot) = self.exact_blooms[index].get_mut(shard) {
                        *slot = Some(bloom);
                    }
                }
                (None, false) => self.blooms[index] = Some(bloom),
                (None, true) => {
                    if let [slot] = self.exact_blooms[index].as_mut_slice() {
                        *slot = Some(bloom);
                    }
                }
            }
        }
        Ok(())
    }

    fn bloom_of(&self, package: usize, shard: usize) -> Option<&Bloom> {
        self.shard_blooms[package][shard]
            .as_ref()
            .or(self.blooms[package].as_ref())
    }

    pub fn has_exact_filters(&self) -> bool {
        self.exact_blooms.iter().flatten().any(Option::is_some)
    }

    pub fn shard_count(&self) -> usize {
        self.index
            .packages
            .iter()
            .map(|package| package.shards.len())
            .sum()
    }

    pub fn shards_wanted_by_keys(&self, loose: &FxHashSet<u32>) -> Vec<(usize, usize)> {
        self.index
            .packages
            .iter()
            .enumerate()
            .flat_map(|(package, entry)| (0..entry.shards.len()).map(move |shard| (package, shard)))
            .filter(|&(package, shard)| match self.bloom_of(package, shard) {
                Some(bloom) => loose.iter().any(|&key| bloom.contains(key)),
                None => !loose.is_empty(),
            })
            .collect()
    }

    fn exact_hit(&self, package: usize, shard: usize, keys: &ProgramKeys) -> bool {
        let (Some(bloom), Some(pairs)) = (
            self.bloom_of(package, shard),
            self.exact_blooms[package][shard].as_ref(),
        ) else {
            return true;
        };
        keys.pairs
            .iter()
            .any(|&(loose, exact)| bloom.contains(loose) && pairs.contains_pair(loose, exact))
    }

    fn shard_hits(&self, package: usize, shard: usize, keys: &ProgramKeys) -> Hits {
        let Some(bloom) = self.bloom_of(package, shard) else {
            let any = usize::from(!keys.loose.is_empty());
            return Hits {
                all: any * CLUSTER_HITS,
                strong: any,
                program: false,
            };
        };
        let count = |set: &FxHashSet<u32>, cap: usize| {
            set.iter()
                .filter(|&&key| bloom.contains(key))
                .take(cap)
                .count()
        };
        Hits {
            all: count(&keys.loose, CLUSTER_HITS),
            strong: count(&keys.strong, 1),
            program: keys.program.is_some_and(|key| bloom.contains(key)),
        }
    }

    pub fn shards_wanted(&self, keys: &ProgramKeys) -> Vec<(usize, usize)> {
        let mut wanted = Vec::new();
        for (package, entry) in self.index.packages.iter().enumerate() {
            let hits: Vec<Hits> = (0..entry.shards.len())
                .map(|shard| self.shard_hits(package, shard, keys))
                .collect();
            let total: usize = hits.iter().map(|hit| hit.all).sum();
            let strong: usize = hits.iter().map(|hit| hit.strong).sum();
            let whole = hits.iter().any(|hit| hit.program);
            let enough = whole || (strong > 0 && total >= CLUSTER_HITS);
            if !enough {
                continue;
            }
            wanted.extend(
                hits.iter()
                    .enumerate()
                    .filter(|(shard, hit)| {
                        (hit.all > 0 || hit.program) && self.exact_hit(package, *shard, keys)
                    })
                    .map(|(shard, _)| (package, shard)),
            );
        }
        wanted
    }

    pub fn wanted_by_keys(&self, loose: &FxHashSet<u32>) -> Vec<usize> {
        let mut packages: Vec<usize> = self
            .shards_wanted_by_keys(loose)
            .into_iter()
            .map(|(package, _)| package)
            .collect();
        packages.dedup();
        packages
    }

    pub fn wanted_by_text(&self, text: &str) -> Vec<usize> {
        (0..self.index.packages.len())
            .filter(|&index| {
                self.index.packages[index]
                    .keys
                    .iter()
                    .any(|key| text.contains(key.as_str()))
            })
            .collect()
    }

    pub fn shard_owner(&self, sha256: &str) -> Option<(usize, &Shard)> {
        self.index
            .packages
            .iter()
            .enumerate()
            .find_map(|(index, package)| {
                package
                    .shards
                    .iter()
                    .find(|shard| shard.sha256 == sha256)
                    .map(|shard| (index, shard))
            })
    }
}

pub fn shard_table(shard: &Shard, bytes: &[u8]) -> Result<Table, CollectionError> {
    let failed = |reason: &str| CollectionError::Shard {
        file: shard.file.clone(),
        reason: reason.to_string(),
    };
    if bytes.len() as u64 != shard.bytes {
        return Err(failed("the size differs from the index"));
    }
    if sha256_hex(bytes) != shard.sha256 {
        return Err(failed("SHA-256 mismatch"));
    }
    let text =
        inflate_with_limit(bytes, INFLATED_LIMIT).ok_or_else(|| failed("not a gzip table"))?;
    Table::parse(&text).ok_or_else(|| failed("the table does not parse"))
}
