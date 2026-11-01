use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const SKIP_MESSAGE: &str =
    "skipping behaviour differential: `node` not found on PATH and UNTWIST_NODE is unset";

const TIMEOUT: Duration = Duration::from_secs(10);

pub struct Observed {
    pub exit: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn skip_allowed(required: Option<&str>) -> bool {
    required.is_none_or(str::is_empty)
}

fn is_location_line(line: &str) -> bool {
    line.strip_prefix("<file>:")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn drop_location_banner(lines: &mut Vec<&str>) {
    let mut drop_to = 3.min(lines.len());
    if lines.get(drop_to).is_some_and(|line| line.is_empty()) {
        drop_to += 1;
    }
    lines.drain(0..drop_to);
}

fn drop_node_banner(lines: &mut Vec<&str>) {
    if let Some(blank) = lines.iter().position(|line| line.is_empty()) {
        lines.drain(0..=blank);
    }
}

pub fn normalise_stderr(text: &str, paths: &[&Path]) -> String {
    let mut text = text.to_string();
    for path in paths {
        text = text.replace(&path.display().to_string(), "<file>");
    }
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.first().is_some_and(|line| is_location_line(line)) {
        drop_location_banner(&mut lines);
    } else if lines.first().is_some_and(|line| line.starts_with("node:")) {
        drop_node_banner(&mut lines);
    }
    lines.retain(|line| !line.starts_with("    at "));
    if lines
        .last()
        .is_some_and(|line| line.starts_with("Node.js v"))
    {
        lines.pop();
    }
    lines.join("\n").trim_end().to_string()
}

pub fn node() -> Option<PathBuf> {
    let candidate = std::env::var_os("UNTWIST_NODE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("node"));
    let ok = Command::new(&candidate)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    ok.then_some(candidate)
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buffer);
        }
        buffer
    })
}

pub fn run(node: &Path, file: &Path) -> std::io::Result<Option<Observed>> {
    let started = Instant::now();
    let mut child = Command::new(node)
        .arg(file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    loop {
        if let Some(status) = child.try_wait()? {
            let out = stdout.join().unwrap_or_default();
            let err = stderr.join().unwrap_or_default();
            return Ok(Some(Observed {
                exit: status.code(),
                stdout: String::from_utf8_lossy(&out).into_owned(),
                stderr: String::from_utf8_lossy(&err).into_owned(),
            }));
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn syntax_check(node: &Path, file: &Path) -> std::io::Result<Result<(), String>> {
    let out = Command::new(node).arg("--check").arg(file).output()?;
    if out.status.success() {
        return Ok(Ok(()));
    }
    Ok(Err(String::from_utf8_lossy(&out.stderr).into_owned()))
}

pub fn printed_by_node(text: &str) -> Option<String> {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
        return None;
    };
    let dir = tempfile::tempdir().ok()?;
    let path = dir.path().join("program.js");
    std::fs::write(&path, text).ok()?;
    run(&node, &path)
        .ok()
        .flatten()
        .map(|observed| observed.stdout)
}

pub fn assert_node_agrees(input: &str, output: &str) {
    let Some(expected) = printed_by_node(input) else {
        return;
    };
    assert_eq!(
        printed_by_node(output),
        Some(expected),
        "node on the output: {output}"
    );
}
