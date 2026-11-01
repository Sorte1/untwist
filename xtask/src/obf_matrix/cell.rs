use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use super::notes::{warnings_of, Warnings};
use super::residue::{measure, Measured, Residue};

pub const LANES: [&str; 2] = ["safe", "unsafe"];
pub const RENAMES: [&str; 4] = ["none", "simple", "unique", "mangle"];
pub const OUTPUTS: [&str; 3] = ["stdout", "single-file", "dir"];
pub const RUNNER: &str = "tools/unpack-run.mjs";
pub const DEOBFUSCATED: &str = "deobfuscated.js";
const BANNER_START: &str = "// ==== ";
const BANNER_END: &str = " ====";
const NODE_TIMEOUT: Duration = Duration::from_secs(20);
const TIME_BIN: &str = "/usr/bin/time";
const TIMEOUT_BIN: &str = "/usr/bin/timeout";
const TIMED_OUT: i32 = 124;
const KILL_GRACE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    pub lane: &'static str,
    pub rename: &'static str,
    pub output: &'static str,
}

impl Cell {
    pub fn label(&self) -> String {
        format!("{}/{}/{}", self.lane, self.rename, self.output)
    }

    pub fn with_rename(self, rename: &'static str) -> Cell {
        Cell { rename, ..self }
    }

    pub fn with_output(self, output: &'static str) -> Cell {
        Cell { output, ..self }
    }

    pub fn with_lane(self, lane: &'static str) -> Cell {
        Cell { lane, ..self }
    }

    fn flags(&self, out_dir: &Path) -> Vec<String> {
        let mut flags = vec!["--report".to_string(), "text".to_string()];
        if self.lane == "unsafe" {
            flags.push("--unsafe".to_string());
        }
        flags.push("--rename".to_string());
        flags.push(self.rename.to_string());
        match self.output {
            "single-file" => flags.push("--single-file".to_string()),
            "dir" => {
                flags.push("-f".to_string());
                flags.push("-o".to_string());
                flags.push(out_dir.display().to_string());
            }
            _ => {}
        }
        flags
    }
}

pub fn all_cells() -> Vec<Cell> {
    let mut cells = Vec::new();
    for lane in LANES {
        for rename in RENAMES {
            for output in OUTPUTS {
                cells.push(Cell {
                    lane,
                    rename,
                    output,
                });
            }
        }
    }
    cells
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    pub exit: Option<i32>,
    pub stdout: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Behaviour {
    NotChecked,
    Skipped(&'static str),
    Ran(Option<Ran>),
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub exit: Option<i32>,
    pub timed_out: bool,
    pub wall_ms: u64,
    pub rss_kb: Option<u64>,
    pub program: Option<Measured>,
    pub modules: Option<Measured>,
    pub module_set: Vec<String>,
    pub bundle: String,
    pub warnings: Warnings,
    pub behaviour: Behaviour,
}

impl Observation {
    pub fn residue(&self) -> Residue {
        let program = self.program.map(|m| m.residue).unwrap_or_default();
        let modules = self.modules.map(|m| m.residue).unwrap_or_default();
        program.plus(modules)
    }

    pub fn parses(&self) -> bool {
        self.program.is_none_or(|m| m.parses) && self.modules.is_none_or(|m| m.parses)
    }

    pub fn runs(&self) -> bool {
        self.exit == Some(0) && !self.timed_out && self.parses()
    }
}

pub struct Runner {
    pub untwist: PathBuf,
    pub node: Option<PathBuf>,
    pub root: PathBuf,
    pub timeout: Duration,
}

fn read_lossy(path: &Path) -> String {
    let mut bytes = Vec::new();
    if let Ok(mut file) = File::open(path) {
        let _ = file.read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn banner_name(line: &str) -> Option<&str> {
    line.strip_prefix(BANNER_START)?.strip_suffix(BANNER_END)
}

pub fn split_banners(text: &str) -> Option<Vec<(String, String)>> {
    let mut modules: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if let Some(name) = banner_name(line) {
            modules.push((name.to_string(), String::new()));
        } else if let Some((_, body)) = modules.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    (!modules.is_empty()).then_some(modules)
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names
}

fn measure_all<'a>(texts: impl Iterator<Item = &'a str>) -> Option<Measured> {
    let mut total: Option<Measured> = None;
    for text in texts {
        let m = measure(text);
        total = Some(match total {
            None => m,
            Some(t) => Measured {
                residue: t.residue.plus(m.residue),
                parses: t.parses && m.parses,
            },
        });
    }
    total
}

fn js_files(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry?.path();
        if path.is_dir() {
            js_files(&path, base, out)?;
        } else if path
            .extension()
            .is_some_and(|e| e == "js" || e == "mjs" || e == "cjs")
        {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            out.push((rel.display().to_string(), path.clone()));
        }
    }
    out.sort();
    Ok(())
}

fn wait_with_timeout(child: &mut std::process::Child, limit: Duration) -> Result<Option<i32>> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status.code().unwrap_or(-1)));
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn parse_time(path: &Path) -> Option<u64> {
    read_lossy(path).lines().last()?.trim().parse().ok()
}

struct Texts {
    program: Option<Measured>,
    modules: Option<Measured>,
    module_set: Vec<String>,
    runnable: Option<PathBuf>,
    skipped: Option<&'static str>,
}

