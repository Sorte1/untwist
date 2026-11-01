#![cfg(target_os = "linux")]

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const SYSTEMD_RUN: &str = "/usr/bin/systemd-run";
const GNU_TIME: &str = "/usr/bin/time";
const TIMEOUT_EXIT: i32 = 124;
const SIGKILL: i32 = 9;

#[derive(Clone, Copy, Debug)]
pub struct Cap {
    pub memory_max: &'static str,
    pub timeout_s: u32,
}

impl Cap {
    pub const DEFAULT: Cap = Cap {
        memory_max: "4G",
        timeout_s: 120,
    };
}

#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub peak_rss_kb: u64,
    pub killed_for_memory: bool,
    pub timed_out: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run(
    program: &OsStr,
    args: &[OsString],
    env: &[(OsString, OsString)],
    stdin: Option<&[u8]>,
    cap: Cap,
) -> Outcome {
    let report = report_path();
    let mut command = capped_command(program, args, cap, &report);
    command.envs(env.iter().map(|(key, value)| (key, value)));
    command.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("cannot launch the capped child: {error}"));
    let writer = match (stdin, child.stdin.take()) {
        (Some(bytes), Some(mut pipe)) => {
            let bytes = bytes.to_vec();
            Some(std::thread::spawn(move || {
                let _ = pipe.write_all(&bytes);
            }))
        }
        _ => None,
    };
    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("cannot wait for the capped child: {error}"));
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    let outcome = outcome_from(&output.status, &report, output.stdout, output.stderr);
    let _ = std::fs::remove_file(&report);
    outcome
}

fn uncapped_allowed() -> bool {
    std::env::var_os("UNTWIST_ALLOW_UNCAPPED").is_some_and(|value| value == "1")
}

fn capped_command(program: &OsStr, args: &[OsString], cap: Cap, report: &Path) -> Command {
    let timeout = cap.timeout_s.to_string();
    let tools_present = Path::new(SYSTEMD_RUN).exists() && Path::new(GNU_TIME).exists();
    if !tools_present {
        assert!(
            uncapped_allowed(),
            "{SYSTEMD_RUN} and {GNU_TIME} are required to cap deep test children \
             in memory; set UNTWIST_ALLOW_UNCAPPED=1 to run them under timeout alone"
        );
        let mut command = Command::new("timeout");
        command.arg(&timeout).arg(program).args(args);
        return command;
    }
    let mut command = Command::new(SYSTEMD_RUN);
    command
        .args(["--user", "--scope", "-q", "-p"])
        .arg(format!("MemoryMax={}", cap.memory_max))
        .args([
            "-p",
            "MemorySwapMax=0",
            "-p",
            "OOMPolicy=continue",
            "timeout",
        ])
        .arg(&timeout)
        .args([GNU_TIME, "-f", "%M %x", "-o"])
        .arg(report)
        .arg(program)
        .args(args);
    command
}

fn report_path() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "untwist-capped-{}-{serial}.time",
        std::process::id()
    ))
}

fn outcome_from(
    status: &std::process::ExitStatus,
    report: &Path,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
) -> Outcome {
    let timed_out = status.code() == Some(TIMEOUT_EXIT);
    let mut outcome = Outcome {
        exit_code: status.code(),
        signal: status.signal(),
        timed_out,
        stdout,
        stderr,
        ..Outcome::default()
    };
    if let Ok(text) = std::fs::read_to_string(report) {
        read_time_report(&text, &mut outcome);
    }
    outcome.killed_for_memory = outcome.signal == Some(SIGKILL) && !outcome.timed_out;
    outcome
}

fn read_time_report(text: &str, outcome: &mut Outcome) {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Command terminated by signal ") {
            outcome.signal = rest.trim().parse().ok();
            outcome.exit_code = None;
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(peak), Some(code), None) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        if let (Ok(peak), Ok(code)) = (peak.parse::<u64>(), code.parse::<i32>()) {
            outcome.peak_rss_kb = peak;
            if outcome.signal.is_none() {
                outcome.exit_code = Some(code);
            }
        }
    }
}
