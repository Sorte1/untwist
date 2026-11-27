use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use untwist::libraries::{self, Collection, Shard, ShardSource, INDEX};

const BUILTIN: &str = "builtin";
const NONE: &str = "none";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Builtin,
    None,
    Path(PathBuf),
}

pub fn source(value: &str) -> Result<Source, String> {
    match value {
        BUILTIN => Ok(Source::Builtin),
        NONE => Ok(Source::None),
        "" => Err("expected builtin, none or a path".to_string()),
        _ if value.starts_with("http://") || value.starts_with("https://") => {
            Err("library collections are read from a local path in this version".to_string())
        }
        path => Ok(Source::Path(PathBuf::from(path))),
    }
}

struct Directory {
    root: PathBuf,
}

impl ShardSource for Directory {
    fn read(&self, shard: &Shard) -> Result<Vec<u8>, String> {
        std::fs::read(self.root.join(&shard.file)).map_err(|error| error.to_string())
    }
}

fn is_table(path: &Path) -> bool {
    path.is_file() && path.to_string_lossy().ends_with(".table.gz")
}

fn collection_root(path: &Path) -> PathBuf {
    match path.is_dir() {
        true => path.to_path_buf(),
        false => path.parent().map(Path::to_path_buf).unwrap_or_default(),
    }
}

fn load_table(path: &Path) -> Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let tables = libraries::LibraryTables::from_gz_bytes(&bytes)
        .with_context(|| format!("{} is not a library table", path.display()))?;
    libraries::load(tables);
    Ok(())
}

fn attach_collection(path: &Path, err: &mut dyn Write) -> Result<()> {
    let root = collection_root(path);
    let index = root.join(INDEX);
    let json =
        std::fs::read_to_string(&index).with_context(|| format!("read {}", index.display()))?;
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
        let attached = std::fs::read(&file)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
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
        return Ok(());
    }
    let builtin = !sources.contains(&Source::None) || sources.contains(&Source::Builtin);
    libraries::use_builtin(builtin);
    let mut collections = 0;
    for source in sources {
        let Source::Path(path) = source else {
            continue;
        };
        if is_table(path) {
            load_table(path)?;
            continue;
        }
        collections += 1;
        if collections > 1 {
            bail!("--libraries takes one collection directory");
        }
        attach_collection(path, err)?;
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
