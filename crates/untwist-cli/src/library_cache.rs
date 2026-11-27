use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use untwist::libraries::{sha256_hex, Collection, Shard, ShardSource, INDEX};

use crate::wasm_supply::fetch::{Failure, Fetcher, Limits};
use crate::wasm_supply::url;

pub const DEFAULT_URL: &str = "https://untwist.dev/libraries/";
const CACHE_ENV: &str = "UNTWIST_LIBRARY_CACHE";
const SOURCE: &str = "source.txt";
const SHARDS: &str = "shards";
const PARTIAL: &str = ".partial";
const MEBIBYTE: u64 = 1024 * 1024;
const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

pub const LIMITS: Limits = Limits {
    references: 4096,
    response_bytes: 256 * MEBIBYTE,
    total_bytes: 2048 * MEBIBYTE,
    redirects: 3,
    phase: Duration::from_secs(1800),
    request: Duration::from_secs(300),
    flag: "untwist libraries",
    items: "files",
};

pub fn cache_dir() -> Option<PathBuf> {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(dir) = env(CACHE_ENV) {
        return Some(PathBuf::from(dir));
    }
    let base = env("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("untwist").join("libraries"))
}

pub fn cached_index(dir: &Path) -> Option<PathBuf> {
    let index = dir.join(INDEX);
    index.is_file().then_some(index)
}

pub struct Remote {
    pub root: String,
    pub index: String,
}

pub fn remote(url: &str) -> Result<Remote> {
    let Some(url) = url::fetchable(url) else {
        bail!("{url} is not an http(s) URL");
    };
    if url.ends_with(".json") {
        let root = url::joined(&url, "./").context("the collection URL has no directory")?;
        return Ok(Remote { root, index: url });
    }
    let root = match url.ends_with('/') {
        true => url,
        false => format!("{url}/"),
    };
    let index = url::joined(&root, INDEX).context("the collection URL does not join")?;
    Ok(Remote { root, index })
}

fn is_loopback(url: &str) -> bool {
    url.split_once("://")
        .map(|(_, rest)| rest.split(['/', '?']).next().unwrap_or_default())
        .map(|authority| {
            authority
                .rsplit_once(':')
                .map_or(authority, |(host, port)| {
                    match port.bytes().all(|byte| byte.is_ascii_digit()) {
                        true => host,
                        false => authority,
                    }
                })
        })
        .is_some_and(|host| LOOPBACK.contains(&host))
}

pub fn fetcher(root: &str) -> Fetcher {
    Fetcher::new(is_loopback(root), LIMITS)
}

fn fetch(fetcher: &mut Fetcher, target: &str, err: &mut dyn Write) -> Result<Vec<u8>> {
    match fetcher.fetch(target, err) {
        Ok(bytes) => Ok(bytes),
        Err(Failure::Fetch(reason)) => bail!("{target}: {reason}"),
        Err(Failure::Stderr(error)) => Err(error).context("write to stderr"),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let partial = path.with_extension(format!(
        "{}{PARTIAL}",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
    ));
    std::fs::write(&partial, bytes).with_context(|| format!("write {}", partial.display()))?;
    std::fs::rename(&partial, path).with_context(|| format!("write {}", path.display()))
}

const LOCAL_LIMIT: u64 = 256 * MEBIBYTE;

pub fn read_regular(path: &Path, expected: Option<u64>) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|error| error.to_string())?;
    if !meta.is_file() {
        return Err("not a regular file".to_string());
    }
    if expected.is_some_and(|bytes| bytes != meta.len()) {
        return Err("the size differs from the index".to_string());
    }
    if meta.len() > LOCAL_LIMIT {
        return Err(format!("it is larger than {} MiB", LOCAL_LIMIT / MEBIBYTE));
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    std::fs::File::open(path)
        .and_then(|file| {
            std::io::Read::read_to_end(&mut std::io::Read::take(file, LOCAL_LIMIT + 1), &mut bytes)
        })
        .map_err(|error| error.to_string())?;
    match expected.is_some_and(|size| size != bytes.len() as u64)
        || bytes.len() as u64 > LOCAL_LIMIT
    {
        true => Err("the size differs from the index".to_string()),
        false => Ok(bytes),
    }
}

fn has_file(dir: &Path, file: &str, sha256: &str) -> bool {
    read_regular(&dir.join(file), None).is_ok_and(|bytes| sha256_hex(&bytes) == sha256)
}

fn checked(bytes: Vec<u8>, file: &str, sha256: &str, size: Option<u64>) -> Result<Vec<u8>> {
    if size.is_some_and(|size| size != 0 && size != bytes.len() as u64) {
        bail!("{file}: the size differs from the index");
    }
    if sha256_hex(&bytes) != sha256 {
        bail!("{file}: SHA-256 mismatch");
    }
    Ok(bytes)
}

fn referenced(collection: &Collection) -> BTreeSet<String> {
    collection
        .packages()
        .iter()
        .flat_map(|package| package.shards.iter().map(|shard| shard.file.clone()))
        .chain(collection.filters().map(|filters| filters.file.clone()))
        .collect()
}

fn collect_garbage(dir: &Path, keep: &BTreeSet<String>) -> Result<usize> {
    let Ok(entries) = std::fs::read_dir(dir.join(SHARDS)) else {
        return Ok(0);
    };
    let mut removed = 0;
    for entry in entries {
        let entry = entry?;
        let name = format!("{SHARDS}/{}", entry.file_name().to_string_lossy());
        if entry.file_type()?.is_file() && !keep.contains(&name) {
            std::fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

pub fn update(dir: &Path, url: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<()> {
    let remote = remote(url)?;
    let mut fetcher = fetcher(&remote.root);
    let index_bytes = fetch(&mut fetcher, &remote.index, err)?;
    let index_text = String::from_utf8(index_bytes).context("the collection index is not UTF-8")?;
    let collection = Collection::parse(&index_text)
        .map_err(|error| anyhow::anyhow!("{}: {error}", remote.index))?;
    let mut wanted: Vec<(String, String, Option<u64>)> = collection
        .packages()
        .iter()
        .flat_map(|package| &package.shards)
        .map(|shard| (shard.file.clone(), shard.sha256.clone(), Some(shard.bytes)))
        .collect();
    if let Some(filters) = collection.filters() {
        wanted.push((
            filters.file.clone(),
            filters.sha256.clone(),
            Some(filters.bytes),
        ));
    }
    let (mut fetched, mut fetched_bytes, mut cached) = (0usize, 0u64, 0usize);
    for (file, sha256, size) in &wanted {
        if has_file(dir, file, sha256) {
            cached += 1;
            continue;
        }
        let target = url::joined(&remote.root, file).context("a shard URL does not join")?;
        let bytes = checked(
            fetch(&mut fetcher, &target, &mut std::io::sink())?,
            file,
            sha256,
            *size,
        )?;
        fetched_bytes += bytes.len() as u64;
        write_atomic(&dir.join(file), &bytes)?;
        fetched += 1;
    }
    write_atomic(&dir.join(INDEX), index_text.as_bytes())?;
    write_atomic(&dir.join(SOURCE), format!("{url}\n").as_bytes())?;
    let removed = collect_garbage(dir, &referenced(&collection))?;
    writeln!(
        out,
        "library collection {} (shape epoch {}): {} packages; fetched {fetched} files ({}), {cached} already cached, {removed} old files removed; in {}",
        collection.index.version,
        collection.index.shape_epoch,
        collection.packages().len(),
        megabytes(fetched_bytes),
        dir.display()
    )?;
    Ok(())
}

pub fn status(dir: &Path, out: &mut dyn Write) -> Result<()> {
    let Some(index) = cached_index(dir) else {
        writeln!(
            out,
            "no library collection in {}; run `untwist libraries update` to fetch one",
            dir.display()
        )?;
        return Ok(());
    };
    let text =
        std::fs::read_to_string(&index).with_context(|| format!("read {}", index.display()))?;
    let source = std::fs::read_to_string(dir.join(SOURCE)).unwrap_or_default();
    let collection = match Collection::parse(&text) {
        Ok(collection) => collection,
        Err(error) => {
            writeln!(
                out,
                "library collection in {}: {error}; run `untwist libraries update`",
                dir.display()
            )?;
            return Ok(());
        }
    };
    let files = referenced(&collection);
    let present: Vec<u64> = files
        .iter()
        .filter_map(|file| std::fs::metadata(dir.join(file)).ok())
        .map(|meta| meta.len())
        .collect();
    writeln!(
        out,
        "library collection {} (shape epoch {}) from {}",
        collection.index.version,
        collection.index.shape_epoch,
        source.trim()
    )?;
    writeln!(
        out,
        "{} packages, {} of {} files present ({}), in {}",
        collection.packages().len(),
        present.len(),
        files.len(),
        megabytes(present.iter().sum()),
        dir.display()
    )?;
    Ok(())
}

pub fn clear(dir: &Path, out: &mut dyn Write) -> Result<()> {
    let mut removed = 0;
    for name in [INDEX, SOURCE] {
        if dir.join(name).is_file() {
            std::fs::remove_file(dir.join(name))?;
            removed += 1;
        }
    }
    removed += collect_garbage(dir, &BTreeSet::new())?;
    let _ = std::fs::remove_dir(dir.join(SHARDS));
    writeln!(out, "removed {removed} files from {}", dir.display())?;
    Ok(())
}

pub struct Fetched {
    root: String,
    cache: Option<PathBuf>,
    fetcher: Mutex<Fetcher>,
}

impl ShardSource for Fetched {
    fn read(&self, shard: &Shard) -> Result<Vec<u8>, String> {
        if let Some(cache) = &self.cache {
            if let Ok(bytes) = read_regular(&cache.join(&shard.file), Some(shard.bytes)) {
                if sha256_hex(&bytes) == shard.sha256 {
                    return Ok(bytes);
                }
            }
        }
        let target = url::joined(&self.root, &shard.file)
            .ok_or_else(|| "the shard URL does not join".to_string())?;
        let mut fetcher = self
            .fetcher
            .lock()
            .map_err(|_| "the fetcher is poisoned".to_string())?;
        let mut quiet = std::io::sink();
        let bytes = match fetcher.fetch(&target, &mut quiet) {
            Ok(bytes) => bytes,
            Err(Failure::Fetch(reason)) => return Err(reason),
            Err(Failure::Stderr(error)) => return Err(error.to_string()),
        };
        let bytes = checked(bytes, &shard.file, &shard.sha256, Some(shard.bytes))
            .map_err(|error| error.to_string())?;
        if let Some(cache) = &self.cache {
            let _ = write_atomic(&cache.join(&shard.file), &bytes);
        }
        Ok(bytes)
    }
}

pub struct Opened {
    pub collection: Collection,
    pub source: Fetched,
}

pub fn open_remote(url: &str, err: &mut dyn Write) -> Result<Opened> {
    let remote = remote(url)?;
    let mut fetcher = fetcher(&remote.root);
    let bytes = fetch(&mut fetcher, &remote.index, err)?;
    let text = String::from_utf8(bytes).context("the collection index is not UTF-8")?;
    let mut collection =
        Collection::parse(&text).map_err(|error| anyhow::anyhow!("{}: {error}", remote.index))?;
    if let Some(filters) = collection.filters().cloned() {
        let target =
            url::joined(&remote.root, &filters.file).context("the filter URL does not join")?;
        let bytes = fetch(&mut fetcher, &target, err)?;
        if let Err(error) = collection.attach_filters(&bytes) {
            writeln!(
                err,
                "warning: library collection {url}: {error}; every package is loaded"
            )?;
        }
    }
    Ok(Opened {
        collection,
        source: Fetched {
            root: remote.root,
            cache: cache_dir(),
            fetcher: Mutex::new(fetcher),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_collection_url_names_its_index_and_root() {
        let found = remote("https://untwist.dev/libraries").unwrap();
        assert_eq!(found.root, "https://untwist.dev/libraries/");
        assert_eq!(found.index, "https://untwist.dev/libraries/collection.json");
        let named = remote("https://x.dev/c/2026.12.1.json").unwrap();
        assert_eq!(named.root, "https://x.dev/c/");
        assert_eq!(named.index, "https://x.dev/c/2026.12.1.json");
        assert!(remote("ftp://x.dev/").is_err());
    }

    #[test]
    fn only_a_loopback_host_may_be_fetched_from_a_local_address() {
        assert!(is_loopback("http://127.0.0.1:8080/libraries/"));
        assert!(is_loopback("http://localhost/"));
        assert!(is_loopback("http://[::1]:9/"));
        assert!(!is_loopback("https://untwist.dev/libraries/"));
        assert!(!is_loopback("http://127.0.0.1.example.com/"));
    }
}
