use std::ffi::OsString;
use std::io::{ErrorKind, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::harness::{
    argv, codes, refused, untwist, untwist_stdin, workspace_file, Rows, Run, Scratch,
};
use super::supervisor::{died_warning, json_run, without_first_line, FALLBACK, HIGH, OFF};

const ABORT: &str = "UNTWIST_TEST_WORKER_ABORT";
const CUT_INPUTS: usize = 12;
const CUT_INDEX: usize = 10;
const OOM_SCORE_ADJ_MAX: &str = "1000";
const WORKER_MARKER: &[u8] = b"UNTWIST_INTERNAL_WORKER=1";
const OPEN_NONBLOCKING: i32 = 0o4000;
const PROC_WAIT: Duration = Duration::from_secs(20);
const PROC_POLL: Duration = Duration::from_millis(10);

fn small_inputs(scratch: &Scratch) -> (Vec<PathBuf>, Vec<String>) {
    (0..CUT_INPUTS)
        .map(|index| {
            let source = format!("var value{index} = {index} + 1;\n");
            (scratch.file(&format!("cut{index}.js"), &source), source)
        })
        .unzip()
}

fn others_equal(left: &[String], right: &[String], skip: usize) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .enumerate()
            .all(|(index, (left, right))| index == skip || left == right)
}

pub fn a_death_inside_a_record_refuses_its_input(rows: &mut Rows, scratch: &Scratch) {
    let (inputs, sources) = small_inputs(scratch);
    let cut = format!("mid-record:{CUT_INDEX}");
    for jobs in ["4", "1"] {
        let off = json_run(&["-j", jobs], &inputs, &[OFF]);
        let run = json_run(&["-j", jobs], &inputs, &[(ABORT, &cut)]);
        let value = run.json();
        let got = codes(&value);
        let holds = run.code() == Some(3)
            && !run.died()
            && refused(&value) == Some(vec![CUT_INDEX as u64])
            && got.get(CUT_INDEX).map(|code| code.trim()) == Some(sources[CUT_INDEX].trim())
            && others_equal(&got, &codes(&off.json()), CUT_INDEX)
            && run.stderr().contains(&died_warning(CUT_INDEX + 1));
        rows.check(
            &format!("a death inside F {CUT_INDEX}'s header under -j {jobs}"),
            holds,
            || run.summary(),
        );
    }
}

pub fn a_death_inside_an_output_frame(rows: &mut Rows) {
    let high = workspace_file(HIGH);
    let off = untwist(&argv(&[], &[&high]), &[OFF]);
    let run = untwist(&argv(&[], &[&high]), &[(ABORT, "mid-output")]);
    let stdout = run.stdout();
    let holds = run.code() == Some(1)
        && !run.died()
        && run
            .stderr()
            .ends_with("error: the worker process died while writing its output (signal 6)\n")
        && !stdout.is_empty()
        && stdout.len() < off.stdout().len()
        && off.stdout().starts_with(stdout);
    rows.check("a death inside an O frame", holds, || run.summary());
}

pub fn a_stdin_writer_that_cannot_start_falls_back(rows: &mut Rows) {
    let high = workspace_file(HIGH);
    let bytes = std::fs::read(&high).unwrap_or_default();
    let supervised = untwist_stdin(&[], &[], &bytes);
    let run = untwist_stdin(&[], &[("UNTWIST_TEST_SUPERVISOR", "fail-writer")], &bytes);
    let stderr = run.stderr();
    let holds = run.code() == supervised.code()
        && run.outcome.stdout == supervised.outcome.stdout
        && stderr.starts_with(FALLBACK)
        && without_first_line(&stderr) == Some(supervised.stderr().as_str());
    rows.check("a stdin writer that cannot start", holds, || run.summary());
}

fn masked_timings(stderr: &str) -> String {
    stderr
        .lines()
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let masked: Vec<&str> = words
                .iter()
                .enumerate()
                .map(|(at, word)| {
                    let timed = words.get(at + 1) == Some(&"ms") && word.parse::<f64>().is_ok();
                    if timed {
                        "#"
                    } else {
                        word
                    }
                })
                .collect();
            masked.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn same_text_reports(supervised: &Run, off: &Run) -> bool {
    supervised.code() == off.code()
        && supervised.outcome.stdout == off.outcome.stdout
        && masked_timings(&supervised.stderr()) == masked_timings(&off.stderr())
        && supervised.stderr().contains(" ms")
}

fn proc_text(pid: &str, name: &str) -> Vec<u8> {
    std::fs::read(Path::new("/proc").join(pid).join(name)).unwrap_or_default()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn scores_of(fifo: &Path) -> (Option<String>, Option<String>) {
    let name = fifo.as_os_str().as_encoded_bytes();
    let (mut worker, mut supervisor) = (None, None);
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return (None, None);
    };
    for entry in entries.filter_map(Result::ok) {
        let pid = entry.file_name().to_string_lossy().into_owned();
        let named = contains(&proc_text(&pid, "cmdline"), name);
        if !named || proc_text(&pid, "comm") != b"untwist\n" {
            continue;
        }
        let score = String::from_utf8_lossy(&proc_text(&pid, "oom_score_adj"))
            .trim()
            .to_string();
        if contains(&proc_text(&pid, "environ"), WORKER_MARKER) {
            worker = Some(score);
        } else {
            supervisor = Some(score);
        }
    }
    (worker, supervisor)
}

fn feed(fifo: &Path, bytes: &[u8]) -> bool {
    let started = Instant::now();
    while started.elapsed() < PROC_WAIT {
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(OPEN_NONBLOCKING)
            .open(fifo);
        match opened {
            Ok(mut pipe) => return pipe.write_all(bytes).is_ok(),
            Err(error) if error.kind() == ErrorKind::NotFound => return false,
            Err(_) => std::thread::sleep(PROC_POLL),
        }
    }
    false
}

fn scores_while_waiting(fifo: &Path) -> (Option<String>, Option<String>) {
    let started = Instant::now();
    let mut seen = (None, None);
    while started.elapsed() < PROC_WAIT {
        seen = scores_of(fifo);
        if seen.0.as_deref() == Some(OOM_SCORE_ADJ_MAX) && seen.1.is_some() {
            break;
        }
        std::thread::sleep(PROC_POLL);
    }
    seen
}

pub fn the_worker_is_the_oom_killers_first_choice(rows: &mut Rows, scratch: &Scratch) {
    let fifo = scratch.path("oom.fifo");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .is_ok_and(|status| status.success());
    let args: Vec<OsString> = vec![fifo.as_os_str().to_owned()];
    let running = std::thread::spawn(move || untwist(&args, &[]));
    let (worker, supervisor) = scores_while_waiting(&fifo);
    let fed = feed(&fifo, b"var a = 1;\n");
    let run = running.join().ok();
    let holds = made
        && fed
        && worker.as_deref() == Some(OOM_SCORE_ADJ_MAX)
        && supervisor
            .as_ref()
            .is_some_and(|score| score != OOM_SCORE_ADJ_MAX)
        && run.as_ref().is_some_and(|run| run.code() == Some(0));
    rows.check("the worker's oom_score_adj", holds, || {
        format!(
            "fifo {made} fed {fed} worker {worker:?} supervisor {supervisor:?} {}",
            run.map(|run| run.summary()).unwrap_or_default()
        )
    });
}
