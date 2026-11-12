use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};

use super::checks::measure_files;
use super::manifest::Cell;
use crate::obf_matrix::cell::wait_with_timeout;
use crate::obf_options::machinery::Found;

pub const RUNNER: &str = "tools/bundle-matrix/run.mjs";
const UNTWIST_TIMEOUT: Duration = Duration::from_secs(120);
const NODE_TIMEOUT: Duration = Duration::from_secs(20);

pub const LANES: [(&str, &[&str]); 3] = [
    ("default", &[]),
    ("split", &["--split-hoisted"]),
    ("unsafe", &["--unsafe", "--split-hoisted"]),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    pub exit: Option<i32>,
    pub stdout: String,
}

impl Ran {
    pub fn render(&self) -> String {
        format!(
            "exit={}\n{}",
            self.exit.map_or("timeout".to_string(), |e| e.to_string()),
            self.stdout
        )
    }
}

#[derive(Debug)]
pub struct Observation {
    pub untwist_error: Option<String>,
    pub bundle: Option<(String, usize)>,
    pub machinery: Found,
    pub ran: Option<Ran>,
}

fn capture(command: &mut Command, limit: Duration) -> Result<(Option<i32>, String, String)> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn")?;
    let mut out = child.stdout.take().context("stdout")?;
    let mut err = child.stderr.take().context("stderr")?;
    let out_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = out.read_to_end(&mut buffer);
        buffer
    });
    let err_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = err.read_to_end(&mut buffer);
        buffer
    });
    let exit = wait_with_timeout(&mut child, limit)?;
    let stdout = String::from_utf8_lossy(&out_reader.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_reader.join().unwrap_or_default()).into_owned();
    Ok((exit, stdout, stderr))
}

pub fn run_node(node: &Path, root: &Path, loader: &str, entries: &[PathBuf]) -> Result<Ran> {
    let dir = entries
        .last()
        .and_then(|e| e.parent())
        .unwrap_or(root)
        .to_path_buf();
    let (exit, stdout, _) = capture(
        Command::new(node)
            .arg("--no-warnings")
            .arg(root.join(RUNNER))
            .args(["--loader", loader])
            .args(entries)
            .current_dir(dir),
        NODE_TIMEOUT,
    )?;
    Ok(Ran { exit, stdout })
}

fn js_under(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry?.path();
        if path.is_dir() {
            js_under(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|e| e == "js" || e == "mjs" || e == "cjs")
        {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn bundle_of(out: &Path) -> Result<Option<(String, usize)>> {
    let manifest = out.join("bundle.json");
    if !manifest.is_file() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&manifest)?)?;
    let kind = value["type"].as_str().unwrap_or("?").to_string();
    Ok(Some((
        kind,
        value["modules"].as_array().map_or(0, Vec::len),
    )))
}

fn untwist(
    untwist: &Path,
    flags: &[&str],
    args: &[&Path],
) -> Result<std::result::Result<String, String>> {
    let (exit, stdout, stderr) = capture(
        Command::new(untwist).args(flags).args(args),
        UNTWIST_TIMEOUT,
    )?;
    Ok(match exit {
        Some(0) => Ok(stdout),
        other => Err(format!(
            "untwist exited {other:?}: {}",
            stderr.lines().take(3).collect::<Vec<_>>().join(" / ")
        )),
    })
}

pub struct Setup<'a> {
    pub untwist: &'a Path,
    pub node: &'a Path,
    pub root: &'a Path,
}

pub fn observe(
    setup: &Setup<'_>,
    cell: &Cell,
    inputs: &[PathBuf],
    flags: &[&str],
    work: &Path,
) -> Result<Observation> {
    let out = work.join("out");
    let mut args: Vec<&Path> = vec![Path::new("-f"), Path::new("-o"), out.as_path()];
    args.extend(inputs.iter().map(PathBuf::as_path));
    let failed = |error: String| Observation {
        untwist_error: Some(error),
        bundle: None,
        machinery: Found::default(),
        ran: None,
    };
    if let Err(error) = untwist(setup.untwist, flags, &args)? {
        return Ok(failed(error));
    }
    if let Some(bundle) = bundle_of(&out)? {
        let mut outputs = Vec::new();
        js_under(&out, &mut outputs)?;
        outputs.retain(|f| {
            !f.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("deobfuscated."))
        });
        let ran = run_node(setup.node, setup.root, "node", &[out.join("index.js")])?;
        return Ok(Observation {
            untwist_error: None,
            bundle: Some(bundle),
            machinery: measure_files(&outputs),
            ran: Some(ran),
        });
    }
    let cell_dir = cell.dir(setup.root);
    let mirror = work.join("mirror");
    let mut outputs = Vec::new();
    for input in inputs {
        let target = mirror.join(input.strip_prefix(&cell_dir).unwrap_or(input));
        std::fs::create_dir_all(target.parent().context("mirror parent")?)?;
        match untwist(setup.untwist, flags, &[input.as_path()])? {
            Ok(text) => std::fs::write(&target, text)?,
            Err(error) => return Ok(failed(error)),
        }
        outputs.push(target);
    }
    let entries: Vec<PathBuf> = cell.entry.iter().map(|e| mirror.join(e)).collect();
    let ran = run_node(setup.node, setup.root, &cell.loader, &entries)?;
    Ok(Observation {
        untwist_error: None,
        bundle: None,
        machinery: measure_files(&outputs),
        ran: Some(ran),
    })
}