fn texts_of(cell: Cell, out_file: &Path, out_dir: &Path) -> Result<Texts> {
    if cell.output == "dir" {
        let mut files = Vec::new();
        if out_dir.is_dir() {
            js_files(out_dir, out_dir, &mut files)?;
        }
        let program_path = out_dir.join(DEOBFUSCATED);
        let program = program_path
            .is_file()
            .then(|| measure(&read_lossy(&program_path)));
        let modules: Vec<(String, PathBuf)> = files
            .into_iter()
            .filter(|(name, _)| name != DEOBFUSCATED)
            .collect();
        let bodies: Vec<String> = modules.iter().map(|(_, p)| read_lossy(p)).collect();
        let entry = out_dir.join("index.js");
        let runnable = if entry.is_file() {
            Some(entry)
        } else {
            program_path.is_file().then_some(program_path)
        };
        return Ok(Texts {
            program,
            modules: measure_all(bodies.iter().map(String::as_str)),
            module_set: modules.into_iter().map(|(name, _)| name).collect(),
            runnable,
            skipped: None,
        });
    }
    let text = read_lossy(out_file);
    if cell.output == "single-file" {
        if let Some(modules) = split_banners(&text) {
            return Ok(Texts {
                program: None,
                modules: measure_all(modules.iter().map(|(_, body)| body.as_str())),
                module_set: sorted(modules.into_iter().map(|(name, _)| name).collect()),
                runnable: None,
                skipped: Some("a single-file bundle is a reading view and is not runnable"),
            });
        }
    }
    Ok(Texts {
        program: Some(measure(&text)),
        modules: None,
        module_set: Vec::new(),
        runnable: Some(out_file.to_path_buf()),
        skipped: None,
    })
}

impl Runner {
    pub fn run_node(&self, file: &Path) -> Result<Option<Ran>> {
        let Some(node) = &self.node else {
            return Ok(None);
        };
        let mut child = Command::new(node)
            .arg("--no-warnings")
            .arg(self.root.join(RUNNER))
            .args(["--loader", "node"])
            .arg(file)
            .current_dir(file.parent().unwrap_or(&self.root))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn node")?;
        let mut pipe = child.stdout.take().context("node stdout")?;
        let reader = std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = pipe.read_to_end(&mut buffer);
            buffer
        });
        let exit = wait_with_timeout(&mut child, NODE_TIMEOUT)?;
        let out = reader.join().unwrap_or_default();
        Ok(exit.map(|code| Ran {
            exit: Some(code),
            stdout: String::from_utf8_lossy(&out).into_owned(),
        }))
    }

    fn command(&self, cell: Cell, input: &Path, work: &Path, out_dir: &Path) -> Command {
        let timed = Path::new(TIME_BIN).exists() && Path::new(TIMEOUT_BIN).exists();
        let mut command = if timed {
            let mut c = Command::new(TIME_BIN);
            c.args(["-f", "%M", "-o"]).arg(work.join("time.txt"));
            c.args([TIMEOUT_BIN, "--kill-after=5"])
                .arg(self.timeout.as_secs().to_string())
                .arg(&self.untwist);
            c
        } else {
            Command::new(&self.untwist)
        };
        command.args(cell.flags(out_dir)).arg(input);
        command
    }

    pub fn observe(
        &self,
        input: &Path,
        cell: Cell,
        work: &Path,
        behaviour: bool,
    ) -> Result<Observation> {
        std::fs::create_dir_all(work)?;
        let out_file = work.join("out.js");
        let out_dir = work.join("out");
        let started = Instant::now();
        let mut child = self
            .command(cell, input, work, &out_dir)
            .stdin(Stdio::null())
            .stdout(File::create(&out_file)?)
            .stderr(File::create(work.join("err.txt"))?)
            .spawn()
            .with_context(|| format!("spawn {}", self.untwist.display()))?;
        let exit = wait_with_timeout(&mut child, self.timeout + KILL_GRACE)?
            .filter(|&code| code != TIMED_OUT);
        let wall_ms = started.elapsed().as_millis() as u64;
        let stderr = read_lossy(&work.join("err.txt"));
        let texts = texts_of(cell, &out_file, &out_dir)?;
        let behaviour = match (behaviour, texts.skipped, &texts.runnable) {
            (false, _, _) => Behaviour::NotChecked,
            (true, Some(reason), _) => Behaviour::Skipped(reason),
            (true, None, Some(file)) => Behaviour::Ran(self.run_node(file)?),
            (true, None, None) => Behaviour::Skipped("the run wrote nothing runnable"),
        };
        let bundle = stderr
            .lines()
            .find_map(|line| line.strip_prefix("bundle: "))
            .unwrap_or("none")
            .to_string();
        Ok(Observation {
            exit,
            timed_out: exit.is_none(),
            wall_ms,
            rss_kb: parse_time(&work.join("time.txt")),
            program: texts.program,
            modules: texts.modules,
            module_set: texts.module_set,
            bundle,
            warnings: warnings_of(&stderr, exit),
            behaviour,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_twenty_four_cells() {
        let cells = all_cells();
        assert_eq!(cells.len(), 24);
        assert_eq!(cells[0].label(), "safe/none/stdout");
    }

    #[test]
    fn banners_split_modules() {
        let text = "// untwist: header\n\n// ==== a.js ====\nx();\n// ==== b/c.js ====\ny();\n";
        let modules = split_banners(text).unwrap();
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[1], ("b/c.js".to_string(), "y();\n".to_string()));
        assert!(split_banners("x();").is_none());
    }
}
