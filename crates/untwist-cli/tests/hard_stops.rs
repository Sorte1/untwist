mod common;

use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

const MODES: [&str; 2] = ["on", "off"];
const SMALL: &str = "var a = !0;\nconsole.log(a);\n";
const OTHER: &str = "var b = !1;\nconsole.log(b);\n";
const HARD_STOP_BOUND: Duration = Duration::from_secs(20);

fn inputs(dir: &Path) -> Vec<PathBuf> {
    [("one.js", SMALL), ("two.js", OTHER)]
        .iter()
        .map(|(name, source)| {
            let path = dir.join(name);
            std::fs::write(&path, source).unwrap();
            path
        })
        .collect()
}

fn run(mode: &str, hook: (&str, &str), args: &[&str], files: &[PathBuf]) -> (Output, Duration) {
    let started = Instant::now();
    let out = common::bin()
        .env("UNTWIST_TEST_SUPERVISOR", mode)
        .env(hook.0, hook.1)
        .args(args)
        .args(files)
        .output()
        .unwrap();
    (out, started.elapsed())
}

fn written(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

#[test]
fn an_input_that_allocates_past_the_cap_inside_one_step_is_passed_through_in_both_modes() {
    for mode in MODES {
        let dir = tempfile::tempdir().unwrap();
        let files = inputs(dir.path());
        let (out, _) = run(
            mode,
            ("UNTWIST_TEST_HOG", "1"),
            &["--max-memory", "64MiB", "-j", "1"],
            &files,
        );
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert_eq!(out.status.code(), Some(3), "{mode}: {stderr}");
        assert!(
            stderr.contains("input 2 was not processed: budget-memory"),
            "{mode}: {stderr}"
        );
        assert!(
            !stderr.contains("input 1 was not processed"),
            "{mode}: {stderr}"
        );
        assert!(stdout.contains("var a = true;"), "{mode}: {stdout}");
        assert!(stdout.contains("var b = !1;"), "{mode}: {stdout}");
    }
}

#[test]
fn an_input_that_allocates_past_the_cap_under_parallel_jobs_is_passed_through_in_both_modes() {
    for mode in MODES {
        let dir = tempfile::tempdir().unwrap();
        let files = inputs(dir.path());
        let target = dir.path().join("out");
        let mut args = vec!["--max-memory", "64MiB", "-j", "2", "-o"];
        args.push(target.to_str().unwrap());
        let (out, _) = run(mode, ("UNTWIST_TEST_HOG", "0"), &args, &files);
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(3), "{mode}: {stderr}");
        assert!(
            stderr.contains("input 1 was not processed: budget-memory"),
            "{mode}: {stderr}"
        );
        assert_eq!(
            written(&target, "deobfuscated.one.js").trim(),
            SMALL.trim(),
            "{mode}: {stderr}"
        );
        assert!(written(&target, "deobfuscated.two.js").contains("console.log(b)"));
    }
}

#[test]
fn an_input_that_runs_past_the_timeout_inside_one_step_is_passed_through_in_both_modes() {
    for mode in MODES {
        let dir = tempfile::tempdir().unwrap();
        let files = inputs(dir.path());
        let (out, took) = run(
            mode,
            ("UNTWIST_TEST_STALL", "1"),
            &["--timeout", "1", "-j", "1"],
            &files,
        );
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(took < HARD_STOP_BOUND, "{mode}: {took:?} {stderr}");
        assert_eq!(out.status.code(), Some(3), "{mode}: {stderr}");
        assert!(
            stderr.contains("input 2 was not processed: budget-time"),
            "{mode}: {stderr}"
        );
        assert!(stdout.contains("console.log(a)"), "{mode}: {stdout}");
        assert!(stdout.contains("var b = !1;"), "{mode}: {stdout}");
    }
}

#[test]
fn a_stalled_input_from_stdin_is_passed_through_unchanged_in_both_modes() {
    for mode in MODES {
        let started = Instant::now();
        let mut child = common::bin()
            .env("UNTWIST_TEST_SUPERVISOR", mode)
            .env("UNTWIST_TEST_STALL", "0")
            .args(["--timeout", "1"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(SMALL.as_bytes())
                .unwrap();
        }
        let out = child.wait_with_output().unwrap();
        let took = started.elapsed();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(took < HARD_STOP_BOUND, "{mode}: {took:?} {stderr}");
        assert_eq!(out.status.code(), Some(3), "{mode}: {stderr}");
        assert!(
            stderr.contains("ran past the time budget"),
            "{mode}: {stderr}"
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), SMALL.trim());
    }
}
