use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use untwist::libraries::{self, Collection, Shard, ShardSource, INDEX};

use crate::library_cache;

const BUILTIN: &str = "builtin";
const NONE: &str = "none";
const CACHE: &str = "cache";
const CORE_JS: &str = "core-js";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Builtin,
    None,
    Cache,
    Path(PathBuf),
    Url(String),
}

pub fn source(value: &str) -> Result<Source, String> {
    match value {
        BUILTIN => Ok(Source::Builtin),
        NONE => Ok(Source::None),
        CACHE => Ok(Source::Cache),
        "" => Err("expected builtin, none, cache, a path or an http(s) URL".to_string()),
        _ if value.starts_with("http://") || value.starts_with("https://") => {
            Ok(Source::Url(value.to_string()))
        }
        path => Ok(Source::Path(PathBuf::from(path))),
    }
}

struct Directory {
    root: PathBuf,
}

impl ShardSource for Directory {
    fn read(&self, shard: &Shard) -> Result<Vec<u8>, String> {
        library_cache::read_regular(&self.root.join(&shard.file), Some(shard.bytes))
    }
}

fn is_table(path: &Path) -> bool {
    path.is_file() && path.to_string_lossy().ends_with(".table.gz")
}

fn collection_root(path: &Path) -> Result<PathBuf> {
    if path.is_dir() {
        return Ok(path.to_path_buf());
    }
    let index = path.file_name().is_some_and(|name| name == INDEX) && path.is_file();
    match (index, path.parent()) {
        (true, Some(parent)) => Ok(parent.to_path_buf()),
        _ => bail!(
            "--libraries {}: expected a directory holding {INDEX}, a {INDEX} file or a .table.gz file",
            path.display()
        ),
    }
}

fn load_table(path: &Path) -> Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let tables = libraries::LibraryTables::from_gz_bytes(&bytes)
        .with_context(|| format!("{} is not a library table", path.display()))?;
    libraries::load(tables);
    Ok(())
}

fn attach_cache(err: &mut dyn Write, asked: bool) -> Result<()> {
    let cached =
        library_cache::cache_dir().filter(|dir| library_cache::cached_index(dir).is_some());
    match (cached, asked) {
        (Some(dir), _) => attach_collection(&dir, err),
        (None, true) => {
            writeln!(
                err,
                "warning: no cached library collection; run `untwist libraries update`; using the built-in tables"
            )?;
            Ok(())
        }
        (None, false) => Ok(()),
    }
}

fn attach_url(url: &str, err: &mut dyn Write) -> Result<()> {
    let opened = library_cache::open_remote(url, err)?;
    libraries::attach(opened.collection, Some(Box::new(opened.source)));
    Ok(())
}

fn attach_collection(path: &Path, err: &mut dyn Write) -> Result<()> {
    let root = collection_root(path)?;
    let index = root.join(INDEX);
    let bytes = library_cache::read_regular(&index, None)
        .map_err(|error| anyhow::anyhow!("read {}: {error}", index.display()))?;
    let json = String::from_utf8_lossy(&bytes);
    let mut collection = match Collection::parse(&json) {
        Ok(collection) => collection,
        Err(error) => {
            writeln!(
                err,
                "warning: library collection {}: {error}; using the built-in tables",
                root.display()
            )?;
            return Ok(());
        }
    };
    if let Some(filters) = collection.filters().cloned() {
        let file = root.join(&filters.file);
        let expected = (filters.bytes > 0).then_some(filters.bytes);
        let attached = library_cache::read_regular(&file, expected).and_then(|bytes| {
            collection
                .attach_filters(&bytes)
                .map_err(|error| error.to_string())
        });
        if let Err(reason) = attached {
            writeln!(
                err,
                "warning: library collection {}: {reason}; every package is loaded",
                root.display()
            )?;
        }
    }
    libraries::attach(collection, Some(Box::new(Directory { root })));
    Ok(())
}

pub fn configure(sources: &[Source], err: &mut dyn Write) -> Result<()> {
    if sources.is_empty() {
        return attach_cache(err, false);
    }
    let builtin = !sources.contains(&Source::None) || sources.contains(&Source::Builtin);
    libraries::use_builtin(builtin);
    let mut collections = 0;
    for source in sources {
        if let Source::Path(path) = source {
            if is_table(path) {
                load_table(path)?;
                continue;
            }
        }
        if matches!(source, Source::Builtin | Source::None) {
            continue;
        }
        collections += 1;
        if collections > 1 {
            bail!("--libraries takes one collection (a directory, cache or a URL)");
        }
        match source {
            Source::Path(path) => attach_collection(path, err)?,
            Source::Cache => attach_cache(err, true)?,
            Source::Url(url) => attach_url(url, err)?,
            Source::Builtin | Source::None => {}
        }
    }
    Ok(())
}

pub fn require_core_js(modern: bool) -> Result<()> {
    if modern && !libraries::provides(CORE_JS) {
        bail!(
            "core-js fingerprints are not available: run `untwist libraries update` or build with `libraries-corejs`"
        );
    }
    Ok(())
}

pub fn warn_failures(err: &mut dyn Write) -> std::io::Result<()> {
    let Some(status) = libraries::collection_status() else {
        return Ok(());
    };
    for failure in &status.failures {
        writeln!(
            err,
            "warning: library collection: {failure}; that package is skipped"
        )?;
    }
    Ok(())
}
