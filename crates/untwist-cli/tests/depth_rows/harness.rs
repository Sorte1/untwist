use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;
use untwist_testing::capped_child::{self, Cap, Outcome};

pub const MB: u64 = 1_000_000;

static ROW_SETS: Mutex<()> = Mutex::new(());

pub fn one_row_set_at_a_time() -> MutexGuard<'static, ()> {
    ROW_SETS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct Run {
    pub outcome: Outcome,
    pub elapsed: Duration,
}

impl Run {
    pub fn code(&self) -> Option<i32> {
        self.outcome.exit_code
    }

    pub fn stdout(&self) -> &[u8] {
        &self.outcome.stdout
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.outcome.stderr).into_owned()
    }

    pub fn peak_bytes(&self) -> u64 {
        self.outcome.peak_rss_kb * 1024
    }

    pub fn died(&self) -> bool {
        let outcome = &self.outcome;
        outcome.signal.is_some() || outcome.killed_for_memory || outcome.timed_out
    }

    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.outcome.stdout).unwrap_or(Value::Null)
    }

    pub fn summary(&self) -> String {
        let outcome = &self.outcome;
        let stderr = self.stderr();
        let tail: String = stderr.chars().take(400).collect();
        format!(
            "exit {:?} signal {:?} memory {} timeout {} peak {} kB {:?}: {tail}",
            outcome.exit_code,
            outcome.signal,
            outcome.killed_for_memory,
            outcome.timed_out,
            outcome.peak_rss_kb,
            self.elapsed
        )
    }
}

pub fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_untwist"))
}

pub fn run_with(
    program: &Path,
    args: &[OsString],
    env: &[(&str, &str)],
    stdin: Option<&[u8]>,
    cap: Cap,
) -> Run {
    let env: Vec<(OsString, OsString)> = env
        .iter()
        .map(|(key, value)| (OsString::from(key), OsString::from(value)))
        .collect();
    let started = Instant::now();
    let outcome = capped_child::run(program.as_os_str(), args, &env, stdin, cap);
    Run {
        outcome,
        elapsed: started.elapsed(),
    }
}

pub fn untwist(args: &[OsString], env: &[(&str, &str)]) -> Run {
    run_with(&binary(), args, env, None, Cap::DEFAULT)
}

pub fn untwist_stdin(args: &[OsString], env: &[(&str, &str)], stdin: &[u8]) -> Run {
    run_with(&binary(), args, env, Some(stdin), Cap::DEFAULT)
}

pub fn argv(flags: &[&str], files: &[&Path]) -> Vec<OsString> {
    flags
        .iter()
        .map(OsString::from)
        .chain(files.iter().map(|path| path.as_os_str().to_owned()))
        .collect()
}

pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    pub fn new() -> Self {
        let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR"))
            .unwrap_or_else(|error| panic!("a scratch dir: {error}"));
        Self { dir }
    }

    pub fn file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, contents).unwrap_or_else(|error| panic!("write {name}: {error}"));
        path
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

pub struct Rows {
    failures: Vec<String>,
}

impl Rows {
    pub fn new() -> Self {
        Self {
            failures: Vec::new(),
        }
    }

    pub fn check(&mut self, label: &str, holds: bool, detail: impl FnOnce() -> String) {
        if holds {
            return;
        }
        let line = format!("{label}: {}", detail());
        println!("FAIL {line}");
        self.failures.push(line);
    }

    pub fn finish(self) {
        assert!(self.failures.is_empty(), "{}", self.failures.join("\n"));
    }
}

pub fn passed_through(stdout: &[u8], input: &str) -> bool {
    stdout.strip_suffix(b"\n") == Some(input.as_bytes())
}

pub fn workspace_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

pub fn deobfuscate_fixtures() -> Vec<PathBuf> {
    let dir = workspace_file("fixtures/reference/deobfuscate");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort();
    files
}

pub fn without_timings(mut json: Value) -> Value {
    if let Some(stages) = json.get_mut("stages").and_then(Value::as_array_mut) {
        for stage in stages {
            if let Some(fields) = stage.as_object_mut() {
                for key in [
                    "elapsed_ms",
                    "per_pass_ms",
                    "iterations_ms",
                    "fixpoint_per_pass_ms",
                ] {
                    fields.remove(key);
                }
            }
        }
    }
    json
}

pub fn codes(json: &Value) -> Vec<String> {
    if let Some(codes) = json.get("codes").and_then(Value::as_array) {
        return codes
            .iter()
            .map(|code| code.as_str().unwrap_or_default().to_string())
            .collect();
    }
    json.get("code")
        .and_then(Value::as_str)
        .map(|code| vec![code.to_string()])
        .unwrap_or_default()
}

pub fn refused(json: &Value) -> Option<Vec<u64>> {
    json.get("depth_refused")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_u64).collect())
}
