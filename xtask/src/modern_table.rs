use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const TABLE: &str = "crates/untwist-rename/data/core-js-compat.tsv";
const GENERATOR: &str = "tools/core-js-compat/table.mjs";

#[derive(clap::Args, Debug)]
pub struct Request {
    #[arg(long)]
    pub check: bool,
}

fn generate(root: &Path) -> Result<String> {
    let tools = root.join("tools/core-js-compat");
    if !tools.join("node_modules").is_dir() {
        bail!("run `npm ci --ignore-scripts` in {} first", tools.display());
    }
    let output = Command::new("node")
        .arg(root.join(GENERATOR))
        .current_dir(&tools)
        .output()
        .context("running node")?;
    if !output.status.success() {
        bail!(
            "{GENERATOR} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).context("the table is not UTF-8")
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let table = generate(root)?;
    let path = root.join(TABLE);
    if request.check {
        let committed =
            std::fs::read_to_string(&path).with_context(|| format!("reading {TABLE}"))?;
        if committed != table {
            bail!("{TABLE} is stale: run `cargo xtask modern-table`");
        }
        println!("{TABLE} is up to date");
        return Ok(());
    }
    std::fs::write(&path, &table).with_context(|| format!("writing {TABLE}"))?;
    println!("wrote {TABLE} ({} lines)", table.lines().count());
    Ok(())
}
