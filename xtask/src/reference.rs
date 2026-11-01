use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

pub const DIR_VAR: &str = "UNTWIST_REFERENCE_DIR";
const DEFAULT_DIR: &str = "reference";

pub fn dir(repo_root: &Path) -> PathBuf {
    std::env::var_os(DIR_VAR).map_or_else(|| repo_root.join(DEFAULT_DIR), PathBuf::from)
}

pub fn cli(repo_root: &Path) -> Result<PathBuf> {
    let root = dir(repo_root);
    let built = std::fs::read_dir(root.join("packages"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("dist/cli.js"))
        .find(|cli| cli.is_file());
    match built {
        Some(cli) => Ok(cli),
        None => bail!(
            "the reference deobfuscator is not built: no packages/*/dist/cli.js under {}; \
             clone it there or set {DIR_VAR}, then run `pnpm install && pnpm build` inside it",
            root.display()
        ),
    }
}
