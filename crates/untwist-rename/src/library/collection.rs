use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::hash::mix;
use super::table::{inflate_with_limit, Table, INFLATED_LIMIT};
use super::SHAPE_EPOCH;

pub const SCHEMA: &str = "untwist.library-collection/1";
pub const INDEX: &str = "collection.json";
const BLOOM_MAGIC: &[u8; 8] = b"UTBLOOM1";
pub const BLOOM_BITS_PER_KEY: u64 = 10;
pub const BLOOM_HASHES: u8 = 7;
const BLOOM_SEED: u64 = 0x5bd1_e995_9e37_79b9;
const BLOOM_MIN_BITS: u64 = 64;

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

fn positions(key: u32, count: u64, hashes: u8) -> impl Iterator<Item = u64> {
    let hash = mix(u64::from(key) ^ BLOOM_SEED);
    let first = hash & 0xffff_ffff;
    let step = (hash >> 32) | 1;
    (0..u64::from(hashes)).map(move |index| first.wrapping_add(index.wrapping_mul(step)) % count)
}

impl Bloom {
    pub fn with_keys(keys: &[u32]) -> Self {
        let mut unique: Vec<u32> = keys.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let count = (unique.len() as u64 * BLOOM_BITS_PER_KEY).max(BLOOM_MIN_BITS);
        let count = count.div_ceil(8) * 8;
        let mut bits = vec![0u8; (count / 8) as usize];
        for key in unique {
            for position in positions(key, count, BLOOM_HASHES) {
                bits[(position / 8) as usize] |= 1 << (position % 8);
            }
        }
        Self {
            bits,
            count,
            hashes: BLOOM_HASHES,
        }
    }

    pub fn contains(&self, key: u32) -> bool {
        positions(key, self.count, self.hashes)
            .all(|position| self.bits[(position / 8) as usize] & (1 << (position % 8)) != 0)
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
        Ok(Self { index, blooms })
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
            if let Some(index) = self
                .index
                .packages
                .iter()
                .position(|package| package.id == id)
            {
                self.blooms[index] = Some(bloom);
            }
        }
        Ok(())
    }

    pub fn wanted_by_keys(&self, loose: &FxHashSet<u32>) -> Vec<usize> {
        (0..self.index.packages.len())
            .filter(|&index| match &self.blooms[index] {
                Some(bloom) => loose.iter().any(|&key| bloom.contains(key)),
                None => !loose.is_empty(),
            })
            .collect()
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
